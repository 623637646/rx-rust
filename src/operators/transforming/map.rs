use crate::utils::types::NecessarySend;
use crate::{
    observable::Observable,
    observer::{Observer, Termination},
    subscription::Subscription,
    utils::types::MarkerType,
};
use educe::Educe;
use std::marker::PhantomData;

#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct Map<T0, OE, F> {
    source: OE,
    callback: F,
    _marker: MarkerType<T0>,
}

impl<T0, OE, F> Map<T0, OE, F> {
    pub fn new<'or, 'sub, T, E>(source: OE, callback: F) -> Self
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
    OE: Observable<'or, 'sub, T0, E>,
    F: FnMut(T0) -> T + NecessarySend + 'or,
{
    fn subscribe(self, observer: impl Observer<T, E> + NecessarySend + 'or) -> Subscription<'sub> {
        let observer = MapObserver {
            observer,
            callback: self.callback,
        };
        self.source.subscribe(observer)
    }
}

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
