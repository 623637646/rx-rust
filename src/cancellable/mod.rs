pub mod anonymous_cancellable;
pub mod non_cancellable;

/// A trait for objects that can be cancelled. This is useful for cancelling subscriptions.
pub trait Cancellable {
    fn cancel(self);
}
