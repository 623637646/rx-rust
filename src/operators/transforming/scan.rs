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
pub struct Scan<T, T1, OE, F> {
    source: OE,
    initial_value: T,
    callback: F,
    _marker: MarkerType<T1>,
}

impl<T, T1, OE, F> Scan<T, T1, OE, F> {
    pub fn new<'or, 'sub, E>(source: OE, initial_value: T, callback: F) -> Self
    where
        OE: Observable<'or, 'sub, T1, E>,
        F: FnMut(T, T1) -> T,
    {
        Self {
            source,
            initial_value,
            callback,
            _marker: PhantomData,
        }
    }
}

impl<'or, 'sub, T, T1, E, OE, F> Observable<'or, 'sub, T, E> for Scan<T, T1, OE, F>
where
    T: Clone + NecessarySend + 'or,
    OE: Observable<'or, 'sub, T1, E>,
    F: FnMut(T, T1) -> T + NecessarySend + 'or,
{
    fn subscribe(self, observer: impl Observer<T, E> + NecessarySend + 'or) -> Subscription<'sub> {
        let observer = ScanObserver {
            observer,
            value: self.initial_value,
            callback: self.callback,
        };
        self.source.subscribe(observer)
    }
}

struct ScanObserver<T, OR, F> {
    observer: OR,
    value: T,
    callback: F,
}

impl<T, T1, E, OR, F> Observer<T1, E> for ScanObserver<T, OR, F>
where
    T: Clone,
    OR: Observer<T, E>,
    F: FnMut(T, T1) -> T,
{
    fn on_next(&mut self, value: T1) {
        self.value = (self.callback)(self.value.clone(), value);
        self.observer.on_next(self.value.clone())
    }

    fn on_termination(self, termination: Termination<E>) {
        self.observer.on_termination(termination)
    }
}
