use crate::{
    observable::Observable,
    observer::{Observer, Terminal},
    subscription::Subscription,
};
use std::{
    marker::PhantomData,
    sync::{Arc, Mutex},
};

/// This is an observable that maps the values of the source observable using a mapper function.
#[derive(Clone)]
pub struct Map<OE, F, TF, OR> {
    source: OE,
    mapper: Arc<Mutex<F>>,
    // Adding this to avoid the compiler error: `type annotations needed. multiple `impl`s satisfying `_: observer::Observer<*, *>` found`
    _marker: PhantomData<(TF, OR)>,
}

impl<OE, F, TF, OR> Map<OE, F, TF, OR> {
    pub fn new(source: OE, mapper: F) -> Map<OE, F, TF, OR> {
        Map {
            source,
            mapper: Arc::new(Mutex::new(mapper)),
            _marker: PhantomData,
        }
    }
}

impl<'a, TF, TT, E, OR, OE, F> Observable<'a, TT, E, OR> for Map<OE, F, TF, OR>
where
    OR: Observer<TT, E>,
    OE: Observable<'a, TF, E, MapObserver<OR, F>>,
    F: FnMut(TF) -> TT,
{
    fn subscribe(self, observer: OR) -> Subscription<'a> {
        let observer = MapObserver {
            observer,
            mapper: self.mapper,
        };
        self.source.subscribe(observer)
    }
}

pub struct MapObserver<OR, F> {
    observer: OR,
    mapper: Arc<Mutex<F>>,
}

impl<TF, TT, E, OR, F> Observer<TF, E> for MapObserver<OR, F>
where
    OR: Observer<TT, E>,
    F: FnMut(TF) -> TT,
{
    fn on_next(&mut self, value: TF) {
        self.observer.on_next(self.mapper.lock().unwrap()(value))
    }

    fn on_terminal(self, terminal: Terminal<E>) {
        self.observer.on_terminal(terminal)
    }
}

/// Make the `Observable` mappable.
pub trait MappableObservable<TF, TT, E, OR, F>: Sized {
    /// Maps the values of the source observable using a mapper function.
    ///
    /// # Example
    /// ```rust
    /// use rx_rust::operators::just::Just;
    /// use rx_rust::operators::map::MappableObservable;
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
    fn map(self, f: F) -> Map<Self, F, TF, OR>;
}

impl<'a, TF, TT, E, OR, F, OE> MappableObservable<TF, TT, E, OR, F> for OE
where
    OR: Observer<TT, E>,
    OE: Observable<'a, TF, E, MapObserver<OR, F>>,
    F: FnMut(TF) -> TT,
{
    fn map(self, f: F) -> Map<Self, F, TF, OR> {
        Map::new(self, f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        observable::observable_subscribe_ext::ObservableSubscribeExt,
        operators::{create::Create, just::Just},
        subject::publish_subject::PublishSubject,
        utils::checking_observer::CheckingObserver,
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

        drop(subscription); // keep the subscription alive
    }

    #[test]
    fn test_error() {
        let observable = Create::new(|mut observer| {
            observer.on_next(333);
            observer.on_terminal(Terminal::Error("error".to_owned()));
            Subscription::new_none_disposal()
        });
        let observable = observable.map(|value| value.to_string());
        let checker = CheckingObserver::new();
        observable.subscribe(checker.clone());
        assert!(checker.is_values_matched(&["333".to_owned()]));
        assert!(checker.is_error("error".to_owned()));
    }

    #[test]
    fn test_unterminated() {
        let observable = Create::new(|mut observer| {
            observer.on_next(333);
            observer.on_next(444);
            Subscription::new_none_disposal()
        });
        let observable = observable.map(|value| value.to_string());
        let checker: CheckingObserver<String, String> = CheckingObserver::new();
        let subscription = observable.subscribe(checker.clone());
        assert!(checker.is_values_matched(&["333".to_owned(), "444".to_owned()]));
        assert!(checker.is_unterminated());
        drop(subscription); // keep the subscription alive
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

        drop(subscription_1); // keep the subscription alive
        drop(subscription_2); // keep the subscription alive
    }

    #[test]
    fn test_multiple_subscribe() {
        let observable = Just::new(333);
        let observable = observable.clone().map(|value| value.to_string());

        let checker = CheckingObserver::new();
        observable.clone().subscribe(checker.clone());
        assert!(checker.is_values_matched(&["333".to_owned()]));
        assert!(checker.is_completed());

        let checker = CheckingObserver::new();
        observable.subscribe(checker.clone());
        assert!(checker.is_values_matched(&["333".to_owned()]));
        assert!(checker.is_completed());
    }

    #[test]
    fn test_multiple_operate() {
        let observable = Just::new(333)
            .map(|value| value.to_string())
            .map(|value| value + "?");
        let checker = CheckingObserver::new();
        observable.subscribe(checker.clone());
        assert!(checker.is_values_matched(&["333?".to_owned()]));
        assert!(checker.is_completed());
    }

    #[tokio::test]
    async fn test_async() {
        let observable = Create::new(|mut observer| {
            observer.on_next(1);
            let handle = tokio::spawn(async {
                tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
                observer.on_next(2);
                tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
                observer.on_terminal(Terminal::<Infallible>::Completed);
            });
            Subscription::new_with_disposal_callback(move || handle.abort())
        })
        .map(|value| value.to_string())
        .map(|value| value + "?");
        let checker = CheckingObserver::new();
        let subscription = observable.subscribe(checker.clone());
        assert!(checker.is_values_matched(&["1?".to_owned()]));
        assert!(checker.is_unterminated());
        tokio::time::sleep(tokio::time::Duration::from_millis(5)).await;
        assert!(checker.is_values_matched(&["1?".to_owned()]));
        assert!(checker.is_unterminated());
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        assert!(checker.is_values_matched(&["1?".to_owned(), "2?".to_owned()]));
        assert!(checker.is_unterminated());
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        assert!(checker.is_values_matched(&["1?".to_owned(), "2?".to_owned()]));
        assert!(checker.is_completed());
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        assert!(checker.is_values_matched(&["1?".to_owned(), "2?".to_owned()]));
        assert!(checker.is_completed());
        drop(subscription); // keep the subscription alive
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
            value
        });

        let mut checker_cloned_1 = checker.clone();
        let checker_cloned_2 = checker.clone();
        let subscription = observable.subscribe_on(
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

        assert!(checker.is_values_matched(&[222]));
        assert!(checker.is_error(222));
        assert_eq!(value, 444);
        assert_eq!(error, 444);

        drop(subscription); // keep the subscription alive
    }

    #[test]
    fn test_fn() {
        struct MyStruct;
        impl MyStruct {
            // fn test(self) {}
            fn mut_test(&mut self) {}
            // fn ref_test(&self) {}
        }
        let mut s = MyStruct;

        let subject: PublishSubject<'_, i32, &str> = PublishSubject::default();

        // Custom operations
        let observable = subject.clone();
        let observable = observable.map(|value| {
            s.mut_test();
            value.to_string()
        });

        observable.subscribe_on(|_| {}, |_| {});
    }
}
