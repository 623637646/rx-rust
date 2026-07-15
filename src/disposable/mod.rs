pub mod bound_drop_disposal;
pub mod boxed_disposal;
pub mod callback_disposal;
pub mod chain_disposal;
mod delegate_disposal;
pub mod empty_disposal;
pub mod option_disposal;
pub mod shared_disposal;
pub use crate::delegate_disposal;
use crate::{
    disposable::{
        bound_drop_disposal::BoundDropDisposal, boxed_disposal::BoxedDisposal,
        chain_disposal::ChainDisposal, either_disposal::EitherDisposal,
        option_disposal::OptionDisposal,
    },
    observable::Subscription,
    safe_lock_option_disposable,
    utils::types::{MaybeSend, Mutable, Shared},
};
pub mod either_disposal;

/// A trait that represents a disposable resource.
pub trait Disposable {
    /// Disposes of the resource.
    fn dispose(self);
}

// TODO: remove this after using SharedDisposal
impl<D> Disposable for Shared<Mutable<Option<D>>>
where
    D: Disposable,
{
    fn dispose(self) {
        safe_lock_option_disposable!(dispose: self);
    }
}

pub trait DisposableExt: Disposable + Sized {
    fn into_boxed<'dis>(self) -> BoxedDisposal<'dis>
    where
        Self: MaybeSend + 'dis,
    {
        BoxedDisposal::new(self)
    }

    fn into_bound_drop(self) -> BoundDropDisposal<Self> {
        BoundDropDisposal::new(self)
    }

    /// Converts this disposal into a subscription whose inner disposal is
    /// created through [`From`].
    fn into_subscription<D>(self) -> Subscription<D>
    where
        D: Disposable + From<Self>,
    {
        Subscription::new(self.into())
    }

    fn into_option(self) -> OptionDisposal<Self> {
        OptionDisposal::some(self)
    }

    fn then<D: Disposable>(self, other: D) -> ChainDisposal<Self, D> {
        ChainDisposal::new(self, other)
    }

    fn into_left<D2: Disposable>(self) -> EitherDisposal<Self, D2> {
        EitherDisposal::Left(self)
    }

    fn into_right<D1: Disposable>(self) -> EitherDisposal<D1, Self> {
        EitherDisposal::Right(self)
    }
}

impl<D> DisposableExt for D where D: Disposable {}
