use super::{Observable, Observer};
use crate::subscription::Subscription;

/// TODO: doc
/// https://stackoverflow.com/a/56447952/9315497
pub struct BoxedObservable<'sub, 'oe, OR>(Box<dyn FnOnce(OR) -> Subscription<'sub> + Send + 'oe>);

impl<'sub, 'oe, OR> BoxedObservable<'sub, 'oe, OR> {
    pub fn new<T, E>(observable: impl Observable<'sub, T, E, OR> + Send + 'oe) -> Self
    where
        OR: Observer<T, E>,
    {
        Self(Box::new(|observer| observable.subscribe(observer)))
    }
}

impl<'sub, T, E, OR> Observable<'sub, T, E, OR> for BoxedObservable<'sub, '_, OR>
where
    OR: Observer<T, E>,
{
    fn subscribe(self, observer: OR) -> Subscription<'sub> {
        self.0(observer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        observable::observable_ext::ObservableExt,
        observer::Terminal,
        operators::creating::{create::Create, just::Just},
        subject::publish_subject::PublishSubject,
        utils::tests_utils::{checking_observer::CheckingObserver, test_struct::TestStruct},
    };

    #[test]
    fn test_completed() {
        let mut subject = PublishSubject::default();
        let checker = CheckingObserver::new();

        // Custom operations
        let observable = BoxedObservable::new(subject.clone());

        let subscription = observable.subscribe(checker.clone());
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());

        subject.on_next(111);
        assert!(checker.is_values_matched(&[111]));
        assert!(checker.is_unterminated());

        subject.clone().on_terminal(Terminal::<&str>::Completed);
        assert!(checker.is_values_matched(&[111]));
        assert!(checker.is_completed());

        _ = subscription; // keep the subscription alive
    }

    #[test]
    fn test_error() {
        let mut subject = PublishSubject::default();
        let checker = CheckingObserver::new();

        // Custom operations
        let observable = BoxedObservable::new(subject.clone());

        let subscription = observable.subscribe(checker.clone());
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());

        subject.on_next(111);
        assert!(checker.is_values_matched(&[111]));
        assert!(checker.is_unterminated());

        subject.clone().on_terminal(Terminal::Error("error"));
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
        let observable_1 = BoxedObservable::new(subject.clone());
        let observable_2 = BoxedObservable::new(subject.clone());

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

        subject.clone().on_terminal(Terminal::Error("error"));
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
        let observable = BoxedObservable::new(subject.clone());

        let subscription = observable.subscribe(checker.clone());
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());

        subject.on_next(&value);
        assert!(checker.is_values_matched(&[&value]));
        assert!(checker.is_unterminated());

        subject.clone().on_terminal(Terminal::Error(&error));
        assert!(checker.is_values_matched(&[&value]));
        assert!(checker.is_error(&error));

        _ = subscription; // keep the subscription alive
    }

    #[test]
    fn test_mut_ref() {
        let mut value = 111;

        let observable = Just::new(&mut value);
        let observable = BoxedObservable::new(observable);

        let checker = CheckingObserver::new();

        let mut checker_cloned_1 = checker.clone();
        let checker_cloned_2 = checker.clone();
        let subscription = observable.subscribe_with_callback(
            |value| {
                checker_cloned_1.on_next(*value);
                *value *= 2;
            },
            |terminal| checker_cloned_2.on_terminal(terminal),
        );

        assert!(checker.is_values_matched(&[111]));
        assert!(checker.is_completed());
        assert_eq!(value, 222);

        _ = subscription; // keep the subscription alive
    }

    #[tokio::test]
    async fn test_async() {
        let subject = PublishSubject::default();
        let checker = CheckingObserver::new();

        // Custom operations
        let observable = BoxedObservable::new(subject.clone());

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
            subject_cloned.on_terminal(Terminal::Error("error"));
        });
        handle.await.unwrap();
        assert!(checker.is_values_matched(&[&111]));
        assert!(checker.is_unterminated());
    }

    #[test]
    fn test_subscribe_by_different_observer() {
        let mut subject = PublishSubject::default();
        let checker_1 = CheckingObserver::new();
        let checker_2 = CheckingObserver::new();

        // Custom operations
        let observable_1 = BoxedObservable::new(subject.clone());
        let observable_2 = BoxedObservable::new(subject.clone());

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

        subject.clone().on_terminal(Terminal::Error("error"));
        assert!(checker_1.is_values_matched(&[111]));
        assert!(checker_1.is_error("error"));
        assert!(checker_2.is_values_matched(&[111]));
        assert!(checker_2.is_error("error"));

        _ = subscription_1; // keep the subscription alive
        _ = subscription_2; // keep the subscription alive
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

            let observable = BoxedObservable::new(observable);

            let checker = CheckingObserver::new();
            checker.is_values_matched(&[1]);
            subscription = observable.subscribe(checker);
        }

        _ = subscription; // keep the subscription alive
    }

    #[test]
    fn test_lifetime_b() {
        // OK
        let life_marker = TestStruct;
        let observable: BoxedObservable<'_, '_, CheckingObserver<i32, String>>;

        // Error
        // let observable: BoxedObservable<'_, '_, CheckingObserver<i32, String>>;
        // let life_marker = TestStruct;

        {
            let create = Create::new(|mut observer| {
                life_marker.consume_ref();
                observer.on_next(1);
                observer.on_terminal(Terminal::<String>::Completed);
                Subscription::new_none_disposal()
            });

            observable = BoxedObservable::new(create);
        }

        _ = observable;
    }

    #[test]
    fn test_type_inference_with_subscribe() {
        // Custom operations
        let subject: PublishSubject<'_, i32, String> = PublishSubject::default();
        let observable = BoxedObservable::new(subject);

        let observable = observable.buffer_with_count(1);
        let checker = CheckingObserver::new();
        observable.subscribe(checker);
    }

    #[test]
    fn test_type_inference_without_subscribe() {
        // Custom operations
        let subject: PublishSubject<'_, i32, String> = PublishSubject::default();
        let observable: BoxedObservable<'_, '_, CheckingObserver<i32, String>> =
            BoxedObservable::new(subject);

        let _ = observable.buffer_with_count(1);
    }
}
