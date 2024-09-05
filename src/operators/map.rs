use crate::{
    cancellable::Cancellable,
    observable::Observable,
    observer::{anonymous_observer::AnonymousObserver, Event, Observer},
};
use std::{marker::PhantomData, rc::Rc};

pub struct Map<T, O, F> {
    source: O,
    mapper: Rc<F>,
    /// TODO: Why do we need _marker? in this code. D doesn't use it. which means,
    /// When generics is in Fn() 's return type, we don't need to use _marker.
    /// But when generics is in Fn() 's argument type, we need to use _marker.
    /// TODO: Ask in stackoverflow!
    /*
        impl<'a, T, E, D, F> Observable<'a, T, E> for Create<F>
    where
        D: Fn(),
        F: Fn(&dyn Observer<T, E>) -> D,
    {
        fn subscribe<O>(&'a self, observer: O) -> impl Cancellable
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

impl<T, O, F> Map<T, O, F> {
    pub fn new(source: O, mapper: F) -> Map<T, O, F> {
        Map {
            source,
            mapper: Rc::new(mapper),
            _marker: PhantomData,
        }
    }
}

impl<'a, T, E, O, F, T2> Observable<'a, T2, E> for Map<T, O, F>
where
    F: Fn(T) -> T2 + 'static,
    O: Observable<'a, T, E>,
{
    fn subscribe<O2>(&'a self, observer: O2) -> impl Cancellable
    where
        O2: Observer<T2, E> + 'static,
    {
        let mapper = self.mapper.clone();
        let observer = AnonymousObserver::new(move |event: Event<T, E>| {
            match event {
                Event::Next(value) => observer.on(Event::Next(mapper(value))),
                Event::Terminated(terminated) => observer.on(Event::Terminated(terminated)),
            };
        });
        return self.source.subscribe(observer);
    }
}

// TODO: find a better way to implement this.
trait Mappable<T, E> {
    fn map<F, T2>(self, f: F) -> impl for<'a> Observable<'a, T2, E>
    where
        F: Fn(T) -> T2 + 'static;
}

impl<O, T, E> Mappable<T, E> for O
where
    O: for<'a> Observable<'a, T, E>,
{
    fn map<F, T2>(self, f: F) -> impl for<'a> Observable<'a, T2, E>
    where
        F: Fn(T) -> T2 + 'static,
    {
        Map::new(self, f)
    }
}

// TODO: will do this. unit test.
