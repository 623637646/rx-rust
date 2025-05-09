use super::map::Map;
use crate::{
    observable::{Observable, observable_ext::ObservableExt},
    observer::Observer,
    subscription::Subscription,
};
use educe::Educe;
use std::marker::PhantomData;

// Refer to `MapInfallibleToErrorObserver` for the reason of using `PhantomData<fn((T0, OE2)) -> (T0, OE2)>`
type MarkerType<T0, OE2> = PhantomData<fn((T0, OE2)) -> (T0, OE2)>;

#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct FlatMap<T0, OE, OE2, F> {
    source: OE,
    callback: F,
    _marker: MarkerType<T0, OE2>,
}

impl<T0, OE, OE2, F> FlatMap<T0, OE, OE2, F> {
    pub fn new<'sub, 'or, T, E>(source: OE, callback: F) -> Self
    where
        OE: Observable<'or, 'sub, T0, E>,
        OE2: Observable<'or, 'sub, T, E>,
        F: FnMut(T0) -> OE2,
    {
        Self {
            source,
            callback,
            _marker: PhantomData,
        }
    }
}

impl<'or, 'sub, T0, T, E, OE, OE2, F> Observable<'or, 'sub, T, E> for FlatMap<T0, OE, OE2, F>
where
    T: 'or,
    OE: Observable<'or, 'sub, T0, E>,
    OE2: Observable<'or, 'sub, T, E>,
    F: FnMut(T0) -> OE2 + Send + 'or,
    'sub: 'or,
{
    fn subscribe(self, observer: impl Observer<T, E> + Send + 'or) -> Subscription<'sub> {
        let observable = Map::new(self.source, self.callback);
        let observable = observable.merge();
        observable.subscribe(observer)
    }
}

impl<T0, OE, OE2, F> ObservableExt for FlatMap<T0, OE, OE2, F> {}
