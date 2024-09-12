use crate::{
    observable::Observable,
    observer::{anonymous_observer::AnonymousObserver, event::Event, Observer},
    subscription::Subscription,
};
use std::{marker::PhantomData, sync::Arc};

/// This is an observable that maps the values of the source observable using a mapper function.
pub struct Map<T, O, F> {
    source: O,
    mapper: Arc<F>,
    _marker: PhantomData<T>,
}

impl<T, O, F> Map<T, O, F> {
    pub fn new(source: O, mapper: F) -> Map<T, O, F> {
        Map {
            source,
            mapper: Arc::new(mapper),
            _marker: PhantomData,
        }
    }
}

impl<T, O, F> Clone for Map<T, O, F>
where
    O: Clone,
{
    fn clone(&self) -> Self {
        Map {
            source: self.source.clone(),
            mapper: self.mapper.clone(),
            _marker: PhantomData,
        }
    }
}

impl<T, E, O, F, T2> Observable<T2, E> for Map<T, O, F>
where
    T: Sync + Send + 'static,
    F: Fn(T) -> T2 + Sync + Send + 'static,
    O: Observable<T, E>,
{
    fn subscribe(self, observer: impl Observer<T2, E>) -> Subscription {
        let mapper = self.mapper.clone();
        let observer = AnonymousObserver::new(move |event: Event<T, E>| {
            observer.notify_if_unterminated(event.map_value(|v| mapper(v)))
        });
        self.source.subscribe(observer)
    }
}

/// Make the `Observable` mappable.
pub trait MappableObservable<T, E> {
    /**
    Maps the values of the source observable using a mapper function.

    # Example
    ```rust
    use rx_rust::operators::just::Just;
    use rx_rust::operators::map::MappableObservable;
    use rx_rust::observable::observable_subscribe_ext::ObservableSubscribeExt;
    let observable = Just::new(333);
    let observable = observable.map(|value| (value * 3).to_string());
    observable.subscribe_on_event(|event| {
        println!("{:?}", event);
    });
    ```
     */
    fn map<T2>(self, f: impl Fn(T) -> T2 + Sync + Send + 'static) -> impl Observable<T2, E>;
}

impl<O, T, E> MappableObservable<T, E> for O
where
    O: Observable<T, E>,
    T: Sync + Send + 'static,
{
    fn map<T2>(self, f: impl Fn(T) -> T2 + Sync + Send + 'static) -> impl Observable<T2, E> {
        Map::new(self, f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        observer::event::Terminated,
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
        let observable = Create::new(|observer: Box<dyn Observer<i32, String>>| {
            observer.notify_if_unterminated(Event::Next(333));
            observer
                .notify_if_unterminated(Event::Terminated(Terminated::Error("error".to_owned())));
            Subscription::new_non_disposal_action(observer)
        });
        let observable = observable.map(|value| value.to_string());
        let checker = CheckingObserver::new();
        observable.subscribe(checker.clone());
        assert!(checker.is_values_matched(&["333".to_owned()]));
        assert!(checker.is_error("error".to_owned()));
    }

    #[test]
    fn test_unsubscribed() {
        let observable = Create::new(|observer: Box<dyn Observer<i32, String>>| {
            observer.notify_if_unterminated(Event::Next(333));
            observer.notify_if_unterminated(Event::Next(444));
            Subscription::new_non_disposal_action(observer)
        });
        let observable = observable.map(|value| value.to_string());
        let checker = CheckingObserver::new();
        observable.subscribe(checker.clone());
        assert!(checker.is_values_matched(&["333".to_owned(), "444".to_owned()]));
        assert!(checker.is_unsubscribed());
    }

    #[test]
    fn test_unterminated() {
        let observable = Create::new(|observer: Box<dyn Observer<i32, String>>| {
            observer.notify_if_unterminated(Event::Next(333));
            observer.notify_if_unterminated(Event::Next(444));
            Subscription::new_non_disposal_action(observer)
        });
        let observable = observable.map(|value| value.to_string());
        let checker = CheckingObserver::new();
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
        let observable = Create::new(|observer: Box<dyn Observer<i32, String>>| {
            let observer = Arc::new(observer);
            observer.notify_if_unterminated(Event::Next(1));
            let observer_cloned = observer.clone();
            tokio::spawn(async move {
                tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
                observer_cloned.notify_if_unterminated(Event::Next(2));
            });
            let observer_cloned = observer.clone();
            tokio::spawn(async move {
                tokio::time::sleep(tokio::time::Duration::from_millis(20)).await;
                observer_cloned.notify_if_unterminated(Event::Terminated(Terminated::Completed));
            });
            let observer_cloned = observer.clone();
            tokio::spawn(async move {
                tokio::time::sleep(tokio::time::Duration::from_millis(30)).await;
                observer_cloned.notify_if_unterminated(Event::Next(3));
            });
            Subscription::new_non_disposal_action(observer)
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
        _ = subscription; // keep the subscription alive
    }
}
