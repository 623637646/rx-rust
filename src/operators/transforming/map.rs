use crate::{
    observable::Observable,
    observer::{Observer, Terminal, boxed_observer::BoxedObserver},
    subscription::Subscription,
};
use std::marker::PhantomData;

/// This is an observable that maps the values of the source observable using a mapper function.
pub struct Map<OE, F, T1> {
    source: OE,
    mapper: F,
    _marker: PhantomData<T1>,
}

impl<OE, F, T1> Map<OE, F, T1> {
    pub fn new<'a, 'b, T2, E>(source: OE, mapper: F) -> Map<OE, F, T1>
    where
        OE: Observable<'a, T1, E, MapObserver<'b, T2, E, F>>,
        F: FnMut(T1) -> T2,
    {
        Map {
            source,
            mapper,
            _marker: PhantomData,
        }
    }
}

impl<OE, F, T1> Clone for Map<OE, F, T1>
where
    OE: Clone,
    F: Clone,
{
    fn clone(&self) -> Self {
        Self {
            source: self.source.clone(),
            mapper: self.mapper.clone(),
            _marker: PhantomData,
        }
    }
}

impl<'a, 'b, T1, T2, E, OR, OE, F> Observable<'a, T2, E, OR> for Map<OE, F, T1>
where
    OR: Observer<T2, E> + Send + 'b,
    OE: Observable<'a, T1, E, MapObserver<'b, T2, E, F>>,
    F: FnMut(T1) -> T2,
{
    fn subscribe(self, observer: OR) -> Subscription<'a> {
        let observer = MapObserver {
            observer: BoxedObserver::new(observer),
            mapper: self.mapper,
        };
        self.source.subscribe(observer)
    }
}

pub struct MapObserver<'b, T2, E, F> {
    observer: BoxedObserver<'b, T2, E>,
    mapper: F,
}

impl<T1, T2, E, F> Observer<T1, E> for MapObserver<'_, T2, E, F>
where
    F: FnMut(T1) -> T2,
{
    fn on_next(&mut self, value: T1) {
        self.observer.on_next((self.mapper)(value))
    }

    fn on_terminal(self, terminal: Terminal<E>) {
        self.observer.on_terminal(terminal)
    }
}

/// Make the `Observable` mappable.
pub trait MappableObservable<T1, T2, E, F>: Sized {
    /// Maps the values of the source observable using a mapper function.
    ///
    /// # Example
    /// ```rust
    /// use rx_rust::operators::creating::just::Just;
    /// use rx_rust::operators::transforming::map::MappableObservable;
    /// use rx_rust::observable::observable_subscribe_ext::ObservableSubscribeExt;
    /// use rx_rust::observer::Terminal;
    /// let observable = Just::new(333);
    /// let observable = observable.map(|value| (value * 3).to_string());
    /// observable.subscribe_on(
    ///     |value| {
    ///         println!("Next value: {}", value);
    ///     },
    ///     |terminal| {
    ///         println!("Terminal event: {:?}", terminal);
    ///     }
    /// );
    /// ```
    fn map(self, f: F) -> Map<Self, F, T1>;
}

impl<'a, 'b, T1, T2, E, F, OE> MappableObservable<T1, T2, E, F> for OE
where
    OE: Observable<'a, T1, E, MapObserver<'b, T2, E, F>>,
    F: FnMut(T1) -> T2,
{
    fn map(self, f: F) -> Map<Self, F, T1> {
        Map::new(self, f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        observable::observable_subscribe_ext::ObservableSubscribeExt,
        operators::creating::{create::Create, just::Just},
        subject::publish_subject::PublishSubject,
        utils::tests_utils::{checking_observer::CheckingObserver, test_struct::TestStruct},
    };

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
        let subscription = observable.subscribe_on(
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

        let (on_next, on_terminal) = checker_2.fn_for_subscribe_on();
        let subscription_2 = observable_2.subscribe_on(on_next, on_terminal);
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
        let checker_1 = CheckingObserver::new();
        let checker_2 = CheckingObserver::new();

        // Custom operations
        let observable = subject.clone();
        let observable = observable
            .map(|value| value.to_string())
            .map(|value| value + "?");

        let (on_next, on_terminal) = checker_1.fn_for_subscribe_on();
        let subscription_1 = observable.clone().subscribe_on(on_next, on_terminal);
        let (on_next, on_terminal) = checker_2.fn_for_subscribe_on();
        let subscription_2 = observable.subscribe_on(on_next, on_terminal);
        assert!(checker_1.is_values_matched(&[]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[]));
        assert!(checker_2.is_unterminated());

        subject.on_next(111);
        assert!(checker_1.is_values_matched(&["111?".to_owned()]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&["111?".to_owned()]));
        assert!(checker_2.is_unterminated());

        subject.on_terminal(Terminal::Error("error"));
        assert!(checker_1.is_values_matched(&["111?".to_owned()]));
        assert!(checker_1.is_error("error"));
        assert!(checker_2.is_values_matched(&["111?".to_owned()]));
        assert!(checker_2.is_error("error"));

        _ = subscription_1; // keep the subscription alive
        _ = subscription_2; // keep the subscription alive
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
    fn test_lifetime() {
        let observable = Just::new(&1);

        // Custom operations
        let observable = observable.map(|value| value);

        let subscription;
        {
            let b = 1;
            let checker = CheckingObserver::new();
            checker.is_values_matched(&[&b]);
            subscription = observable.subscribe(checker);
        }
        _ = subscription; // keep the subscription alive
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

        observable.subscribe_on(|_| {}, |_| {});
    }

    #[test]
    fn test_clone() {
        let observable = Create::new(|mut observer| {
            observer.on_next(TestStruct);
            observer.on_terminal(Terminal::Error(TestStruct));
            Subscription::new_none_disposal()
        });
        let observable = observable.map(|value| value);
        let _ = observable.clone(); // make sure Map is Clone when T is not Clone.
    }
}
