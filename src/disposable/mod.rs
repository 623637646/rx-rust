pub mod bound_drop_disposal;
pub mod boxed_disposal;
pub mod callback_disposal;
pub mod disposable_bag;
pub mod disposable_ext;
pub mod subscription;

/// A trait that represents a disposable resource.
pub trait Disposable {
    /// Disposes of the resource.
    fn dispose(self);
}
