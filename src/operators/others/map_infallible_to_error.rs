use crate::{
    observable::Observable,
    observer::{Observer, Terminal},
    subscription::Subscription,
};
use educe::Educe;
use std::{convert::Infallible, marker::PhantomData};

#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct MapInfallibleToError<OE>(OE);

impl<OE> MapInfallibleToError<OE> {
    pub fn new(source: OE) -> Self {
        Self(source)
    }
}

impl<'sub, T, E, OR, OE> Observable<'sub, T, E, OR> for MapInfallibleToError<OE>
where
    OR: Observer<T, E>,
    OE: Observable<'sub, T, Infallible, MapInfallibleToErrorObserver<E, OR>>,
{
    fn subscribe(self, observer: impl Observer<T, E> + Send + 'or) -> Subscription<'sub> {
        let observer = MapInfallibleToErrorObserver {
            observer,
            _marker: PhantomData,
        };
        self.0.subscribe(observer)
    }
}

pub struct MapInfallibleToErrorObserver<E, OR> {
    observer: OR,
    /// Using `PhantomData<fn(E) -> E>` instead of `PhantomData<E>` to make MapInfallibleToErrorObserver being `Send + Sync` when OR is `Send + Sync` but E is not.
    /// For more detail: https://doc.rust-lang.org/nomicon/phantom-data.html#table-of-phantomdata-patterns
    /// But the lifetime of MapInfallibleToErrorObserver is affected by E.
    /// Which means that when OR is 'static but E is not 'static, MapInfallibleToErrorObserver is not 'statac.
    /// TODO: find a better way to fix this, so we can remove `E: 'static` from BufferWithTime.
    /// For more detail: https://users.rust-lang.org/t/getting-phantomdata-to-have-a-static-lifetime/38505
    _marker: PhantomData<fn(E) -> E>,
}

impl<T, E, OR> Observer<T, Infallible> for MapInfallibleToErrorObserver<E, OR>
where
    OR: Observer<T, E>,
{
    fn on_next(&mut self, value: T) {
        self.observer.on_next(value);
    }

    fn on_terminal(self, terminal: Terminal<Infallible>) {
        match terminal {
            Terminal::Completed => self.observer.on_terminal(Terminal::Completed),
            Terminal::Error(_) => unreachable!(),
        }
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

    #[test]
    fn test_completed() {
        let mut subject = PublishSubject::default();
        let checker: CheckingObserver<i32, String> = CheckingObserver::new();

        // Custom operations
        let observable = subject.clone().map_infallible_to_error();

        let subscription = observable.subscribe(checker.clone());
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());

        subject.on_next(111);
        assert!(checker.is_values_matched(&[111]));
        assert!(checker.is_unterminated());

        subject.clone().on_terminal(Terminal::Completed);
        assert!(checker.is_values_matched(&[111]));
        assert!(checker.is_completed());

        _ = subscription; // keep the subscription alive
    }

    #[test]
    fn test_unsubscribe() {
        let mut subject = PublishSubject::default();
        let checker_1: CheckingObserver<i32, String> = CheckingObserver::new();
        let checker_2: CheckingObserver<i32, i32> = CheckingObserver::new();

        // Custom operations
        let observable = subject.clone().map_infallible_to_error();
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

        subject.clone().on_terminal(Terminal::Completed);
        assert!(checker_1.is_values_matched(&[111]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[111, 222]));
        assert!(checker_2.is_completed());

        _ = subscription_2; // keep the subscription alive
    }

    #[test]
    fn test_ref() {
        let value = 111;

        let mut subject = PublishSubject::default();
        let checker: CheckingObserver<&i32, String> = CheckingObserver::new();

        // Custom operations
        let observable = subject.clone().map_infallible_to_error();

        let subscription = observable.subscribe(checker.clone());
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());

        subject.on_next(&value);
        assert!(checker.is_values_matched(&[&value]));
        assert!(checker.is_unterminated());

        subject.clone().on_terminal(Terminal::Completed);
        assert!(checker.is_values_matched(&[&value]));
        assert!(checker.is_completed());

        _ = subscription; // keep the subscription alive
    }

    #[test]
    fn test_mut_ref() {
        let mut value_1 = 111;
        let mut value_2 = 222;
        let mut value_3 = 333;

        // Custom operations
        let observable = Create::new(|mut observer| {
            observer.on_next(&mut value_1);
            observer.on_next(&mut value_2);
            observer.on_next(&mut value_3);
            observer.on_terminal(Terminal::Completed);
            Subscription::new_none_disposal()
        });
        let observable = observable.map_infallible_to_error();

        let subscription = observable.subscribe_with_callback(
            |value| {
                *value *= 2;
            },
            |terminal: Terminal<String>| assert!(matches!(terminal, Terminal::Completed)),
        );

        assert_eq!(value_1, 222);
        assert_eq!(value_2, 444);
        assert_eq!(value_3, 666);

        _ = subscription; // keep the subscription alive
    }

    #[tokio::test]
    async fn test_async() {
        let subject = PublishSubject::default();
        let checker: CheckingObserver<&i32, String> = CheckingObserver::new();

        // Custom operations
        let observable = subject.clone().map_infallible_to_error();

        let checker_cloned = checker.clone();
        let handle = tokio::spawn(async move { observable.subscribe(checker_cloned) });
        let subscription = handle.await.unwrap();
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());

        let mut subject_cloned = subject.clone();
        let handle = tokio::spawn(async move {
            subject_cloned.on_next(&111);
        });
        handle.await.unwrap();
        assert!(checker.is_values_matched(&[&111]));
        assert!(checker.is_unterminated());

        let handle = tokio::spawn(async { subscription.unsubscribe() });
        handle.await.unwrap();
        assert!(checker.is_values_matched(&[&111]));
        assert!(checker.is_unterminated());

        let subject_cloned = subject.clone();
        let handle = tokio::spawn(async move {
            subject_cloned.on_terminal(Terminal::Completed);
        });
        handle.await.unwrap();
        assert!(checker.is_values_matched(&[&111]));
        assert!(checker.is_unterminated());
    }

    #[test]
    fn test_subscribe_by_different_observer() {
        let mut subject = PublishSubject::default();
        let checker_1: CheckingObserver<i32, String> = CheckingObserver::new();
        let checker_2: CheckingObserver<i32, i32> = CheckingObserver::new();

        // Custom operations
        let observable = subject.clone().map_infallible_to_error();
        let observable_1 = observable;
        let observable_2 = observable_1.clone();

        let subscription_1 = observable_1.subscribe(checker_1.clone());

        let (on_next, on_terminal) = checker_2.clone().into_callbacks();
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

        subject.clone().on_terminal(Terminal::Completed);
        assert!(checker_1.is_values_matched(&[111]));
        assert!(checker_1.is_completed());
        assert!(checker_2.is_values_matched(&[111]));
        assert!(checker_2.is_completed());

        _ = subscription_1; // keep the subscription alive
        _ = subscription_2; // keep the subscription alive
    }

    #[test]
    fn test_multiple_operation() {
        let mut subject = PublishSubject::default();
        let checker: CheckingObserver<_, String> = CheckingObserver::new();

        // Custom operations
        let observable = subject.clone();
        let observable = observable
            .map_infallible_to_error()
            .map_infallible_to_error();

        let subscription = observable.subscribe(checker.clone());
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());

        subject.on_next(111);
        assert!(checker.is_values_matched(&[111]));
        assert!(checker.is_unterminated());

        subject.on_terminal(Terminal::Completed);
        assert!(checker.is_values_matched(&[111]));
        assert!(checker.is_completed());

        _ = subscription; // keep the subscription alive
    }

    #[test]
    fn test_without_convenient_api() {
        let mut subject = PublishSubject::default();
        let checker: CheckingObserver<i32, String> = CheckingObserver::new();

        // Custom operations
        let observable = MapInfallibleToError::new(subject.clone());

        let subscription = observable.subscribe(checker.clone());
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());

        subject.on_next(111);
        assert!(checker.is_values_matched(&[111]));
        assert!(checker.is_unterminated());

        subject.clone().on_terminal(Terminal::Completed);
        assert!(checker.is_values_matched(&[111]));
        assert!(checker.is_completed());

        _ = subscription; // keep the subscription alive
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
                Subscription::new_with_disposal_callback(|| {
                    life_marker.consume_ref();
                })
            });
            let observable = observable.map_infallible_to_error();

            let checker: CheckingObserver<i32, String> = CheckingObserver::new();
            subscription = observable.subscribe(checker);
        }

        _ = subscription; // keep the subscription alive
    }

    #[test]
    fn test_clone() {
        let subject: PublishSubject<'_, i32, &i32> = PublishSubject::default();
        let observable = subject.map_infallible_to_error();
        let _ = observable.clone();
    }

    #[test]
    fn test_type_inference_with_subscribe() {
        // Custom operations
        let subject: PublishSubject<'_, i32, _> = PublishSubject::default();
        let observable = subject.map_infallible_to_error();

        let observable = observable.buffer_with_count(1);
        let checker: CheckingObserver<_, String> = CheckingObserver::new();
        observable.subscribe(checker);
    }

    #[test]
    fn test_type_inference_without_subscribe() {
        // Custom operations
        let subject: PublishSubject<'_, i32, &str> = PublishSubject::default();
        let observable = subject.map_infallible_to_error();

        let _ = observable.buffer_with_count(1);
    }
}
