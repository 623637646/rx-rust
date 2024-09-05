use crate::cancellable::Cancellable;

pub struct NonCancellable;

impl Cancellable for NonCancellable {
    fn cancel(self) {}
}
