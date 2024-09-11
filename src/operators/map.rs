use crate::{
    observable::Observable,
    observer::{anonymous_observer::AnonymousObserver, event::Event, Observer},
    subscription::Subscription,
};
use std::{marker::PhantomData, rc::Rc};

/// This is an observable that maps the values of the source observable using a mapper function.
pub struct Map<'a, T, O, F> {
    source: &'a O,
    mapper: Rc<F>,
    /// TODO: Why do we need _marker?  【Ask in stackoverflow!】
    /// in this code. C doesn't use it. which means,
    /// When generics is in Fn() 's return type, we don't need to use _marker.
    /// But when generics is in Fn() 's argument type, we need to use _marker.
    /*
        impl<'a, T, E, C, F> Observable<'a, T, E> for Create<F>
    where
        C: Fn(),
        F: Fn(&dyn Observer<T, E>) -> C,
    {
        fn subscribe<O>(&'a self, observer: O) -> impl Cancellable+ 'static
        where
            O: Observer<T, E> + 'static,
        {
            let cancellable_closure = (self.subscribe_handler)(&observer);
            AnonymousCancellable::new(cancellable_closure)
        }
    }
         */
    _marker: PhantomData<T>,
}

impl<'a, T, O, F> Map<'a, T, O, F> {
    pub fn new(source: &'a O, mapper: F) -> Map<'a, T, O, F> {
        Map {
            source,
            mapper: Rc::new(mapper),
            _marker: PhantomData,
        }
    }
}

impl<'a, T, E, O, F, T2> Observable<'a, T2, E> for Map<'_, T, O, F>
where
    F: Fn(T) -> T2 + 'static,
    O: for<'b> Observable<'b, T, E>,
{
    fn subscribe(&'a self, observer: impl Observer<T2, E> + 'static) -> Subscription {
        let mapper = self.mapper.clone();
        let observer = AnonymousObserver::new(move |event: Event<T, E>| {
            observer.on(event.map_value(|v| mapper(v)))
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
    fn map<F, T2>(&self, f: F) -> impl for<'a> Observable<'a, T2, E>
    where
        F: Fn(T) -> T2 + 'static;
}

impl<O, T, E> MappableObservable<T, E> for O
where
    O: for<'a> Observable<'a, T, E>,
{
    fn map<F, T2>(&self, f: F) -> impl for<'a> Observable<'a, T2, E>
    where
        F: Fn(T) -> T2 + 'static,
    {
        Map::new(self, f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{operators::just::Just, utils::checking_observer::CheckingObserver};

    #[test]
    fn test_normal() {
        let observable = Just::new(333);
        let observable = observable.map(|value| value.to_string());
        let checker = CheckingObserver::new();
        observable.subscribe(checker.clone());
        assert!(checker.is_values_matched(&["333".to_owned()]));
        assert!(checker.is_completed());
    }

    #[test]
    fn test_multiple_map() {
        let just = Just::new(333);
        let observable1 = just.map(|value| value.to_string());
        let observable2 = just.map(|value| value * 2);
        let checker1 = CheckingObserver::new();
        let checker2 = CheckingObserver::new();
        observable1.subscribe(checker1.clone());
        observable2.subscribe(checker2.clone());
        assert!(checker1.is_values_matched(&["333".to_owned()]));
        assert!(checker1.is_completed());
        assert!(checker2.is_values_matched(&[666]));
        assert!(checker2.is_completed());
    }
}
