use crate::{
    observable::Observable,
    observer::{Observer, Terminal, boxed_observer::BoxedObserver},
    subscription::Subscription,
};
use educe::Educe;

#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct DoOnTerminal<OE, F> {
    source: OE,
    callback: F,
}

impl<OE, F> DoOnTerminal<OE, F> {
    pub fn new<'sub, 'or, T, E>(source: OE, callback: F) -> Self
    where
        F: FnOnce(&Terminal<E>),
        OE: Observable<'sub, T, E, DoOnTerminalObserver<'or, T, E, F>>,
    {
        Self { source, callback }
    }
}

impl<'sub, 'or, T, E, OR, OE, F> Observable<'sub, T, E, OR> for DoOnTerminal<OE, F>
where
    OR: Observer<T, E> + Send + 'or,
    OE: Observable<'sub, T, E, DoOnTerminalObserver<'or, T, E, F>>,
    F: FnOnce(&Terminal<E>),
{
    fn subscribe(self, observer: OR) -> Subscription<'sub> {
        let observer = DoOnTerminalObserver {
            observer: BoxedObserver::new(observer),
            callback: self.callback,
        };
        self.source.subscribe(observer)
    }
}

pub struct DoOnTerminalObserver<'or, T, E, F> {
    observer: BoxedObserver<'or, T, E>, // TODO: Find a better way to avoid using BoxedObserver here.
    callback: F,
}

impl<T, E, F> Observer<T, E> for DoOnTerminalObserver<'_, T, E, F>
where
    F: FnOnce(&Terminal<E>),
{
    fn on_next(&mut self, value: T) {
        self.observer.on_next(value);
    }

    fn on_terminal(self, terminal: Terminal<E>) {
        (self.callback)(&terminal);
        self.observer.on_terminal(terminal);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        observable::observable_ext::ObservableExt,
        operators::creating::create::Create,
        subject::publish_subject::PublishSubject,
        utils::tests_utils::{checking_observer::CheckingObserver, test_struct::TestStruct},
    };
    use std::{
        convert::Infallible,
        sync::{Arc, Mutex},
    };

    #[test]
    fn test_completed() {
        let mut subject = PublishSubject::default();
        let checker_1 = CheckingObserver::new();
        let checker_2: CheckingObserver<(), _> = CheckingObserver::new();

        // Custom operations
        let observable = subject.clone();
        let checker_2_cloned = checker_2.clone();
        let observable = observable.do_on_terminal(move |terminal| {
            checker_2_cloned.on_terminal(terminal.clone());
        });

        let subscription = observable.subscribe(checker_1.clone());
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
        assert!(checker_1.is_completed());
        assert!(checker_2.is_values_matched(&[]));
        assert!(checker_2.is_completed());

        _ = subscription; // keep the subscription alive
    }

    #[test]
    fn test_error() {
        let mut subject = PublishSubject::default();
        let checker_1 = CheckingObserver::new();
        let checker_2: CheckingObserver<(), _> = CheckingObserver::new();

        // Custom operations
        let observable = subject.clone();
        let checker_2_cloned = checker_2.clone();
        let observable = observable.do_on_terminal(move |terminal| {
            checker_2_cloned.on_terminal(terminal.clone());
        });

        let subscription = observable.subscribe(checker_1.clone());
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
        assert!(checker_1.is_error("error"));
        assert!(checker_2.is_values_matched(&[]));
        assert!(checker_2.is_error("error"));

        _ = subscription; // keep the subscription alive
    }

    #[test]
    fn test_unsubscribe() {
        let mut subject = PublishSubject::default();
        let checker_1 = CheckingObserver::new();
        let checker_2 = CheckingObserver::new();
        let checker_3: CheckingObserver<(), _> = CheckingObserver::new();

        // Custom operations
        let observable = subject.clone();
        let checker_3_cloned = checker_3.clone();
        let observable = observable.do_on_terminal(move |terminal| {
            checker_3_cloned.on_terminal(terminal.clone());
        });
        let observable_1 = observable;
        let observable_2 = observable_1.clone();

        let subscription_1 = observable_1.subscribe(checker_1.clone());
        let subscription_2 = observable_2.subscribe(checker_2.clone());
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
        assert!(checker_2.is_error("error"));
        assert!(checker_3.is_values_matched(&[]));
        assert!(checker_3.is_error("error"));

        _ = subscription_2; // keep the subscription alive
    }

    #[test]
    fn test_ref() {
        let value = 111;
        let error = 222;

        let checker_1 = CheckingObserver::new();
        let checker_2: CheckingObserver<(), _> = CheckingObserver::new();
        let checker_2_cloned = checker_2.clone();

        let mut subject = PublishSubject::default();

        // Custom operations
        let observable = subject.clone();
        let observable = observable.do_on_terminal(|terminal| {
            checker_2_cloned.on_terminal(terminal.clone());
        });

        let subscription = observable.subscribe(checker_1.clone());
        assert!(checker_1.is_values_matched(&[]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[]));
        assert!(checker_2.is_unterminated());

        subject.on_next(&value);
        assert!(checker_1.is_values_matched(&[&value]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[]));
        assert!(checker_2.is_unterminated());

        subject.on_terminal(Terminal::Error(&error));
        assert!(checker_1.is_values_matched(&[&value]));
        assert!(checker_1.is_error(&error));
        assert!(checker_2.is_values_matched(&[]));
        assert!(checker_2.is_error(&error));

        _ = subscription; // keep the subscription alive
    }

    #[test]
    fn test_mut_ref() {
        let mut value = 111;
        let mut error = 222;

        let observable = Create::new(|mut observer| {
            observer.on_next(&mut value);
            observer.on_terminal(Terminal::Error(&mut error));
            Subscription::new_none_disposal()
        });
        let checker: CheckingObserver<(), _> = CheckingObserver::new();
        let checker_cloned = checker.clone();

        // Custom operations
        let observable = observable.do_on_terminal(|terminal| match terminal {
            Terminal::Completed => panic!(),
            Terminal::Error(error) => {
                checker_cloned.on_terminal(Terminal::Error(**error));
            }
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
        assert_eq!(error, 444);

        _ = subscription; // keep the subscription alive
    }

    #[tokio::test]
    async fn test_async() {
        let subject = PublishSubject::default();
        let checker_1 = CheckingObserver::new();
        let checker_2: CheckingObserver<(), _> = CheckingObserver::new();

        // Custom operations
        let observable = subject.clone();
        let checker_2_cloned = checker_2.clone();
        let observable = observable.do_on_terminal(|terminal| {
            checker_2_cloned.on_terminal(terminal.clone());
        });

        let checker_cloned = checker_1.clone();
        let handle = tokio::spawn(async move { observable.subscribe(checker_cloned) });
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
        let checker_1 = CheckingObserver::new();
        let checker_2 = CheckingObserver::new();
        let terminals = Arc::new(Mutex::new(Vec::new()));

        // Custom operations
        let observable = subject.clone();
        let terminals_cloned = terminals.clone();
        let observable = observable.do_on_terminal(move |terminal| {
            terminals_cloned.lock().unwrap().push(terminal.clone());
        });
        let observable_1 = observable;
        let observable_2 = observable_1.clone();

        let subscription_1 = observable_1.subscribe(checker_1.clone());

        let (on_next, on_terminal) = checker_2.clone().into_callbacks();
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
        assert!(checker_1.is_error("error"));
        assert!(checker_2.is_values_matched(&[111]));
        assert!(checker_2.is_error("error"));
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
        let checker = CheckingObserver::new();
        let terminals = Arc::new(Mutex::new(Vec::new()));

        // Custom operations
        let observable = subject.clone();
        let terminals_cloned_1 = terminals.clone();
        let terminals_cloned_2 = terminals.clone();
        let observable = observable
            .do_on_terminal(move |terminal| {
                terminals_cloned_1.lock().unwrap().push(terminal.clone());
            })
            .do_on_terminal(move |terminal| {
                terminals_cloned_2.lock().unwrap().push(terminal.clone());
            });

        let subscription = observable.subscribe(checker.clone());
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());
        assert!(terminals.lock().unwrap().is_empty());

        subject.on_next(111);
        assert!(checker.is_values_matched(&[111]));
        assert!(checker.is_unterminated());
        assert!(terminals.lock().unwrap().is_empty());

        subject.on_terminal(Terminal::Error("error"));
        assert!(checker.is_values_matched(&[111]));
        assert!(checker.is_error("error"));
        assert_eq!(
            terminals.lock().unwrap().as_ref(),
            vec![Terminal::Error("error"), Terminal::Error("error")]
        );

        _ = subscription; // keep the subscription alive
    }

    #[test]
    fn test_without_convenient_api() {
        let mut subject = PublishSubject::default();
        let checker_1 = CheckingObserver::new();
        let checker_2: CheckingObserver<(), _> = CheckingObserver::new();

        // Custom operations
        let observable = subject.clone();
        let checker_2_cloned = checker_2.clone();
        let observable = DoOnTerminal::new(observable, move |terminal| {
            checker_2_cloned.on_terminal(terminal.clone());
        });

        let subscription = observable.subscribe(checker_1.clone());
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
        assert!(checker_1.is_error("error"));
        assert!(checker_2.is_values_matched(&[]));
        assert!(checker_2.is_error("error"));

        _ = subscription; // keep the subscription alive
    }

    #[test]
    fn test_lifetime_a() {
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

            let observable = observable.do_on_terminal(|_| {});

            let checker = CheckingObserver::new();
            subscription = observable.subscribe(checker);
        }

        _ = subscription; // keep the subscription alive
    }

    #[test]
    fn test_lifetime_b() {
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
            let observable = observable.do_on_terminal(|_| {});

            let mut checker: CheckingObserver<_, Infallible> = CheckingObserver::new();
            checker.on_next(Some(&life_marker_2));
            let subscription = observable.subscribe(checker);

            _ = subscription; // keep the subscription alive
        }
    }

    #[test]
    fn test_fn() {
        let s = TestStruct;

        let subject: PublishSubject<'_, i32, &str> = PublishSubject::default();

        // Custom operations
        let observable = subject.clone();
        let observable = observable.do_on_terminal(|_| {
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
        let observable = observable.do_on_terminal(|_| {});
        let _ = observable.clone(); // make sure it's Clone when T and E is not Clone.
    }

    #[test]
    fn test_type_inference_with_subscribe() {
        // Custom operations
        let subject: PublishSubject<'_, i32, String> = PublishSubject::default();
        let observable = subject.do_on_terminal(|_| {});

        let observable = observable.buffer_with_count(1);
        let checker = CheckingObserver::new();
        observable.subscribe(checker);
    }

    #[test]
    fn test_type_inference_without_subscribe() {
        // Custom operations
        let subject: PublishSubject<'_, i32, String> = PublishSubject::default();
        let observable = subject.do_on_terminal(|_| {});

        let _ = observable.buffer_with_count(1);
    }
}
