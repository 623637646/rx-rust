use crate::{
    observable::Observable,
    observer::{Observer, Terminal},
    subscription::Subscription,
};
use std::marker::PhantomData;

/// This is an observable that maps the values of the source observable using a mapper function.
pub struct Map<TF, OE, F> {
    source: OE,
    mapper: F,
    _marker: PhantomData<TF>,
}

impl<TF, OE, F> Map<TF, OE, F> {
    pub fn new(source: OE, mapper: F) -> Map<TF, OE, F> {
        Map {
            source,
            mapper,
            _marker: PhantomData,
        }
    }
}

impl<TF, OE, F> Clone for Map<TF, OE, F>
where
    OE: Clone,
    F: Clone,
{
    fn clone(&self) -> Self {
        Map {
            source: self.source.clone(),
            mapper: self.mapper.clone(),
            _marker: PhantomData,
        }
    }
}

impl<TF, TT, E, OR, OE, F> Observable<TT, E, OR> for Map<TF, OE, F>
where
    OR: Observer<TT, E>,
    OE: Observable<TF, E, MapObserver<OR, F>>,
    F: FnMut(TF) -> TT + Clone,
{
    fn subscribe(self, observer: OR) -> Subscription {
        let mapper = self.mapper.clone();
        let observer = MapObserver { observer, mapper };
        self.source.subscribe(observer)
    }
}

pub struct MapObserver<OR, F> {
    observer: OR,
    mapper: F,
}

impl<TF, TT, E, OR, F> Observer<TF, E> for MapObserver<OR, F>
where
    OR: Observer<TT, E>,
    F: FnMut(TF) -> TT,
{
    fn on_next(&mut self, value: TF) {
        self.observer.on_next((self.mapper)(value))
    }

    fn on_terminal(self, terminal: Terminal<E>) {
        self.observer.on_terminal(terminal)
    }
}

/// Make the `Observable` mappable.
pub trait MappableObservable<TF, TT, E, OR, F>
where
    OR: Observer<TT, E>,
{
    /**
    Maps the values of the source observable using a mapper function.

    # Example
    ```rust
    use rx_rust::operators::just::Just;
    use rx_rust::operators::map::MappableObservable;
    use rx_rust::observable::observable_subscribe_ext::ObservableSubscribeExt;
    use rx_rust::observer::Terminal;
    let observable = Just::new(333);
    let observable = observable.map(|value| (value * 3).to_string());
    observable.subscribe_on(
        |value| {
            println!("Next value: {}", value);
        },
        |terminal| {
            println!("Terminal event: {:?}", terminal);
        }
    );
    ```
     */
    fn map(self, f: F) -> impl Observable<TT, E, OR>; // TODO: return impl ... or Map?
}

impl<TF, TT, E, OR, F, OE> MappableObservable<TF, TT, E, OR, F> for OE
where
    OR: Observer<TT, E>,
    OE: Observable<TF, E, MapObserver<OR, F>>,
    F: FnMut(TF) -> TT + Clone,
{
    fn map(self, f: F) -> impl Observable<TT, E, OR> {
        Map::new(self, f)
    }
}

#[cfg(test)]
mod tests {
    use std::convert::Infallible;

    use super::*;
    use crate::{
        operators::{create::Create, just::Just},
        utils::checking_observer::CheckingObserver,
    };

    #[test]
    fn test_completed() {
        let observable = Just::new(333);
        let observable = observable.map(|value| value.to_string());
        let checker = CheckingObserver::new();
        observable.subscribe(checker.clone());
        assert!(checker.is_values_matched(&["333".to_owned()]));
        assert!(checker.is_completed());
    }

    #[test]
    fn test_error() {
        let observable = Create::new(|mut observer| {
            observer.on_next(333);
            observer.on_terminal(Terminal::Error("error".to_owned()));
            Subscription::new_empty()
        });
        let observable = observable.map(|value: i32| value.to_string());
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
            Subscription::new_empty()
        });
        let observable = observable.map(|value: i32| value.to_string());
        let checker: CheckingObserver<String, String> = CheckingObserver::new();
        let subscription = observable.subscribe(checker.clone());
        assert!(checker.is_values_matched(&["333".to_owned(), "444".to_owned()]));
        assert!(checker.is_unterminated());
        _ = subscription; // keep the subscription alive
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
            tokio::spawn(async move {
                tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
                observer.on_next(2);
                tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
                observer.on_terminal(Terminal::<Infallible>::Completed);
            });
            Subscription::new_empty()
        })
        .map(|value: i32| value.to_string())
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
        _ = subscription; // keep the subscription alive
    }
}
