use crate::{
    observable::Observable,
    observer::{Observer, Terminal},
    subscription::Subscription,
};
use educe::Educe;
use std::marker::PhantomData;

#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct MapValueToVoid<T, OE> {
    source: OE,
    _marker: PhantomData<fn(T) -> T>, // Refer to `MapInfallibleToErrorObserver` for the reason of using `PhantomData<fn(T) -> T>`
}

impl<T, OE> MapValueToVoid<T, OE> {
    pub fn new(source: OE) -> Self {
        Self {
            source,
            _marker: PhantomData,
        }
    }
}

impl<'or, 'sub, T, E, OE> Observable<'or, 'sub, (), E> for MapValueToVoid<T, OE>
where
    OE: Observable<'or, 'sub, T, E>,
{
    fn subscribe(self, observer: impl Observer<(), E> + Send + 'or) -> Subscription<'sub> {
        let observer = MapValueToVoidObserver(observer);
        self.source.subscribe(observer)
    }
}

pub struct MapValueToVoidObserver<OR>(OR);

impl<T, E, OR> Observer<T, E> for MapValueToVoidObserver<OR>
where
    OR: Observer<(), E>,
{
    fn on_next(&mut self, _: T) {
        self.0.on_next(());
    }

    fn on_terminal(self, terminal: Terminal<E>) {
        self.0.on_terminal(terminal);
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

    #[test]
    fn test_completed() {
        let mut subject: PublishSubject<'_, _, String> = PublishSubject::default();
        let (checker, observer) = Checker::new();

        // Custom operations
        let observable = subject.clone().map_value_to_void();

        let subscription = observable.subscribe(observer);
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());

        subject.on_next(111);
        assert!(checker.is_values_matched(&[()]));
        assert!(checker.is_unterminated());

        subject.clone().on_terminal(Terminal::Completed);
        assert!(checker.is_values_matched(&[()]));
        assert!(checker.is_completed());

        _ = subscription; // keep the subscription alive
    }

    #[test]
    fn test_unsubscribe() {
        let mut subject: PublishSubject<'_, _, String> = PublishSubject::default();
        let (checker_1, observer_1) = Checker::new();
        let (checker_2, observer_2) = Checker::new();

        // Custom operations
        let observable = subject.clone().map_value_to_void();
        let observable_1 = observable;
        let observable_2 = observable_1.clone();

        let subscription_1 = observable_1.subscribe(observer_1);
        let subscription_2 = observable_2.subscribe(observer_2);
        assert!(checker_1.is_values_matched(&[]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[]));
        assert!(checker_2.is_unterminated());

        subject.on_next(111);
        assert!(checker_1.is_values_matched(&[()]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[()]));
        assert!(checker_2.is_unterminated());

        subscription_1.unsubscribe();
        assert!(checker_1.is_values_matched(&[()]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[()]));
        assert!(checker_2.is_unterminated());

        subject.on_next(222);
        assert!(checker_1.is_values_matched(&[()]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[(), ()]));
        assert!(checker_2.is_unterminated());

        subject.clone().on_terminal(Terminal::Completed);
        assert!(checker_1.is_values_matched(&[()]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[(), ()]));
        assert!(checker_2.is_completed());

        _ = subscription_2; // keep the subscription alive
    }

    #[test]
    fn test_ref() {
        let value = 111;

        let mut subject: PublishSubject<'_, _, String> = PublishSubject::default();
        let (checker, observer) = Checker::new();

        // Custom operations
        let observable = subject.clone().map_value_to_void();

        let subscription = observable.subscribe(observer);
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());

        subject.on_next(&value);
        assert!(checker.is_values_matched(&[()]));
        assert!(checker.is_unterminated());

        subject.clone().on_terminal(Terminal::Completed);
        assert!(checker.is_values_matched(&[()]));
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
        let observable = observable.map_value_to_void();

        let subscription = observable.subscribe_with_callback(
            |_| {},
            |terminal: Terminal<String>| assert!(matches!(terminal, Terminal::Completed)),
        );

        assert_eq!(value_1, 111);
        assert_eq!(value_2, 222);
        assert_eq!(value_3, 333);

        _ = subscription; // keep the subscription alive
    }

    #[tokio::test]
    async fn test_async() {
        let subject: PublishSubject<'_, _, String> = PublishSubject::default();
        let (checker, observer) = Checker::new();

        // Custom operations
        let observable = subject.clone().map_value_to_void();

        let checker_cloned = checker.clone();
        let handle = tokio::spawn(async move { observable.subscribe(observer) });
        let subscription = handle.await.unwrap();
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());

        let mut subject_cloned = subject.clone();
        let handle = tokio::spawn(async move {
            subject_cloned.on_next(&111);
        });
        handle.await.unwrap();
        assert!(checker.is_values_matched(&[()]));
        assert!(checker.is_unterminated());

        let handle = tokio::spawn(async { subscription.unsubscribe() });
        handle.await.unwrap();
        assert!(checker.is_values_matched(&[()]));
        assert!(checker.is_unterminated());

        let subject_cloned = subject.clone();
        let handle = tokio::spawn(async move {
            subject_cloned.on_terminal(Terminal::Completed);
        });
        handle.await.unwrap();
        assert!(checker.is_values_matched(&[()]));
        assert!(checker.is_unterminated());
    }

    #[test]
    fn test_subscribe_by_different_observer() {
        let mut subject: PublishSubject<'_, _, String> = PublishSubject::default();
        let (checker_1, observer_1) = Checker::new();
        let (checker_2, observer_2) = Checker::new();

        // Custom operations
        let observable = subject.clone().map_value_to_void();
        let observable_1 = observable;
        let observable_2 = observable_1.clone();

        let subscription_1 = observable_1.subscribe(observer_1);

        let (on_next, on_terminal) = observer_2.into_callbacks();
        let subscription_2 = observable_2.subscribe_with_callback(on_next, on_terminal);
        assert!(checker_1.is_values_matched(&[]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[]));
        assert!(checker_2.is_unterminated());

        subject.on_next(111);
        assert!(checker_1.is_values_matched(&[()]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[()]));
        assert!(checker_2.is_unterminated());

        subject.clone().on_terminal(Terminal::Completed);
        assert!(checker_1.is_values_matched(&[()]));
        assert!(checker_1.is_completed());
        assert!(checker_2.is_values_matched(&[()]));
        assert!(checker_2.is_completed());

        _ = subscription_1; // keep the subscription alive
        _ = subscription_2; // keep the subscription alive
    }

    #[test]
    fn test_multiple_operation() {
        let mut subject: PublishSubject<'_, _, String> = PublishSubject::default();
        let (checker, observer) = Checker::new();

        // Custom operations
        let observable = subject.clone();
        let observable = observable.map_value_to_void().map_value_to_void();

        let subscription = observable.subscribe(observer);
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());

        subject.on_next(111);
        assert!(checker.is_values_matched(&[()]));
        assert!(checker.is_unterminated());

        subject.on_terminal(Terminal::Completed);
        assert!(checker.is_values_matched(&[()]));
        assert!(checker.is_completed());

        _ = subscription; // keep the subscription alive
    }

    #[test]
    fn test_without_convenient_api() {
        let mut subject: PublishSubject<'_, _, String> = PublishSubject::default();
        let (checker, observer) = Checker::new();

        // Custom operations
        let observable = MapValueToVoid::new(subject.clone());

        let subscription = observable.subscribe(observer);
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());

        subject.on_next(111);
        assert!(checker.is_values_matched(&[()]));
        assert!(checker.is_unterminated());

        subject.clone().on_terminal(Terminal::Completed);
        assert!(checker.is_values_matched(&[()]));
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
                observer.on_terminal(Terminal::Error("error"));
                Subscription::new_with_disposal_callback(|| {
                    life_marker.consume_ref();
                })
            });
            let observable = observable.map_value_to_void();

            let (checker, observer) = Checker::new();
            subscription = observable.subscribe(observer);
        }

        _ = subscription; // keep the subscription alive
    }

    #[test]
    fn test_clone() {
        let subject: PublishSubject<'_, i32, &i32> = PublishSubject::default();
        let observable: MapValueToVoid<&str, _> = subject.map_value_to_void();
        let _ = observable.clone();
    }

    #[test]
    fn test_type_inference_with_subscribe() {
        // Custom operations
        let subject: PublishSubject<'_, i32, String> = PublishSubject::default();
        let observable = subject.map_value_to_void();

        let observable = observable.buffer_with_count(1);
        let (checker, observer) = Checker::new();
        observable.subscribe(observer);
    }

    #[test]
    fn test_type_inference_without_subscribe() {
        // Custom operations
        let subject: PublishSubject<'_, i32, &str> = PublishSubject::default();
        let observable: MapValueToVoid<String, _> = subject.map_value_to_void();

        let _ = observable.buffer_with_count(1);
    }
}
