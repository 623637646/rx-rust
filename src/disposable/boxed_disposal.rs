//! A disposal whose concrete type is erased.

use crate::disposable::Disposable;
use educe::Educe;

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

macro_rules! boxed_disposal {
    ($(#[$meta:meta])* $name:ident $(, $send:ident)?) => {
        $(#[$meta])*
        #[derive(Educe)]
        #[educe(Debug)]
        pub struct $name<'dis>(#[educe(Debug(ignore))] Box<dyn ErasedDisposable $(+ $send)? + 'dis>);

        impl<'dis> $name<'dis> {
            /// Boxes `disposal`.
            pub fn new(disposal: impl Disposable $(+ $send)? + 'dis) -> Self {
                Self(Box::new(disposal))
            }
        }

        impl Disposable for $name<'_> {
            #[inline]
            fn dispose(self) {
                self.0.dispose_boxed();
            }
        }
    };
}

boxed_disposal!(
    /// A disposal whose concrete type is erased.
    ///
    /// [`Disposable::dispose`] takes `self` by value, which a `Box<dyn Disposable>` could not call
    /// (see <https://stackoverflow.com/q/46620790/9315497>), so the erasure goes through a private
    /// trait that disposes a `Box<Self>` instead. It is not `Send`; [`SendBoxedDisposal`] is.
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
    BoxedDisposal
);

boxed_disposal!(
    /// A [`BoxedDisposal`] that is also `Send`.
    SendBoxedDisposal,
    Send
);
