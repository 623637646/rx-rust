//! A disposal whose concrete type is erased.

use crate::{disposable::Disposable, utils::types::MaybeSend};

trait ErasedDisposable {
    fn dispose_boxed(self: Box<Self>);
}

impl<D> ErasedDisposable for D
where
    D: Disposable,
{
    fn dispose_boxed(self: Box<Self>) {
        Disposable::dispose(*self);
    }
}

cfg_if::cfg_if! {
    if #[cfg(feature = "single-threaded")] {
        type Erased<'dis> = dyn ErasedDisposable + 'dis;
    } else {
        type Erased<'dis> = dyn ErasedDisposable + Send + 'dis;
    }
}

/// A disposal whose concrete type is erased.
///
/// [`Disposable::dispose`] takes `self` by value, which a `Box<dyn Disposable>` could not call
/// (see <https://stackoverflow.com/q/46620790/9315497>), so the erasure goes through a private
/// trait that disposes a `Box<Self>` instead. In a multi-threaded build the box is also `Send`.
///
/// # Examples
/// ```rust
/// use rx_rust::disposable::{
///     boxed_disposal::BoxedDisposal, callback_disposal::CallbackDisposal, Disposable, DisposableExt,
/// };
///
/// let mut disposed = false;
/// let boxed: BoxedDisposal<'_> = CallbackDisposal::new(|| disposed = true).into_boxed();
/// boxed.dispose();
/// assert!(disposed);
/// ```
pub struct BoxedDisposal<'dis>(Box<Erased<'dis>>);

impl<'dis> BoxedDisposal<'dis> {
    /// Boxes `disposal`; [`DisposableExt::into_boxed`](crate::disposable::DisposableExt::into_boxed)
    /// is the fluent form.
    pub fn new(disposal: impl Disposable + MaybeSend + 'dis) -> Self {
        Self(Box::new(disposal))
    }
}

impl Disposable for BoxedDisposal<'_> {
    #[inline]
    fn dispose(self) {
        self.0.dispose_boxed();
    }
}

impl std::fmt::Debug for BoxedDisposal<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(std::any::type_name::<Self>())
    }
}
