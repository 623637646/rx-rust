use crate::{
    observable::{Observable, observable_ext::ObservableExt},
    observer::{Observer, Termination},
    subscription::Subscription,
    utils::marker::MarkerType,
};
use educe::Educe;
use std::{convert::Infallible, marker::PhantomData};

#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct MapInfallibleToError<OE>(OE);

impl<OE> MapInfallibleToError<OE> {
    pub fn new(source: OE) -> Self {
        Self(source)
    }
}

impl<'or, 'sub, T, E, OE> Observable<'or, 'sub, T, E> for MapInfallibleToError<OE>
where
    E: 'or,
    OE: Observable<'or, 'sub, T, Infallible>,
{
    fn subscribe(self, observer: impl Observer<T, E> + Send + 'or) -> Subscription<'sub> {
        let observer = MapInfallibleToErrorObserver {
            observer,
            _marker: PhantomData,
        };
        self.0.subscribe(observer)
    }
}

impl<OE> ObservableExt for MapInfallibleToError<OE> {}

struct MapInfallibleToErrorObserver<E, OR> {
    observer: OR,
    _marker: MarkerType<E>,
}

impl<T, E, OR> Observer<T, Infallible> for MapInfallibleToErrorObserver<E, OR>
where
    OR: Observer<T, E>,
{
    fn on_next(&mut self, value: T) {
        self.observer.on_next(value);
    }

    fn on_termination(self, termination: Termination<Infallible>) {
        match termination {
            Termination::Completed => self.observer.on_termination(Termination::Completed),
            Termination::Error(_) => unreachable!(),
        }
    }
}
