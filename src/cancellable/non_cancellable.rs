use crate::cancellable::Cancellable;

/// A cancellable that does nothing when cancelled.
pub struct NonCancellable;

impl Cancellable for NonCancellable {
    fn cancel(self) {}
}
