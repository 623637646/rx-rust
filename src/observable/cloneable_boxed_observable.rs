use super::{Observable, Observer};
use crate::{
    disposable::subscription::Subscription,
    observable::{boxed_observable::BoxedObservable, observable_ext::ObservableExt},
    utils::types::{NecessarySend, NecessarySync, Shared},
};
use educe::Educe;

cfg_if::cfg_if! {
    if #[cfg(feature = "single-threaded")] {
        /// Cloneable `BoxedObservable` for single-threaded builds.
        #[derive(Educe)]
        #[educe(Clone)]
        pub struct CloneableBoxedObservable<'or, 'sub, 'oe, T, E>(
            Shared<dyn Fn() -> BoxedObservable<'or, 'sub, 'oe, T, E> + 'oe>,
        );
    } else {
        /// Cloneable `BoxedObservable` for multi-threaded builds.
        #[derive(Educe)]
        #[educe(Clone)]
        pub struct CloneableBoxedObservable<'or, 'sub, 'oe, T, E>(
            Shared<dyn Fn() -> BoxedObservable<'or, 'sub, 'oe, T, E> + Send + Sync + 'oe>,
        );
    }
}

impl<'or, 'sub, 'oe, T, E> CloneableBoxedObservable<'or, 'sub, 'oe, T, E> {
    pub fn new(
        observable: impl Observable<'or, 'sub, T, E> + Clone + NecessarySend + NecessarySync + 'oe,
    ) -> Self
    where
        T: 'or,
        E: 'or,
    {
        Self(Shared::new(move || observable.clone().into_boxed()))
    }
}

impl<'or, 'sub, T, E> Observable<'or, 'sub, T, E>
    for CloneableBoxedObservable<'or, 'sub, '_, T, E>
{
    fn subscribe(self, observer: impl Observer<T, E> + NecessarySend + 'or) -> Subscription<'sub> {
        self.0().subscribe(observer)
    }
}

impl<T, E> std::fmt::Debug for CloneableBoxedObservable<'_, '_, '_, T, E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(std::any::type_name::<Self>())
    }
}
