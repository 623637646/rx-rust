use super::map::Map;
use crate::utils::types::NecessarySend;
use crate::{
    disposable::subscription::Subscription,
    observable::{Observable, observable_ext::ObservableExt},
    observer::Observer,
    utils::types::MarkerType,
};
use educe::Educe;
use std::marker::PhantomData;

#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct ConcatMap<T0, OE, OE1, F> {
    source: OE,
    callback: F,
    _marker: MarkerType<(T0, OE1)>,
}

impl<T0, OE, OE1, F> ConcatMap<T0, OE, OE1, F> {
    pub fn new<'or, 'sub, T, E>(source: OE, callback: F) -> Self
    where
        OE: Observable<'or, 'sub, T0, E>,
        OE1: Observable<'or, 'sub, T, E>,
        F: FnMut(T0) -> OE1,
    {
        Self {
            source,
            callback,
            _marker: PhantomData,
        }
    }
}

impl<'or, 'sub, T0, T, E, OE, OE1, F> Observable<'or, 'sub, T, E> for ConcatMap<T0, OE, OE1, F>
where
    T: 'or,
    E: 'or,
    OE: Observable<'or, 'sub, T0, E>,
    OE1: Observable<'or, 'sub, T, E> + NecessarySend + 'or,
    F: FnMut(T0) -> OE1 + NecessarySend + 'or,
    'sub: 'or,
{
    fn subscribe(self, observer: impl Observer<T, E> + NecessarySend + 'or) -> Subscription<'sub> {
        let observable = Map::new(self.source, self.callback);
        let observable = observable.concat_all();
        observable.subscribe(observer)
    }
}
