use crate::utils::types::NecessarySend;
use crate::{
    observable::Observable,
    observer::{Observer, Termination},
    subscription::Subscription,
    utils::types::MarkerType,
};
use educe::Educe;
use std::{convert::Infallible, marker::PhantomData};

#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct MapInfallibleToValue<T, OE> {
    source: OE,
    _marker: MarkerType<T>,
}

impl<T, OE> MapInfallibleToValue<T, OE> {
    pub fn new(source: OE) -> Self {
        Self {
            source,
            _marker: PhantomData,
        }
    }
}

impl<'or, 'sub, T, E, OE> Observable<'or, 'sub, T, E> for MapInfallibleToValue<T, OE>
where
    T: 'or,
    OE: Observable<'or, 'sub, Infallible, E>,
{
    fn subscribe(self, observer: impl Observer<T, E> + NecessarySend + 'or) -> Subscription<'sub> {
        let observer = MapInfallibleToValueObserver {
            observer,
            _marker: PhantomData,
        };
        self.source.subscribe(observer)
    }
}

struct MapInfallibleToValueObserver<T, OR> {
    observer: OR,
    _marker: MarkerType<T>,
}

impl<T, E, OR> Observer<Infallible, E> for MapInfallibleToValueObserver<T, OR>
where
    OR: Observer<T, E>,
{
    fn on_next(&mut self, _: Infallible) {
        unreachable!()
    }

    fn on_termination(self, termination: Termination<E>) {
        self.observer.on_termination(termination);
    }
}
