use super::{Observable, Observer};
use crate::{
    disposable::subscription::Subscription, observer::boxed_observer::BoxedObserver,
    utils::types::NecessarySend,
};

cfg_if::cfg_if! {
    if #[cfg(feature = "single-threaded")] {
        pub struct BoxedObservable<'or, 'sub, 'oe, T, E>(
            Box<dyn FnOnce(BoxedObserver<'or, T, E>) -> Subscription<'sub> + 'oe>,
        );
    } else {
        /// https://stackoverflow.com/a/56447952/9315497
        pub struct BoxedObservable<'or, 'sub, 'oe, T, E>(
            Box<dyn FnOnce(BoxedObserver<'or, T, E>) -> Subscription<'sub> + Send + 'oe>,
        );
    }
}

impl<'or, 'sub, 'oe, T, E> BoxedObservable<'or, 'sub, 'oe, T, E> {
    pub fn new(observable: impl Observable<'or, 'sub, T, E> + NecessarySend + 'oe) -> Self
    where
        T: 'or,
        E: 'or,
    {
        Self(Box::new(|observer| observable.subscribe(observer)))
    }
}

impl<'or, 'sub, T, E> Observable<'or, 'sub, T, E> for BoxedObservable<'or, 'sub, '_, T, E> {
    fn subscribe(self, observer: impl Observer<T, E> + NecessarySend + 'or) -> Subscription<'sub> {
        self.0(BoxedObserver::new(observer))
    }
}
