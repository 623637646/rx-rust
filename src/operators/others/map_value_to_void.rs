use crate::{
    observable::Observable,
    observer::{Observer, Terminal},
    subscription::Subscription,
};
use educe::Educe;
use std::marker::PhantomData;

#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct MapValueToVoid<T, OE> {
    source: OE,
    _marker: PhantomData<fn(T) -> T>, // Refer to `MapInfallibleToErrorObserver` for the reason of using `PhantomData<fn(T) -> T>`
}

impl<T, OE> MapValueToVoid<T, OE> {
    pub fn new(source: OE) -> Self {
        Self {
            source,
            _marker: PhantomData,
        }
    }
}

impl<'or, 'sub, T, E, OE> Observable<'or, 'sub, (), E> for MapValueToVoid<T, OE>
where
    OE: Observable<'or, 'sub, T, E>,
{
    fn subscribe(self, observer: impl Observer<(), E> + Send + 'or) -> Subscription<'sub> {
        let observer = MapValueToVoidObserver(observer);
        self.source.subscribe(observer)
    }
}

pub struct MapValueToVoidObserver<OR>(OR);

impl<T, E, OR> Observer<T, E> for MapValueToVoidObserver<OR>
where
    OR: Observer<(), E>,
{
    fn on_next(&mut self, _: T) {
        self.0.on_next(());
    }

    fn on_terminal(self, terminal: Terminal<E>) {
        self.0.on_terminal(terminal);
    }
}
