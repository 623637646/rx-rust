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
        chain_disposal::ChainDisposal, option_disposal::OptionDisposal,
    },
    safe_lock_option_disposable,
    utils::types::{MaybeSend, Mutable, Shared},
};

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

    fn into_option(self) -> OptionDisposal<Self> {
        OptionDisposal::some(self)
    }

    fn then<D: Disposable>(self, other: D) -> ChainDisposal<Self, D> {
        ChainDisposal::new(self, other)
    }
}

impl<D> DisposableExt for D where D: Disposable {}
