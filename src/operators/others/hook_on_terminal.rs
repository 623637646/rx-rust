use crate::{
    observable::Observable,
    observer::{Observer, Terminal},
    subscription::Subscription,
};
use educe::Educe;

#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct HookOnTerminal<OE, F> {
    source: OE,
    callback: F,
}

impl<OE, F> HookOnTerminal<OE, F> {
    pub fn new<'or, 'sub, T, E>(source: OE, callback: F) -> Self
    where
        OE: Observable<'or, 'sub, T, E>,
        F: for<'a> FnOnce(Terminal<E>, Box<dyn FnOnce(Terminal<E>) + 'a>),
    {
        Self { source, callback }
    }
}

impl<'or, 'sub, T, E, OE, F> Observable<'or, 'sub, T, E> for HookOnTerminal<OE, F>
where
    OE: Observable<'or, 'sub, T, E>,
    F: for<'a> FnOnce(Terminal<E>, Box<dyn FnOnce(Terminal<E>) + 'a>) + Send + 'or,
{
    fn subscribe(self, observer: impl Observer<T, E> + Send + 'or) -> Subscription<'sub> {
        let observer = HookOnTerminalObserver {
            observer,
            callback: self.callback,
        };
        self.source.subscribe(observer)
    }
}

pub struct HookOnTerminalObserver<OR, F> {
    observer: OR,
    callback: F,
}

impl<T, E, OR, F> Observer<T, E> for HookOnTerminalObserver<OR, F>
where
    OR: Observer<T, E>,
    F: for<'a> FnOnce(Terminal<E>, Box<dyn FnOnce(Terminal<E>) + 'a>),
{
    fn on_next(&mut self, value: T) {
        self.observer.on_next(value);
    }

    fn on_terminal(self, terminal: Terminal<E>) {
        (self.callback)(
            terminal,
            Box::new(|terminal| {
                self.observer.on_terminal(terminal);
            }),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        observable::observable_ext::ObservableExt,
        operators::creating::create::Create,
        subject::publish_subject::PublishSubject,
        utils::tests_utils::{checker::Checker, test_struct::TestStruct},
    };
    use std::{
        convert::Infallible,
        sync::{Arc, Mutex},
    };

    #[test]
    fn test_completed() {
        let mut subject = PublishSubject::default();
        let (checker_1, observer_1) = Checker::new();
        let (checker_2, observer_2) = Checker::<(), _>::new();

        // Custom operations
        let observable = subject.clone();
        let checker_2_cloned = checker_2.clone();
        let observable = observable.hook_on_terminal(move |terminal, original| {
            checker_2_cloned.on_terminal(terminal);
            original(Terminal::Error("error"));
        });

        let subscription = observable.subscribe(observer_1);
        assert!(checker_1.is_values_matched(&[]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[]));
        assert!(checker_2.is_unterminated());

        subject.on_next(111);
        assert!(checker_1.is_values_matched(&[111]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[]));
        assert!(checker_2.is_unterminated());

        subject.on_terminal(Terminal::<&str>::Completed);
        assert!(checker_1.is_values_matched(&[111]));
        assert!(checker_1.is_error("error"));
        assert!(checker_2.is_values_matched(&[]));
        assert!(checker_2.is_completed());

        _ = subscription; // keep the subscription alive
    }

    #[test]
    fn test_completed_no_call_original() {
        let mut subject = PublishSubject::default();
        let (checker_1, observer_1) = Checker::new();
        let (checker_2, observer_2) = Checker::<Infallible, _>::new();

        // Custom operations
        let observable = subject.clone();
        let checker_2_cloned = checker_2.clone();
        let observable = observable.hook_on_terminal(move |terminal, _| {
            checker_2_cloned.on_terminal(terminal);
        });

        let subscription = observable.subscribe(observer_1);
        assert!(checker_1.is_values_matched(&[]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[]));
        assert!(checker_2.is_unterminated());

        subject.on_next(111);
        assert!(checker_1.is_values_matched(&[111]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[]));
        assert!(checker_2.is_unterminated());

        subject.on_terminal(Terminal::<&str>::Completed);
        assert!(checker_1.is_values_matched(&[111]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[]));
        assert!(checker_2.is_completed());

        _ = subscription; // keep the subscription alive
    }

    #[test]
    fn test_error() {
        let mut subject = PublishSubject::default();
        let (checker_1, observer_1) = Checker::new();
        let (checker_2, observer_2) = Checker::<Infallible, _>::new();

        // Custom operations
        let observable = subject.clone();
        let checker_2_cloned = checker_2.clone();
        let observable = observable.hook_on_terminal(move |terminal, original| {
            checker_2_cloned.on_terminal(terminal);
            original(Terminal::Completed);
        });

        let subscription = observable.subscribe(observer_1);
        assert!(checker_1.is_values_matched(&[]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[]));
        assert!(checker_2.is_unterminated());

        subject.on_next(111);
        assert!(checker_1.is_values_matched(&[111]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[]));
        assert!(checker_2.is_unterminated());

        subject.on_terminal(Terminal::Error("error"));
        assert!(checker_1.is_values_matched(&[111]));
        assert!(checker_1.is_completed());
        assert!(checker_2.is_values_matched(&[]));
        assert!(checker_2.is_error("error"));

        _ = subscription; // keep the subscription alive
    }

    #[test]
    fn test_unsubscribe() {
        let mut subject = PublishSubject::default();
        let (checker_1, observer_1) = Checker::new();
        let (checker_2, observer_2) = Checker::new();
        let (checker_3, observer_3) = Checker::<Infallible, _>::new();

        // Custom operations
        let observable = subject.clone();
        let checker_3_cloned = checker_3.clone();
        let observable = observable.hook_on_terminal(move |terminal, original| {
            match terminal {
                Terminal::Completed => panic!(),
                Terminal::Error(error) => {
                    assert_eq!(error, "error");
                    original(Terminal::Error("hooked"));
                }
            }
            checker_3_cloned.on_terminal(terminal);
        });
        let observable_1 = observable;
        let observable_2 = observable_1.clone();

        let subscription_1 = observable_1.subscribe(observer_1);
        let subscription_2 = observable_2.subscribe(observer_2);
        assert!(checker_1.is_values_matched(&[]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[]));
        assert!(checker_2.is_unterminated());
        assert!(checker_3.is_values_matched(&[]));
        assert!(checker_3.is_unterminated());

        subject.on_next(111);
        assert!(checker_1.is_values_matched(&[111]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[111]));
        assert!(checker_2.is_unterminated());
        assert!(checker_3.is_values_matched(&[]));
        assert!(checker_3.is_unterminated());

        subscription_1.unsubscribe();
        assert!(checker_1.is_values_matched(&[111]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[111]));
        assert!(checker_2.is_unterminated());
        assert!(checker_3.is_values_matched(&[]));
        assert!(checker_3.is_unterminated());

        subject.on_next(222);
        assert!(checker_1.is_values_matched(&[111]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[111, 222]));
        assert!(checker_2.is_unterminated());
        assert!(checker_3.is_values_matched(&[]));
        assert!(checker_3.is_unterminated());

        subject.on_terminal(Terminal::Error("error"));
        assert!(checker_1.is_values_matched(&[111]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[111, 222]));
        assert!(checker_2.is_error("hooked"));
        assert!(checker_3.is_values_matched(&[]));
        assert!(checker_3.is_error("error"));

        _ = subscription_2; // keep the subscription alive
    }

    #[test]
    fn test_ref() {
        let value = 111;
        let error_1 = 222;
        let error_2 = 333;

        let (checker_1, observer_1) = Checker::new();
        let (checker_2, observer_2) = Checker::<Infallible, _>::new();
        let checker_2_cloned = checker_2.clone();

        let mut subject = PublishSubject::default();

        // Custom operations
        let observable = subject.clone();
        let observable = observable.hook_on_terminal(|terminal, original| {
            checker_2_cloned.on_terminal(terminal);
            original(Terminal::Error(&error_2));
        });

        let subscription = observable.subscribe(observer_1);
        assert!(checker_1.is_values_matched(&[]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[]));
        assert!(checker_2.is_unterminated());

        subject.on_next(&value);
        assert!(checker_1.is_values_matched(&[&value]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[]));
        assert!(checker_2.is_unterminated());

        subject.on_terminal(Terminal::Error(&error_1));
        assert!(checker_1.is_values_matched(&[&value]));
        assert!(checker_1.is_error(&error_2));
        assert!(checker_2.is_values_matched(&[]));
        assert!(checker_2.is_error(&error_1));

        _ = subscription; // keep the subscription alive
    }

    #[test]
    fn test_mut_ref() {
        let mut value = 111;
        let mut error_1 = 222;
        let mut error_2 = 333;

        let observable = Create::new(|mut observer| {
            observer.on_next(&mut value);
            observer.on_terminal(Terminal::Error(&mut error_1));
            Subscription::new_none_disposal()
        });
        let (checker, observer) = Checker::<Infallible, _>::new();
        let checker_cloned = checker.clone();

        // Custom operations
        let observable = observable.hook_on_terminal(|mut terminal, original| {
            match &mut terminal {
                Terminal::Completed => panic!(),
                Terminal::Error(error) => {
                    checker_cloned.on_terminal(Terminal::Error(**error));
                    **error *= 2;
                }
            }
            original(Terminal::Error(&mut error_2));
        });

        let subscription = observable.subscribe_with_callback(
            |value| {
                *value *= 2;
            },
            |terminal| match terminal {
                Terminal::Completed => panic!(),
                Terminal::Error(error) => {
                    *error *= 2;
                }
            },
        );

        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_error(222));
        assert_eq!(value, 222);
        assert_eq!(error_1, 444);
        assert_eq!(error_2, 666);

        _ = subscription; // keep the subscription alive
    }

    #[tokio::test]
    async fn test_async() {
        let subject = PublishSubject::default();
        let (checker_1, observer_1) = Checker::new();
        let (checker_2, observer_2) = Checker::<Infallible, _>::new();

        // Custom operations
        let observable = subject.clone();
        let checker_2_cloned = checker_2.clone();
        let observable = observable.hook_on_terminal(move |terminal, original| {
            checker_2_cloned.on_terminal(terminal);
            original(Terminal::Completed);
            panic!()
        });

        let checker_cloned = checker_1.clone();
        let handle = tokio::spawn(async move { observable.subscribe(observer) });
        let subscription = handle.await.unwrap();
        assert!(checker_1.is_values_matched(&[]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[]));
        assert!(checker_2.is_unterminated());

        let mut subject_cloned = subject.clone();
        let handle = tokio::spawn(async move {
            subject_cloned.on_next(111);
        });
        handle.await.unwrap();
        assert!(checker_1.is_values_matched(&[111]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[]));
        assert!(checker_2.is_unterminated());

        let handle = tokio::spawn(async { subscription.unsubscribe() });
        handle.await.unwrap();
        assert!(checker_1.is_values_matched(&[111]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[]));
        assert!(checker_2.is_unterminated());

        let subject_cloned = subject.clone();
        let handle = tokio::spawn(async move {
            subject_cloned.on_terminal(Terminal::Error("error"));
        });
        handle.await.unwrap();
        assert!(checker_1.is_values_matched(&[111]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[]));
        assert!(checker_2.is_unterminated());
    }

    #[test]
    fn test_subscribe_by_different_observer() {
        let mut subject = PublishSubject::default();
        let (checker_1, observer_1) = Checker::new();
        let (checker_2, observer_2) = Checker::new();
        let terminals = Arc::new(Mutex::new(Vec::new()));

        // Custom operations
        let observable = subject.clone();
        let terminals_cloned = terminals.clone();
        let observable = observable.hook_on_terminal(move |terminal, original| {
            terminals_cloned.lock().unwrap().push(terminal);
            original(Terminal::Completed);
        });
        let observable_1 = observable;
        let observable_2 = observable_1.clone();

        let subscription_1 = observable_1.subscribe(observer_1);

        let (on_next, on_terminal) = observer_2.into_callbacks();
        let subscription_2 = observable_2.subscribe_with_callback(on_next, on_terminal);
        assert!(checker_1.is_values_matched(&[]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[]));
        assert!(checker_2.is_unterminated());
        assert!(terminals.lock().unwrap().is_empty());

        subject.on_next(111);
        assert!(checker_1.is_values_matched(&[111]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[111]));
        assert!(checker_2.is_unterminated());
        assert!(terminals.lock().unwrap().is_empty());

        subject.on_terminal(Terminal::Error("error"));
        assert!(checker_1.is_values_matched(&[111]));
        assert!(checker_1.is_completed());
        assert!(checker_2.is_values_matched(&[111]));
        assert!(checker_2.is_completed());
        assert_eq!(
            terminals.lock().unwrap().as_ref(),
            vec![Terminal::Error("error"), Terminal::Error("error")]
        );

        _ = subscription_1; // keep the subscription alive
        _ = subscription_2; // keep the subscription alive
    }

    #[test]
    fn test_multiple_operation() {
        let mut subject = PublishSubject::default();
        let (checker_1, observer_1) = Checker::new();
        let (checker_2, observer_2) = Checker::<Infallible, _>::new();
        let (checker_3, observer_3) = Checker::<Infallible, _>::new();

        // Custom operations
        let observable = subject.clone();
        let checker_2_cloned = checker_2.clone();
        let checker_3_cloned = checker_3.clone();
        let observable = observable
            .hook_on_terminal(move |terminal, original| {
                checker_2_cloned.on_terminal(terminal);
                original(Terminal::Error("222"));
            })
            .hook_on_terminal(move |terminal, original| {
                checker_3_cloned.on_terminal(terminal);
                original(Terminal::Error("333"));
            });

        let subscription = observable.subscribe(observer_1);
        assert!(checker_1.is_values_matched(&[]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[]));
        assert!(checker_2.is_unterminated());
        assert!(checker_3.is_values_matched(&[]));
        assert!(checker_3.is_unterminated());

        subject.on_next(111);
        assert!(checker_1.is_values_matched(&[111]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[]));
        assert!(checker_2.is_unterminated());
        assert!(checker_3.is_values_matched(&[]));
        assert!(checker_3.is_unterminated());

        subject.on_terminal(Terminal::Error("111"));
        assert!(checker_1.is_values_matched(&[111]));
        assert!(checker_1.is_error("333"));
        assert!(checker_2.is_values_matched(&[]));
        assert!(checker_2.is_error("111"));
        assert!(checker_3.is_values_matched(&[]));
        assert!(checker_3.is_error("222"));

        _ = subscription; // keep the subscription alive
    }

    #[test]
    fn test_without_convenient_api() {
        let mut subject = PublishSubject::default();
        let (checker_1, observer_1) = Checker::new();
        let (checker_2, observer_2) = Checker::<Infallible, _>::new();

        // Custom operations
        let observable = subject.clone();
        let checker_2_cloned = checker_2.clone();
        let observable = HookOnTerminal::new(observable, move |terminal, original| {
            checker_2_cloned.on_terminal(terminal);
            original(Terminal::Completed);
        });

        let subscription = observable.subscribe(observer_1);
        assert!(checker_1.is_values_matched(&[]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[]));
        assert!(checker_2.is_unterminated());

        subject.on_next(111);
        assert!(checker_1.is_values_matched(&[111]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[]));
        assert!(checker_2.is_unterminated());

        subject.on_terminal(Terminal::Error("error"));
        assert!(checker_1.is_values_matched(&[111]));
        assert!(checker_1.is_completed());
        assert!(checker_2.is_values_matched(&[]));
        assert!(checker_2.is_error("error"));

        _ = subscription; // keep the subscription alive
    }

    #[test]
    fn test_lifetime_sub() {
        // OK
        let life_marker = TestStruct;
        let subscription;

        // Error
        // let subscription;
        // let life_marker = TestStruct;

        {
            let observable = Create::new(|mut observer| {
                observer.on_next(1);
                observer.on_terminal(Terminal::<String>::Completed);
                Subscription::new_with_disposal_callback(|| {
                    life_marker.consume_ref();
                })
            });

            let observable = observable.hook_on_terminal(|_, _| {});

            let (checker, observer) = Checker::new();
            subscription = observable.subscribe(observer);
        }

        _ = subscription; // keep the subscription alive
    }

    #[test]
    fn test_lifetime_or() {
        // OK
        let life_marker_2 = TestStruct;
        let mut life_marker_1 = None;

        // Error
        // let mut life_marker_1 = None;
        // let life_marker_2 = TestStruct;

        {
            let observable = Create::new(|observer| {
                life_marker_1 = Some(observer);
                Subscription::new_none_disposal()
            });
            let observable = observable.hook_on_terminal(|_, _| {});

            let (checker, mut observer) = Checker::<_, Infallible>::new();
            observer.on_next(Some(&life_marker_2));
            let subscription = observable.subscribe(observer);

            _ = subscription; // keep the subscription alive
        }
    }

    #[test]
    fn test_fn() {
        let s = TestStruct;

        let subject: PublishSubject<'_, i32, &str> = PublishSubject::default();

        // Custom operations
        let observable = subject.clone();
        let observable = observable.hook_on_terminal(|_, _| {
            s.consume();
        });

        observable.subscribe_with_callback(|_| {}, |_| {});
    }

    #[test]
    fn test_clone() {
        let observable = Create::new(|mut observer| {
            observer.on_next(TestStruct);
            observer.on_terminal(Terminal::Error(TestStruct));
            Subscription::new_none_disposal()
        });
        let observable = observable.hook_on_terminal(|_, _| {});
        let _ = observable.clone(); // make sure it's Clone when T and E is not Clone.
    }

    #[test]
    fn test_type_inference_with_subscribe() {
        // Custom operations
        let subject: PublishSubject<'_, i32, String> = PublishSubject::default();
        let observable = subject.hook_on_terminal(|_, _| {});

        let observable = observable.buffer_with_count(1);
        let (checker, observer) = Checker::new();
        observable.subscribe(observer);
    }

    #[test]
    fn test_type_inference_without_subscribe() {
        // Custom operations
        let subject: PublishSubject<'_, i32, String> = PublishSubject::default();
        let observable = subject.hook_on_terminal(|_, _| {});

        let _ = observable.buffer_with_count(1);
    }
}
