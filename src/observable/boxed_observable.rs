//! An observable whose concrete type is erased.

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
        type Erased<'or, 'sub, 'oe, T, E> = dyn ErasedObservable<'or, 'sub, T, E> + 'oe;
    } else {
        type Erased<'or, 'sub, 'oe, T, E> = dyn ErasedObservable<'or, 'sub, T, E> + Send + 'oe;
    }
}

/// An observable whose concrete type is erased.
///
/// Two observables of different types can then be stored together, or returned from either
/// branch of an `if`. [`Observable::subscribe`] takes `self` by value, which a
/// `Box<dyn Observable>` could not call (see <https://stackoverflow.com/q/46620790/9315497>), so
/// the erasure goes through a private trait that subscribes a `Box<Self>` instead; the observer
/// and the disposal are boxed along with it. In a multi-threaded build the box is also `Send`.
///
/// The lifetimes bound the observer (`'or`), the disposal (`'sub`) and the observable itself
/// (`'oe`); `'static` for all three is the common case.
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::{boxed_observable::BoxedObservable, ObservableExt},
///     operators::creating::{from_iter::FromIter, just::Just},
/// };
/// use std::sync::Mutex;
///
/// let seen = Mutex::new(Vec::new());
/// let observables: Vec<BoxedObservable<'_, 'static, 'static, i32, _>> = vec![
///     Just::new(1).into_boxed(),
///     FromIter::new([2, 3]).into_boxed(),
/// ];
/// for observable in observables {
///     observable.subscribe_with_callback(|value| seen.lock().unwrap().push(value), |_| {});
/// }
/// assert_eq!(*seen.lock().unwrap(), [1, 2, 3]);
/// ```
pub struct BoxedObservable<'or, 'sub, 'oe, T, E>(Box<Erased<'or, 'sub, 'oe, T, E>>);

impl<'or, 'sub, 'oe, T, E> BoxedObservable<'or, 'sub, 'oe, T, E> {
    /// Boxes `observable`; [`ObservableExt::into_boxed`](crate::observable::ObservableExt::into_boxed)
    /// is the fluent form.
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
