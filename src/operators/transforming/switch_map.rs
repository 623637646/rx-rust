use super::map::Map;
use crate::{
    observable::{Observable, observable_ext::ObservableExt},
    observer::Observer,
    subscription::Subscription,
    utils::marker::MarkerType,
};
use educe::Educe;
use std::marker::PhantomData;

#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct SwitchMap<T0, OE, OE2, F> {
    source: OE,
    callback: F,
    _marker: MarkerType<(T0, OE2)>,
}

impl<T0, OE, OE2, F> SwitchMap<T0, OE, OE2, F> {
    pub fn new<'or, 'sub, T, E>(source: OE, callback: F) -> Self
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

impl<'or, 'sub, T0, T, E, OE, OE2, F> Observable<'or, 'sub, T, E> for SwitchMap<T0, OE, OE2, F>
where
    T: 'or,
    OE: Observable<'or, 'sub, T0, E>,
    OE2: Observable<'or, 'sub, T, E>,
    F: FnMut(T0) -> OE2 + Send + 'or,
    'sub: 'or,
{
    fn subscribe(self, observer: impl Observer<T, E> + Send + 'or) -> Subscription<'sub> {
        let observable = Map::new(self.source, self.callback);
        let observable = observable.switch();
        observable.subscribe(observer)
    }
}
