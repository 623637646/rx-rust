use crate::utils::types::NecessarySend;
use crate::{
    disposable::subscription::Subscription,
    observable::Observable,
    observer::{Observer, Termination},
};
use educe::Educe;
use std::convert::Infallible;

/// Creates an Observable that emits a single item and then terminates normally.
/// See <https://reactivex.io/documentation/operators/just.html>
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct Just<T>(T);

impl<T> Just<T> {
    /// Creates a new `Just` observable with the given value.
    ///
    /// # Arguments
    ///
    /// * `value` - The value to emit.
    pub fn new(value: T) -> Self {
        Self(value)
    }
}

impl<'or, 'sub, T> Observable<'or, 'sub, T, Infallible> for Just<T> {
    fn subscribe(
        self,
        mut observer: impl Observer<T, Infallible> + NecessarySend + 'or,
    ) -> Subscription<'sub> {
        observer.on_next(self.0);
        observer.on_termination(Termination::Completed);
        Subscription::default()
    }
}
