use crate::{
    observable::{Observable, observable_ext::ObservableExt},
    observer::{Observer, Termination},
    subscription::Subscription,
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
    /// Using `PhantomData<fn(E) -> E>` instead of `PhantomData<E>` to make MapInfallibleToErrorObserver being `Send + Sync` when OR is `Send + Sync` but E is not.
    /// For more detail: https://doc.rust-lang.org/nomicon/phantom-data.html#table-of-phantomdata-patterns
    /// But the lifetime of MapInfallibleToErrorObserver is affected by E.
    /// Which means that when OR is 'static but E is not 'static, MapInfallibleToErrorObserver is not 'static.
    /// TODO: find a better way to fix this, so we can remove `E: 'static` from BufferWithTime.
    /// For more detail: https://users.rust-lang.org/t/getting-phantomdata-to-have-a-static-lifetime/38505
    _marker: PhantomData<fn(E) -> E>,
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
