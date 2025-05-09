use super::{Observable, Observer, observable_ext::ObservableExt};
use crate::{observer::boxed_observer::BoxedObserver, subscription::Subscription};

/// https://stackoverflow.com/a/56447952/9315497
pub struct BoxedObservable<'or, 'sub, 'oe, T, E>(
    Box<dyn FnOnce(BoxedObserver<'or, T, E>) -> Subscription<'sub> + Send + 'oe>,
);

impl<'or, 'sub, 'oe, T, E> BoxedObservable<'or, 'sub, 'oe, T, E> {
    pub fn new(observable: impl Observable<'or, 'sub, T, E> + Send + 'oe) -> Self
    where
        T: 'or,
        E: 'or,
    {
        Self(Box::new(|observer| observable.subscribe(observer)))
    }
}

impl<'or, 'sub, T, E> Observable<'or, 'sub, T, E> for BoxedObservable<'or, 'sub, '_, T, E> {
    fn subscribe(self, observer: impl Observer<T, E> + Send + 'or) -> Subscription<'sub> {
        self.0(BoxedObserver::new(observer))
    }
}

impl<T, E> ObservableExt for BoxedObservable<'_, '_, '_, T, E> {}
