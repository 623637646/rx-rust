pub mod binding_drop_disposal;
pub mod boxed_disposal;
pub mod callback_disposal;
pub mod disposable_bag;
#[cfg(feature = "futures")]
pub mod futures_disposable_ext;
pub mod shared_disposal;
pub mod subscription;

/// A trait that represents a disposable resource.
pub trait Disposable {
    /// Disposes of the resource.
    fn dispose(self);
}
