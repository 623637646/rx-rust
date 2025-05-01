use crate::{
    observable::Observable,
    observer::{Observer, Terminal},
    subscription::Subscription,
};
use educe::Educe;

#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct DoOnNext<OE, F> {
    source: OE,
    callback: F,
}

impl<OE, F> DoOnNext<OE, F> {
    pub fn new<'or, 'sub, T, E>(source: OE, callback: F) -> Self
    where
        OE: Observable<'or, 'sub, T, E>,
        F: FnMut(&T),
    {
        Self { source, callback }
    }
}

impl<'or, 'sub, T, E, OE, F> Observable<'or, 'sub, T, E> for DoOnNext<OE, F>
where
    OE: Observable<'or, 'sub, T, E>,
    F: FnMut(&T) + Send + 'or,
{
    fn subscribe(self, observer: impl Observer<T, E> + Send + 'or) -> Subscription<'sub> {
        let observer = DoOnNextObserver {
            observer,
            callback: self.callback,
        };
        self.source.subscribe(observer)
    }
}

pub struct DoOnNextObserver<OR, F> {
    observer: OR,
    callback: F,
}

impl<T, E, OR, F> Observer<T, E> for DoOnNextObserver<OR, F>
where
    OR: Observer<T, E>,
    F: FnMut(&T),
{
    fn on_next(&mut self, value: T) {
        (self.callback)(&value);
        self.observer.on_next(value);
    }

    fn on_terminal(self, terminal: Terminal<E>) {
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
        let (checker_2, mut observer_2) = Checker::<_, String>::new();

        // Custom operations
        let observable = subject.clone();
        let observable = observable.do_on_next(move |value| {
            observer_2.on_next(*value);
        });

        let subscription = observable.subscribe(observer_1);
        assert!(checker_1.is_values_matched(&[]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[]));
        assert!(checker_2.is_unterminated());

        subject.on_next(111);
        assert!(checker_1.is_values_matched(&[111]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[111]));
        assert!(checker_2.is_unterminated());

        subject.on_terminal(Terminal::<&str>::Completed);
        assert!(checker_1.is_values_matched(&[111]));
        assert!(checker_1.is_completed());
        assert!(checker_2.is_values_matched(&[111]));
        assert!(checker_2.is_unterminated());

        _ = subscription; // keep the subscription alive
    }

    #[test]
    fn test_error() {
        let mut subject = PublishSubject::default();
        let (checker_1, observer_1) = Checker::new();
        let (checker_2, mut observer_2) = Checker::<_, String>::new();

        // Custom operations
        let observable = subject.clone();
        let observable = observable.do_on_next(move |value| {
            observer_2.on_next(*value);
        });

        let subscription = observable.subscribe(observer_1);
        assert!(checker_1.is_values_matched(&[]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[]));
        assert!(checker_2.is_unterminated());

        subject.on_next(111);
        assert!(checker_1.is_values_matched(&[111]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[111]));
        assert!(checker_2.is_unterminated());

        subject.on_terminal(Terminal::Error("error"));
        assert!(checker_1.is_values_matched(&[111]));
        assert!(checker_1.is_error("error"));
        assert!(checker_2.is_values_matched(&[111]));
        assert!(checker_2.is_unterminated());

        _ = subscription; // keep the subscription alive
    }

    #[test]
    fn test_unsubscribe() {
        let mut subject = PublishSubject::default();
        let (checker_1, observer_1) = Checker::new();
        let (checker_2, observer_2) = Checker::new();
        let (checker_3, observer_3) = Checker::<_, String>::new();

        // Custom operations
        let observable = subject.clone();
        let observer_3 = Arc::new(Mutex::new(observer_3));
        let observable = observable.do_on_next(move |value| {
            observer_3.lock().unwrap().on_next(*value);
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
        assert!(checker_3.is_values_matched(&[111, 111]));
        assert!(checker_3.is_unterminated());

        subscription_1.unsubscribe();
        assert!(checker_1.is_values_matched(&[111]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[111]));
        assert!(checker_2.is_unterminated());
        assert!(checker_3.is_values_matched(&[111, 111]));
        assert!(checker_3.is_unterminated());

        subject.on_next(222);
        assert!(checker_1.is_values_matched(&[111]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[111, 222]));
        assert!(checker_2.is_unterminated());
        assert!(checker_3.is_values_matched(&[111, 111, 222]));
        assert!(checker_3.is_unterminated());

        subject.on_terminal(Terminal::Error("error"));
        assert!(checker_1.is_values_matched(&[111]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[111, 222]));
        assert!(checker_2.is_error("error"));
        assert!(checker_3.is_values_matched(&[111, 111, 222]));
        assert!(checker_3.is_unterminated());

        _ = subscription_2; // keep the subscription alive
    }

    #[test]
    fn test_ref() {
        let value = 111;
        let error = 222;

        let (checker_1, observer_1) = Checker::new();
        let (checker_2, mut observer_2) = Checker::<_, String>::new();

        let mut subject = PublishSubject::default();

        // Custom operations
        let observable = subject.clone();
        let observable = observable.do_on_next(|value| {
            observer_2.on_next(*value);
        });

        let subscription = observable.subscribe(observer_1);
        assert!(checker_1.is_values_matched(&[]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[]));
        assert!(checker_2.is_unterminated());

        subject.on_next(&value);
        assert!(checker_1.is_values_matched(&[&value]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[&value]));
        assert!(checker_2.is_unterminated());

        subject.on_terminal(Terminal::Error(&error));
        assert!(checker_1.is_values_matched(&[&value]));
        assert!(checker_1.is_error(&error));
        assert!(checker_2.is_values_matched(&[&value]));
        assert!(checker_2.is_unterminated());

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
        let (checker, mut observer) = Checker::<_, String>::new();

        // Custom operations
        let observable = observable.do_on_next(|value| {
            observer.on_next(**value);
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

        assert!(checker.is_values_matched(&[111]));
        assert!(checker.is_unterminated());
        assert_eq!(value, 222);
        assert_eq!(error, 444);

        _ = subscription; // keep the subscription alive
    }

    #[tokio::test]
    async fn test_async() {
        let subject = PublishSubject::default();
        let (checker_1, observer_1) = Checker::new();
        let (checker_2, mut observer_2) = Checker::<_, String>::new();

        // Custom operations
        let observable = subject.clone();
        let observable = observable.do_on_next(move |value| {
            observer_2.on_next(*value);
        });

        let handle = tokio::spawn(async move { observable.subscribe(observer_1) });
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
        assert!(checker_2.is_values_matched(&[111]));
        assert!(checker_2.is_unterminated());

        let handle = tokio::spawn(async { subscription.unsubscribe() });
        handle.await.unwrap();
        assert!(checker_1.is_values_matched(&[111]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[111]));
        assert!(checker_2.is_unterminated());

        let subject_cloned = subject.clone();
        let handle = tokio::spawn(async move {
            subject_cloned.on_terminal(Terminal::Error("error"));
        });
        handle.await.unwrap();
        assert!(checker_1.is_values_matched(&[111]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[111]));
        assert!(checker_2.is_unterminated());
    }

    #[test]
    fn test_subscribe_by_different_observer() {
        let mut subject = PublishSubject::default();
        let (checker_1, observer_1) = Checker::new();
        let (checker_2, observer_2) = Checker::new();
        let (checker_3, observer_3) = Checker::<_, String>::new();

        // Custom operations
        let observable = subject.clone();
        let observer_3 = Arc::new(Mutex::new(observer_3));
        let observable = observable.do_on_next(move |value| {
            observer_3.lock().unwrap().on_next(*value);
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
        assert!(checker_3.is_values_matched(&[]));
        assert!(checker_3.is_unterminated());

        subject.on_next(111);
        assert!(checker_1.is_values_matched(&[111]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[111]));
        assert!(checker_2.is_unterminated());
        assert!(checker_3.is_values_matched(&[111, 111]));
        assert!(checker_3.is_unterminated());

        subject.on_terminal(Terminal::Error("error"));
        assert!(checker_1.is_values_matched(&[111]));
        assert!(checker_1.is_error("error"));
        assert!(checker_2.is_values_matched(&[111]));
        assert!(checker_2.is_error("error"));
        assert!(checker_3.is_values_matched(&[111, 111]));
        assert!(checker_3.is_unterminated());

        _ = subscription_1; // keep the subscription alive
        _ = subscription_2; // keep the subscription alive
    }

    #[test]
    fn test_multiple_operation() {
        let mut subject: PublishSubject<'_, i32, &str> = PublishSubject::default();
        let (checker_1, observer_1) = Checker::new();
        let (checker_2, mut observer_2) = Checker::<_, String>::new();
        let (checker_3, mut observer_3) = Checker::<_, String>::new();

        // Custom operations
        let observable = subject.clone();
        let observable = observable
            .do_on_next(move |value| {
                observer_2.on_next(*value);
            })
            .do_on_next(move |value| {
                observer_3.on_next(*value);
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
        assert!(checker_2.is_values_matched(&[111]));
        assert!(checker_2.is_unterminated());
        assert!(checker_3.is_values_matched(&[111]));
        assert!(checker_3.is_unterminated());

        subject.on_terminal(Terminal::Error("error"));
        assert!(checker_1.is_values_matched(&[111]));
        assert!(checker_1.is_error("error"));
        assert!(checker_2.is_values_matched(&[111]));
        assert!(checker_2.is_unterminated());
        assert!(checker_3.is_values_matched(&[111]));
        assert!(checker_3.is_unterminated());

        _ = subscription; // keep the subscription alive
    }

    #[test]
    fn test_without_convenient_api() {
        let mut subject = PublishSubject::default();
        let (checker_1, observer_1) = Checker::new();
        let (checker_2, mut observer_2) = Checker::<_, String>::new();

        // Custom operations
        let observable = subject.clone();
        let observable = DoOnNext::new(observable, move |value| {
            observer_2.on_next(*value);
        });

        let subscription = observable.subscribe(observer_1);
        assert!(checker_1.is_values_matched(&[]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[]));
        assert!(checker_2.is_unterminated());

        subject.on_next(111);
        assert!(checker_1.is_values_matched(&[111]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[111]));
        assert!(checker_2.is_unterminated());

        subject.on_terminal(Terminal::Error("error"));
        assert!(checker_1.is_values_matched(&[111]));
        assert!(checker_1.is_error("error"));
        assert!(checker_2.is_values_matched(&[111]));
        assert!(checker_2.is_unterminated());

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

            let observable = observable.do_on_next(|_| {});

            let (_, observer) = Checker::new();
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
            let observable = observable.do_on_next(|_| {});

            let (_, mut observer) = Checker::<_, Infallible>::new();
            observer.on_next(Some(&life_marker_2));
            let subscription = observable.subscribe(observer);

            _ = subscription; // keep the subscription alive
        }
    }

    #[test]
    fn test_fn() {
        let mut s = TestStruct;

        let subject: PublishSubject<'_, i32, &str> = PublishSubject::default();

        // Custom operations
        let observable = subject.clone();
        let observable = observable.do_on_next(|_| {
            s.consume_mut();
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
        let observable = observable.do_on_next(|_| {});
        let _ = observable.clone(); // make sure it's Clone when T and E is not Clone.
    }

    #[test]
    fn test_type_inference_with_subscribe() {
        // Custom operations
        let subject: PublishSubject<'_, i32, String> = PublishSubject::default();
        let observable = subject.do_on_next(|_| {});

        let observable = observable.buffer_with_count(1);
        let (_, observer) = Checker::new();
        observable.subscribe(observer);
    }

    #[test]
    fn test_type_inference_without_subscribe() {
        // Custom operations
        let subject: PublishSubject<'_, i32, String> = PublishSubject::default();
        let observable = subject.do_on_next(|_| {});

        let _ = observable.buffer_with_count(1);
    }
}
