//! A cloneable observable whose concrete type is erased.

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
        type Erased<'or, 'sub, 'oe, T, E> = dyn ErasedCloneableObservable<'or, 'sub, 'oe, T, E> + 'oe;
    } else {
        type Erased<'or, 'sub, 'oe, T, E> =
            dyn ErasedCloneableObservable<'or, 'sub, 'oe, T, E> + Send + Sync + 'oe;
    }
}

/// A [`BoxedObservable`](super::boxed_observable::BoxedObservable) that can be cloned.
///
/// The observable is kept behind a shared pointer and cloned on every subscription, so the
/// wrapped type must be `Clone` (and `Sync` in a multi-threaded build). This is what an
/// observable of observables holds when the inner ones must be erased, since
/// `PublishSubject` and friends require `T: Clone`.
///
/// # Examples
/// ```rust
/// use rx_rust::{observable::ObservableExt, operators::creating::just::Just};
/// use std::sync::Mutex;
///
/// let seen = Mutex::new(Vec::new());
/// let observable = Just::new(1).into_cloneable_boxed();
/// let copy = observable.clone();
///
/// observable.subscribe_with_callback(|value| seen.lock().unwrap().push(value), |_| {});
/// copy.subscribe_with_callback(|value| seen.lock().unwrap().push(value), |_| {});
/// assert_eq!(*seen.lock().unwrap(), [1, 1]);
/// ```
#[derive(Educe)]
#[educe(Clone)]
pub struct CloneableBoxedObservable<'or, 'sub, 'oe, T, E>(Shared<Erased<'or, 'sub, 'oe, T, E>>);

impl<'or, 'sub, 'oe, T, E> CloneableBoxedObservable<'or, 'sub, 'oe, T, E> {
    /// Boxes `observable`;
    /// [`ObservableExt::into_cloneable_boxed`](crate::observable::ObservableExt::into_cloneable_boxed)
    /// is the fluent form.
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
