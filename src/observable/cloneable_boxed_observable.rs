use super::{Observable, Observer};
use crate::{
    disposable::{Disposable, DisposableExt, boxed_disposal::BoxedDisposal},
    observable::Subscription,
    observer::boxed_observer::BoxedObserver,
    utils::types::{MaybeSend, MaybeSync, Shared},
};
use educe::Educe;

trait ErasedCloneableObservable<'or, 'sub, 'oe, T, E> {
    fn subscribe_cloned(
        &self,
        observer: BoxedObserver<'or, T, E>,
    ) -> Subscription<BoxedDisposal<'sub>>;
}

impl<'or, 'sub, 'oe, T, E, OE> ErasedCloneableObservable<'or, 'sub, 'oe, T, E> for OE
where
    T: 'or,
    E: 'or,
    OE: Observable<'or, T, E> + Clone + MaybeSend + 'oe,
    OE::D: Disposable + MaybeSend + 'sub,
{
    fn subscribe_cloned(
        &self,
        observer: BoxedObserver<'or, T, E>,
    ) -> Subscription<BoxedDisposal<'sub>> {
        Subscription::new(self.clone().subscribe(observer).into_boxed())
    }
}

cfg_if::cfg_if! {
    if #[cfg(feature = "single-threaded")] {
        /// Cloneable `BoxedObservable` for single-threaded builds.
        #[derive(Educe)]
        #[educe(Clone)]
        pub struct CloneableBoxedObservable<'or, 'sub, 'oe, T, E>(
            Shared<dyn ErasedCloneableObservable<'or, 'sub, 'oe, T, E> + 'oe>,
        );
    } else {
        /// Cloneable `BoxedObservable` for multi-threaded builds.
        #[derive(Educe)]
        #[educe(Clone)]
        pub struct CloneableBoxedObservable<'or, 'sub, 'oe, T, E>(
            Shared<dyn ErasedCloneableObservable<'or, 'sub, 'oe, T, E> + Send + Sync + 'oe>,
        );
    }
}

impl<'or, 'sub, 'oe, T, E> CloneableBoxedObservable<'or, 'sub, 'oe, T, E> {
    pub fn new(
        observable: impl Observable<'or, T, E, D = impl Disposable + MaybeSend + 'sub>
        + Clone
        + MaybeSend
        + MaybeSync
        + 'oe,
    ) -> Self
    where
        T: 'or,
        E: 'or,
    {
        Self(Shared::new(observable))
    }
}

impl<'or, 'sub, T, E> Observable<'or, T, E> for CloneableBoxedObservable<'or, 'sub, '_, T, E> {
    type D = BoxedDisposal<'sub>;

    #[inline]
    fn subscribe(self, observer: impl Observer<T, E> + MaybeSend + 'or) -> Subscription<Self::D> {
        self.0.subscribe_cloned(BoxedObserver::new(observer))
    }
}

impl<T, E> std::fmt::Debug for CloneableBoxedObservable<'_, '_, '_, T, E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(std::any::type_name::<Self>())
    }
}
