use crate::{
    observable::Observable,
    observer::{Observer, Terminal, boxed_observer::BoxedObserver},
    subscription::Subscription,
};
use educe::Educe;
use std::marker::PhantomData;

/// This is an observable that maps the values of the source observable using a mapper function.
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct Map<T0, OE, F> {
    source: OE,
    mapper: F,
    _marker: PhantomData<T0>,
}

impl<T0, OE, F> Map<T0, OE, F> {
    pub fn new<'a, 'b, T, E>(source: OE, mapper: F) -> Map<T0, OE, F>
    where
        OE: Observable<'a, T0, E, MapObserver<'b, T, E, F>>,
        F: FnMut(T0) -> T,
    {
        Map {
            source,
            mapper,
            _marker: PhantomData,
        }
    }
}

impl<'a, 'b, T0, T, E, OR, OE, F> Observable<'a, T, E, OR> for Map<T0, OE, F>
where
    OR: Observer<T, E> + Send + 'b,
    OE: Observable<'a, T0, E, MapObserver<'b, T, E, F>>,
    F: FnMut(T0) -> T,
{
    fn subscribe(self, observer: OR) -> Subscription<'a> {
        let observer = MapObserver {
            observer: BoxedObserver::new(observer),
            mapper: self.mapper,
        };
        self.source.subscribe(observer)
    }
}

pub struct MapObserver<'b, T, E, F> {
    observer: BoxedObserver<'b, T, E>,
    mapper: F,
}

impl<T0, T, E, F> Observer<T0, E> for MapObserver<'_, T, E, F>
where
    F: FnMut(T0) -> T,
{
    fn on_next(&mut self, value: T0) {
        self.observer.on_next((self.mapper)(value))
    }

    fn on_terminal(self, terminal: Terminal<E>) {
        self.observer.on_terminal(terminal)
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
    use std::convert::Infallible;

    #[test]
    fn test_completed() {
        let mut subject: PublishSubject<'_, i32, &str> = PublishSubject::default();
        let checker = CheckingObserver::new();

        // Custom operations
        let observable = subject.clone();
        let observable = observable.map(|value| value.to_string());

        let subscription = observable.subscribe(checker.clone());
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());

        subject.on_next(111);
        assert!(checker.is_values_matched(&["111".to_owned()]));
        assert!(checker.is_unterminated());

        subject.on_terminal(Terminal::<&str>::Completed);
        assert!(checker.is_values_matched(&["111".to_owned()]));
        assert!(checker.is_completed());

        _ = subscription; // keep the subscription alive
    }

    #[test]
    fn test_error() {
        let mut subject: PublishSubject<'_, i32, &str> = PublishSubject::default();
        let checker = CheckingObserver::new();

        // Custom operations
        let observable = subject.clone();
        let observable = observable.map(|value| value.to_string());

        let subscription = observable.subscribe(checker.clone());
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());

        subject.on_next(111);
        assert!(checker.is_values_matched(&["111".to_owned()]));
        assert!(checker.is_unterminated());

        subject.on_terminal(Terminal::Error("error"));
        assert!(checker.is_values_matched(&["111".to_owned()]));
        assert!(checker.is_error("error"));

        _ = subscription; // keep the subscription alive
    }

    #[test]
    fn test_unsubscribe() {
        let mut subject: PublishSubject<'_, i32, &str> = PublishSubject::default();
        let checker_1 = CheckingObserver::new();
        let checker_2 = CheckingObserver::new();

        // Custom operations
        let observable = subject.clone();
        let observable = observable.map(|value| value.to_string());
        let observable_1 = observable;
        let observable_2 = observable_1.clone();

        let subscription_1 = observable_1.subscribe(checker_1.clone());
        let subscription_2 = observable_2.subscribe(checker_2.clone());
        assert!(checker_1.is_values_matched(&[]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[]));
        assert!(checker_2.is_unterminated());

        subject.on_next(111);
        assert!(checker_1.is_values_matched(&["111".to_owned()]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&["111".to_owned()]));
        assert!(checker_2.is_unterminated());

        subscription_1.unsubscribe();
        assert!(checker_1.is_values_matched(&["111".to_owned()]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&["111".to_owned()]));
        assert!(checker_2.is_unterminated());

        subject.on_next(222);
        assert!(checker_1.is_values_matched(&["111".to_owned()]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&["111".to_owned(), "222".to_owned()]));
        assert!(checker_2.is_unterminated());

        subject.on_terminal(Terminal::Error("error"));
        assert!(checker_1.is_values_matched(&["111".to_owned()]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&["111".to_owned(), "222".to_owned()]));
        assert!(checker_2.is_error("error"));

        _ = subscription_2; // keep the subscription alive
    }

    #[test]
    fn test_ref() {
        let value_1 = 111;
        let value_2 = 222;
        let error = 333;

        let checker_1 = CheckingObserver::new();
        let checker_2: CheckingObserver<&i32, &str> = CheckingObserver::new();
        let mut checker_2_cloned = checker_2.clone();

        let mut subject: PublishSubject<'_, &i32, &i32> = PublishSubject::default();

        // Custom operations
        let observable = subject.clone();
        let observable = observable.map(|value| {
            checker_2_cloned.on_next(value);
            &value_2
        });

        let subscription = observable.subscribe(checker_1.clone());
        assert!(checker_1.is_values_matched(&[]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[]));
        assert!(checker_2.is_unterminated());

        subject.on_next(&value_1);
        assert!(checker_1.is_values_matched(&[&value_2]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[&value_1]));
        assert!(checker_2.is_unterminated());

        subject.on_terminal(Terminal::Error(&error));
        assert!(checker_1.is_values_matched(&[&value_2]));
        assert!(checker_1.is_error(&error));
        assert!(checker_2.is_values_matched(&[&value_1]));
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
        let checker = CheckingObserver::new();

        // Custom operations
        let observable = observable.map(|value| {
            *value *= 2;
            (value.to_string(), value)
        });

        let mut checker_cloned_1 = checker.clone();
        let checker_cloned_2 = checker.clone();
        let subscription = observable.subscribe_with_callback(
            |value| {
                checker_cloned_1.on_next(value.0);
                *value.1 *= 2;
            },
            |terminal| match terminal {
                Terminal::Completed => panic!(),
                Terminal::Error(error) => {
                    checker_cloned_2.on_terminal(Terminal::Error(*error));
                    *error *= 2;
                }
            },
        );

        assert!(checker.is_values_matched(&["222".to_owned()]));
        assert!(checker.is_error(222));
        assert_eq!(value, 444);
        assert_eq!(error, 444);

        _ = subscription; // keep the subscription alive
    }

    #[tokio::test]
    async fn test_async() {
        let subject: PublishSubject<'_, i32, _> = PublishSubject::default();
        let checker = CheckingObserver::new();

        // Custom operations
        let observable = subject.clone();
        let observable = observable.map(|value| value.to_string());

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
        assert!(checker.is_values_matched(&["111".to_owned()]));
        assert!(checker.is_unterminated());

        let handle = tokio::spawn(async { subscription.unsubscribe() });
        handle.await.unwrap();
        assert!(checker.is_values_matched(&["111".to_owned()]));
        assert!(checker.is_unterminated());

        let subject_cloned = subject.clone();
        let handle = tokio::spawn(async move {
            subject_cloned.on_terminal(Terminal::Error("error"));
        });
        handle.await.unwrap();
        assert!(checker.is_values_matched(&["111".to_owned()]));
        assert!(checker.is_unterminated());
    }

    #[test]
    fn test_subscribe_by_different_observer() {
        let mut subject: PublishSubject<'_, i32, &str> = PublishSubject::default();
        let checker_1 = CheckingObserver::new();
        let checker_2 = CheckingObserver::new();

        // Custom operations
        let observable = subject.clone();
        let observable = observable.map(|value| value.to_string());
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
        assert!(checker_1.is_values_matched(&["111".to_owned()]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&["111".to_owned()]));
        assert!(checker_2.is_unterminated());

        subject.on_terminal(Terminal::Error("error"));
        assert!(checker_1.is_values_matched(&["111".to_owned()]));
        assert!(checker_1.is_error("error"));
        assert!(checker_2.is_values_matched(&["111".to_owned()]));
        assert!(checker_2.is_error("error"));

        _ = subscription_1; // keep the subscription alive
        _ = subscription_2; // keep the subscription alive
    }

    #[test]
    fn test_multiple_operation() {
        let mut subject: PublishSubject<'_, i32, &str> = PublishSubject::default();
        let checker = CheckingObserver::new();

        // Custom operations
        let observable = subject.clone();
        let observable = observable
            .map(|value| value.to_string())
            .map(|value| value + "?");

        let subscription = observable.subscribe(checker.clone());
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());

        subject.on_next(111);
        assert!(checker.is_values_matched(&["111?".to_owned()]));
        assert!(checker.is_unterminated());

        subject.on_terminal(Terminal::Error("error"));
        assert!(checker.is_values_matched(&["111?".to_owned()]));
        assert!(checker.is_error("error"));

        _ = subscription; // keep the subscription alive
    }

    #[test]
    fn test_without_convenient_api() {
        let mut subject: PublishSubject<'_, i32, &str> = PublishSubject::default();
        let checker = CheckingObserver::new();

        // Custom operations
        let observable = subject.clone();
        let observable = Map::new(observable, |value| value.to_string());

        let subscription = observable.subscribe(checker.clone());
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());

        subject.on_next(111);
        assert!(checker.is_values_matched(&["111".to_owned()]));
        assert!(checker.is_unterminated());

        subject.on_terminal(Terminal::Error("error"));
        assert!(checker.is_values_matched(&["111".to_owned()]));
        assert!(checker.is_error("error"));

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

            let observable = observable.map(|value| value.to_string());

            let checker = CheckingObserver::new();
            checker.is_values_matched(&["1".to_owned()]);
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
            let observable = observable.map(|_: i32| None);

            let mut checker: CheckingObserver<_, Infallible> = CheckingObserver::new();
            checker.on_next(Some(&life_marker_2));
            let subscription = observable.subscribe(checker);

            _ = subscription; // keep the subscription alive
        }
    }

    #[test]
    fn test_fn() {
        let mut s = TestStruct;

        let subject: PublishSubject<'_, i32, &str> = PublishSubject::default();

        // Custom operations
        let observable = subject.clone();
        let observable = observable.map(|value| {
            s.consume_mut();
            value.to_string()
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
        let observable = observable.map(|value| value);
        let _ = observable.clone(); // make sure it's Clone when T is not Clone.
    }

    #[test]
    fn test_type_inference_with_subscribe() {
        // Custom operations
        let subject: PublishSubject<'_, i32, String> = PublishSubject::default();
        let observable = subject.map(|value| value.to_string());

        let observable = observable.buffer_with_count(1);
        let checker = CheckingObserver::new();
        observable.subscribe(checker);
    }

    #[test]
    fn test_type_inference_without_subscribe() {
        // Custom operations
        let subject: PublishSubject<'_, i32, String> = PublishSubject::default();
        let observable = subject.map(|value| value.to_string());

        let _ = observable.buffer_with_count(1);
    }
}
