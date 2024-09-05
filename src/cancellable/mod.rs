pub mod anonymous_cancellable;
pub mod non_cancellable;

pub trait Cancellable {
    fn cancel(self);
}
