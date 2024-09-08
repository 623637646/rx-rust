use crate::{
    cancellable::Cancellable,
    observable::Observable,
    observer::{anonymous_observer::AnonymousObserver, Event, Observer},
};
use std::{cell::RefCell, marker::PhantomData, rc::Rc};

pub struct Map<T, O, F> {
    source: O,
    mapper: Rc<RefCell<F>>,
    /// TODO: Why do we need _marker? in this code. C doesn't use it. which means,
    /// When generics is in Fn() 's return type, we don't need to use _marker.
    /// But when generics is in Fn() 's argument type, we need to use _marker.
    /// TODO: Ask in stackoverflow!
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

impl<T, O, F> Map<T, O, F> {
    pub fn new(source: O, mapper: F) -> Map<T, O, F> {
        Map {
            source,
            mapper: Rc::new(RefCell::new(mapper)),
            _marker: PhantomData,
        }
    }
}

impl<T, E, O, F, T2> Observable<T2, E> for Map<T, O, F>
where
    F: for<'a> FnMut(&'a T) -> T2 + 'static, // why do we need 'static here? comment out and see what happens.
    O: Observable<T, E>,
{
    fn subscribe(
        &mut self,
        mut observer: impl for<'a> Observer<&'a T2, E> + 'static,
    ) -> impl Cancellable + 'static {
        let mapper = self.mapper.clone();
        let observer = AnonymousObserver::new(move |event: Event<&T, E>| {
            // TODO: why can't use for<'a> here?
            match event {
                Event::Next(value) => {
                    let new_value = mapper.borrow_mut()(value);
                    observer.on(Event::Next(&new_value));
                }
                Event::Terminated(terminated) => observer.on(Event::Terminated(terminated)),
            };
        });
        return self.source.subscribe(observer);
    }
}

trait MappableObservable<T, E> {
    fn map<F, T2>(self, f: F) -> impl Observable<T2, E>
    where
        F: for<'a> FnMut(&'a T) -> T2 + 'static;
}

impl<O, T, E> MappableObservable<T, E> for O
where
    O: Observable<T, E>,
{
    fn map<F, T2>(self, f: F) -> impl Observable<T2, E>
    where
        F: for<'a> FnMut(&'a T) -> T2 + 'static,
    {
        Map::new(self, f)
    }
}

// TODO: will do this. unit test.
