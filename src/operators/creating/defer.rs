use crate::{
    observable::{Observable, observable_ext::ObservableExt},
    observer::Observer,
    subscription::Subscription,
};
use educe::Educe;

#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct Defer<F, OE>(F)
where
    F: FnOnce() -> OE;

impl<F, OE> Defer<F, OE>
where
    F: FnOnce() -> OE,
{
    pub fn new(builder: F) -> Defer<F, OE> {
        Defer(builder)
    }
}

impl<'a, T, E, OR, OE, F> Observable<'a, T, E, OR> for Defer<F, OE>
where
    F: FnOnce() -> OE,
    OR: Observer<T, E>,
    OE: Observable<'a, T, E, OR>,
{
    fn subscribe(self, observer: OR) -> Subscription<'a> {
        let observable = self.0();
        observable.subscribe(observer)
    }
}

impl<F, OE> ObservableExt for Defer<F, OE> where F: FnOnce() -> OE {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        observable::boxed_observable::BoxedObservable,
        observer::Terminal,
        operators::creating::{create::Create, just::Just},
        subject::publish_subject::PublishSubject,
        utils::tests_utils::{checking_observer::CheckingObserver, test_struct::TestStruct},
    };
    use std::sync::{Arc, Mutex};

    #[test]
    fn test_completed() {
        let mut subject = PublishSubject::default();
        let checker = CheckingObserver::new();

        // Custom operations
        let observable = subject.clone();
        let observable = Defer::new(|| observable);

        let subscription = observable.subscribe(checker.clone());
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());

        subject.on_next(111);
        assert!(checker.is_values_matched(&[111]));
        assert!(checker.is_unterminated());

        subject.on_terminal(Terminal::<&str>::Completed);
        assert!(checker.is_values_matched(&[111]));
        assert!(checker.is_completed());

        _ = subscription; // keep the subscription alive
    }

    #[test]
    fn test_error() {
        let mut subject = PublishSubject::default();
        let checker = CheckingObserver::new();

        // Custom operations
        let observable = subject.clone();
        let observable = Defer::new(|| observable);

        let subscription = observable.subscribe(checker.clone());
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());

        subject.on_next(111);
        assert!(checker.is_values_matched(&[111]));
        assert!(checker.is_unterminated());

        subject.on_terminal(Terminal::Error("error"));
        assert!(checker.is_values_matched(&[111]));
        assert!(checker.is_error("error"));

        _ = subscription; // keep the subscription alive
    }

    #[test]
    fn test_unsubscribe() {
        let mut subject = PublishSubject::default();
        let checker_1 = CheckingObserver::new();
        let checker_2 = CheckingObserver::new();

        // Custom operations
        let observable = subject.clone();
        let observable = Defer::new(|| observable);
        let observable_1 = observable;
        let observable_2 = observable_1.clone();

        let subscription_1 = observable_1.subscribe(checker_1.clone());
        let subscription_2 = observable_2.subscribe(checker_2.clone());
        assert!(checker_1.is_values_matched(&[]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[]));
        assert!(checker_2.is_unterminated());

        subject.on_next(111);
        assert!(checker_1.is_values_matched(&[111]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[111]));
        assert!(checker_2.is_unterminated());

        subscription_1.unsubscribe();
        assert!(checker_1.is_values_matched(&[111]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[111]));
        assert!(checker_2.is_unterminated());

        subject.on_next(222);
        assert!(checker_1.is_values_matched(&[111]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[111, 222]));
        assert!(checker_2.is_unterminated());

        subject.on_terminal(Terminal::Error("error"));
        assert!(checker_1.is_values_matched(&[111]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[111, 222]));
        assert!(checker_2.is_error("error"));

        _ = subscription_2; // keep the subscription alive
    }

    #[test]
    fn test_ref() {
        let value = 111;
        let error = 222;

        let mut subject = PublishSubject::default();
        let checker = CheckingObserver::new();

        // Custom operations
        let observable = subject.clone();
        let observable = Defer::new(|| observable);

        let subscription = observable.subscribe(checker.clone());
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());
        assert!(subject.terminated().is_none());

        subject.on_next(&value);
        assert!(checker.is_values_matched(&[&value]));
        assert!(checker.is_unterminated());
        assert!(subject.terminated().is_none());

        subject.clone().on_terminal(Terminal::Error(&error));
        assert!(checker.is_values_matched(&[&value]));
        assert!(checker.is_error(&error));
        assert!(matches!(subject.terminated(), Some(Terminal::Error(&222))));

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
        let checker = CheckingObserver::new();

        // Custom operations
        let observable = Defer::new(|| observable);

        let mut checker_cloned_1 = checker.clone();
        let checker_cloned_2 = checker.clone();
        let subscription = observable.subscribe_with_callback(
            |value| {
                checker_cloned_1.on_next(*value);
                *value *= 2;
            },
            |terminal| match terminal {
                Terminal::Completed => panic!(),
                Terminal::Error(error) => {
                    checker_cloned_2.on_terminal(Terminal::Error(*error));
                    *error *= 2;
                }
            },
        );

        assert!(checker.is_values_matched(&[111]));
        assert!(checker.is_error(222));
        assert_eq!(value, 222);
        assert_eq!(error, 444);

        _ = subscription; // keep the subscription alive
    }

    #[tokio::test]
    async fn test_async() {
        let subject = PublishSubject::default();
        let checker = CheckingObserver::new();

        // Custom operations
        let observable = subject.clone();
        let observable = Defer::new(|| observable);

        let checker_cloned = checker.clone();
        let handle = tokio::spawn(async move { observable.subscribe(checker_cloned) });
        let subscription = handle.await.unwrap();
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());

        let mut subject_cloned = subject.clone();
        let handle = tokio::spawn(async move {
            subject_cloned.on_next(111);
        });
        handle.await.unwrap();
        assert!(checker.is_values_matched(&[111]));
        assert!(checker.is_unterminated());

        let handle = tokio::spawn(async { subscription.unsubscribe() });
        handle.await.unwrap();
        assert!(checker.is_values_matched(&[111]));
        assert!(checker.is_unterminated());

        let subject_cloned = subject.clone();
        let handle = tokio::spawn(async move {
            subject_cloned.on_terminal(Terminal::Error("error"));
        });
        handle.await.unwrap();
        assert!(checker.is_values_matched(&[111]));
        assert!(checker.is_unterminated());
    }

    #[test]
    fn test_subscribe_by_different_observer() {
        let mut subject = PublishSubject::default();
        let checker_1 = CheckingObserver::new();
        let checker_2 = CheckingObserver::new();

        // Custom operations
        let observable = subject.clone();
        let observable = Defer::new(|| observable);
        let observable_1 = observable;
        let observable_2 = observable_1.clone();

        let subscription_1 = observable_1.subscribe(checker_1.clone());

        let (on_next, on_terminal) = checker_2.fn_for_subscribe_on();
        let subscription_2 = observable_2.subscribe_with_callback(on_next, on_terminal);
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
        assert!(checker_2.is_error("error"));

        _ = subscription_1; // keep the subscription alive
        _ = subscription_2; // keep the subscription alive
    }

    #[test]
    fn test_lifetime() {
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
            let observable = Defer::new(|| observable);

            let checker = CheckingObserver::new();
            checker.is_values_matched(&[1]);
            subscription = observable.subscribe(checker);
        }

        _ = subscription; // keep the subscription alive
    }

    #[test]
    fn test_fn() {
        let s = TestStruct;

        let subject: PublishSubject<'_, i32, &str> = PublishSubject::default();

        // Custom operations
        let observable = subject.clone();
        let observable = Defer::new(|| {
            s.consume();
            observable
        });

        observable.subscribe_with_callback(|_| {}, |_| {});
    }

    #[test]
    fn test_clone() {
        let observable = Defer::new(|| Just::new(TestStruct));
        let _ = observable.clone(); // make sure it's Clone when OE is not Clone.
    }

    #[test]
    fn test_boxed_observable() {
        // Custom operations
        let switch = Arc::new(Mutex::new(false));
        let observable = Defer::new(|| {
            let observable = Just::new(111);
            if *switch.lock().unwrap() {
                BoxedObservable::new(observable)
            } else {
                BoxedObservable::new(observable.map(|value| value * 2))
            }
        });
        let checker = CheckingObserver::new();
        let subscription = observable.clone().subscribe(checker.clone());
        assert!(checker.is_values_matched(&[222]));
        assert!(checker.is_completed());
        _ = subscription; // keep the subscription alive

        *switch.lock().unwrap() = true;
        let checker = CheckingObserver::new();
        let subscription = observable.subscribe(checker.clone());
        assert!(checker.is_values_matched(&[111]));
        assert!(checker.is_completed());
        _ = subscription; // keep the subscription alive
    }
}
