use super::{Observable, Observer};
use crate::{
    disposable::{Disposable, DisposableExt, boxed_disposal::BoxedDisposal},
    observable::Subscription,
    observer::boxed_observer::BoxedObserver,
    utils::types::MaybeSend,
};

trait ErasedObservable<'or, 'sub, T, E> {
    fn subscribe_boxed(
        self: Box<Self>,
        observer: BoxedObserver<'or, T, E>,
    ) -> Subscription<BoxedDisposal<'sub>>;
}

impl<'or, 'sub, T, E, OE> ErasedObservable<'or, 'sub, T, E> for OE
where
    T: 'or,
    E: 'or,
    OE: Observable<'or, T, E>,
    OE::D: Disposable + MaybeSend + 'sub,
{
    fn subscribe_boxed(
        self: Box<Self>,
        observer: BoxedObserver<'or, T, E>,
    ) -> Subscription<BoxedDisposal<'sub>> {
        Subscription::new(Observable::subscribe(*self, observer).into_boxed())
    }
}

cfg_if::cfg_if! {
    if #[cfg(feature = "single-threaded")] {
        /// Type-erased observable for single-threaded builds to handle this problem <https://stackoverflow.com/q/46620790/9315497>
        pub struct BoxedObservable<'or, 'sub, 'oe, T, E>(
            Box<dyn ErasedObservable<'or, 'sub, T, E> + 'oe>,
        );
    } else {
        /// Type-erased observable for multi-threaded builds to handle this problem <https://stackoverflow.com/q/46620790/9315497>
        pub struct BoxedObservable<'or, 'sub, 'oe, T, E>(
            Box<dyn ErasedObservable<'or, 'sub, T, E> + Send + 'oe>,
        );
    }
}

impl<'or, 'sub, 'oe, T, E> BoxedObservable<'or, 'sub, 'oe, T, E> {
    pub fn new(
        observable: impl Observable<'or, T, E, D = impl Disposable + MaybeSend + 'sub> + MaybeSend + 'oe,
    ) -> Self
    where
        T: 'or,
        E: 'or,
    {
        Self(Box::new(observable))
    }
}

impl<'or, 'sub, T, E> Observable<'or, T, E> for BoxedObservable<'or, 'sub, '_, T, E> {
    type D = BoxedDisposal<'sub>;

    #[inline]
    fn subscribe(self, observer: impl Observer<T, E> + MaybeSend + 'or) -> Subscription<Self::D> {
        self.0.subscribe_boxed(BoxedObserver::new(observer))
    }
}

impl<T, E> std::fmt::Debug for BoxedObservable<'_, '_, '_, T, E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(std::any::type_name::<Self>())
    }
}
