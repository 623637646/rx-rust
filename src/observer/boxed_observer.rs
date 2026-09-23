//! An observer whose concrete type is erased.

use super::{Flow, Observer, Termination};
use crate::utils::types::MaybeSend;

trait ErasedObserver<T, E>: Observer<T, E> {
    fn on_termination_boxed(self: Box<Self>, termination: Termination<E>);
}

impl<T, E, OR> ErasedObserver<T, E> for OR
where
    OR: Observer<T, E>,
{
    fn on_termination_boxed(self: Box<Self>, termination: Termination<E>) {
        Observer::on_termination(*self, termination);
    }
}

cfg_if::cfg_if! {
    if #[cfg(feature = "single-threaded")] {
        type Erased<'or, T, E> = dyn ErasedObserver<T, E> + 'or;
    } else {
        type Erased<'or, T, E> = dyn ErasedObserver<T, E> + Send + 'or;
    }
}

/// An observer whose concrete type is erased.
///
/// [`Observer::on_termination`] takes `self` by value, which a `Box<dyn Observer>` could not call
/// (see <https://stackoverflow.com/q/46620790/9315497>), so the erasure goes through a private
/// trait that terminates a `Box<Self>` instead. In a multi-threaded build the box is also `Send`.
/// Operators that hand the observer to user code, such as `create` and `hook_on_termination`,
/// pass one of these.
///
/// # Examples
/// ```rust
/// use rx_rust::observer::{boxed_observer::BoxedObserver, BoxedObserverExt, Flow, Observer, Termination};
/// use rx_rust::observer::callback_observer::CallbackObserver;
///
/// let mut seen = Vec::new();
/// let mut observer: BoxedObserver<'_, i32, ()> =
///     CallbackObserver::new(|value| seen.push(value), |_| {}).into_boxed();
/// assert_eq!(observer.on_next(1), Flow::Continue);
/// observer.on_termination(Termination::Completed);
/// assert_eq!(seen, [1]);
/// ```
pub struct BoxedObserver<'or, T, E>(Box<Erased<'or, T, E>>);

impl<'or, T, E> BoxedObserver<'or, T, E> {
    /// Boxes `observer`; [`BoxedObserverExt::into_boxed`](crate::observer::BoxedObserverExt::into_boxed)
    /// is the fluent form.
    pub fn new(observer: impl Observer<T, E> + MaybeSend + 'or) -> Self {
        Self(Box::new(observer))
    }
}

impl<T, E> Observer<T, E> for BoxedObserver<'_, T, E> {
    #[inline]
    fn on_next(&mut self, value: T) -> Flow {
        self.0.on_next(value)
    }

    #[inline]
    fn on_termination(self, termination: Termination<E>) {
        self.0.on_termination_boxed(termination);
    }
}

impl<T, E> std::fmt::Debug for BoxedObserver<'_, T, E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(std::any::type_name::<Self>())
    }
}
