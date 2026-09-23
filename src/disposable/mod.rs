//! Releasing what a subscription holds: the [`Disposable`] trait and its combinators.
//!
//! [`Observable::subscribe`](crate::observable::Observable::subscribe) returns a
//! [`Subscription`], which is a [`BoundDropDisposal`](bound_drop_disposal::BoundDropDisposal):
//! dropping it disposes the disposal inside, which is how a subscription is cancelled. Operators
//! build that disposal out of the small types here, each of which does one thing:
//!
//! - [`()`](empty_disposal) disposes nothing;
//! - [`CallbackDisposal`](callback_disposal::CallbackDisposal) runs a closure;
//! - [`ChainDisposal`] disposes two in order, and
//!   [`EitherDisposal`] one of two;
//! - [`OptionDisposal`] disposes a value that may be absent;
//! - [`SharedDisposal`](shared_disposal::SharedDisposal) is a cloneable slot for a disposal that
//!   is built, or replaced, later;
//! - [`BoxedDisposal`] erases the type.
//!
//! [`DisposableExt`] wraps any disposal into these, and [`delegate_disposal!`] names a deeply
//! nested combination.
//!
//! # Examples
//! ```rust
//! use rx_rust::disposable::{callback_disposal::CallbackDisposal, Disposable, DisposableExt};
//! use std::cell::Cell;
//!
//! let disposed = Cell::new(0);
//! let disposal = CallbackDisposal::new(|| disposed.set(disposed.get() + 1))
//!     .then(CallbackDisposal::new(|| disposed.set(disposed.get() * 10)));
//!
//! disposal.dispose(); // Disposes the first, then the second.
//! assert_eq!(disposed.get(), 10);
//! ```

pub mod bound_drop_disposal;
pub mod boxed_disposal;
pub mod callback_disposal;
pub mod chain_disposal;
mod delegate_disposal;
pub mod either_disposal;
pub mod empty_disposal;
pub mod option_disposal;
pub mod shared_disposal;

pub use crate::delegate_disposal;
use crate::{
    disposable::{
        boxed_disposal::BoxedDisposal, chain_disposal::ChainDisposal,
        either_disposal::EitherDisposal, option_disposal::OptionDisposal,
    },
    observable::Subscription,
    utils::types::MaybeSend,
};

/// A resource that is released exactly once, by consuming it.
///
/// Disposing takes `self`, so a disposal cannot be disposed twice, and a type that must dispose
/// when dropped wraps it in a [`BoundDropDisposal`](bound_drop_disposal::BoundDropDisposal).
pub trait Disposable {
    /// Releases the resource.
    fn dispose(self);
}

/// Combinators available on every [`Disposable`]. See the [module documentation](self).
pub trait DisposableExt: Disposable + Sized {
    /// Erases the type of this disposal.
    fn into_boxed<'dis>(self) -> BoxedDisposal<'dis>
    where
        Self: MaybeSend + 'dis,
    {
        BoxedDisposal::new(self)
    }

    /// Converts this disposal into a subscription whose inner disposal is
    /// created through [`From`].
    fn into_subscription<D>(self) -> Subscription<D>
    where
        D: Disposable + From<Self>,
    {
        Subscription::new(self.into())
    }

    /// Wraps this disposal as the present case of an [`OptionDisposal`].
    fn into_option(self) -> OptionDisposal<Self> {
        OptionDisposal::some(self)
    }

    /// Chains `other` after this disposal, so that disposing the result disposes this one first.
    fn then<D: Disposable>(self, other: D) -> ChainDisposal<Self, D> {
        ChainDisposal::new(self, other)
    }

    /// Wraps this disposal as the left case of an [`EitherDisposal`].
    fn into_left<D2: Disposable>(self) -> EitherDisposal<Self, D2> {
        EitherDisposal::Left(self)
    }

    /// Wraps this disposal as the right case of an [`EitherDisposal`].
    fn into_right<D1: Disposable>(self) -> EitherDisposal<D1, Self> {
        EitherDisposal::Right(self)
    }
}

impl<D> DisposableExt for D where D: Disposable {}
