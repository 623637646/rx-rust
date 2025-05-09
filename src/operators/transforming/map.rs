use crate::{
    observable::{Observable, observable_ext::ObservableExt},
    observer::{Observer, Termination},
    subscription::Subscription,
};
use educe::Educe;
use std::marker::PhantomData;

/// This is an observable that maps the values of the source observable using a callback.
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct Map<T0, OE, F> {
    source: OE,
    callback: F,
    _marker: PhantomData<fn(T0) -> T0>, // Refer to `MapInfallibleToErrorObserver` for the reason of using `PhantomData<fn(T0) -> T0>`
}

impl<T0, OE, F> Map<T0, OE, F> {
    pub fn new<'sub, 'or, T, E>(source: OE, callback: F) -> Self
    where
        OE: Observable<'or, 'sub, T0, E>,
        F: FnMut(T0) -> T,
    {
        Self {
            source,
            callback,
            _marker: PhantomData,
        }
    }
}

impl<'or, 'sub, T0, T, E, OE, F> Observable<'or, 'sub, T, E> for Map<T0, OE, F>
where
    T: 'or,
    E: 'or,
    OE: Observable<'or, 'sub, T0, E>,
    F: FnMut(T0) -> T + Send + 'or,
{
    fn subscribe(self, observer: impl Observer<T, E> + Send + 'or) -> Subscription<'sub> {
        let observer = MapObserver {
            observer,
            callback: self.callback,
        };
        self.source.subscribe(observer)
    }
}

impl<T0, OE, F> ObservableExt for Map<T0, OE, F> {}

struct MapObserver<OR, F> {
    observer: OR,
    callback: F,
}

impl<T0, T, E, OR, F> Observer<T0, E> for MapObserver<OR, F>
where
    OR: Observer<T, E>,
    F: FnMut(T0) -> T,
{
    fn on_next(&mut self, value: T0) {
        self.observer.on_next((self.callback)(value))
    }

    fn on_termination(self, termination: Termination<E>) {
        self.observer.on_termination(termination)
    }
}
