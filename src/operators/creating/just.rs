use crate::utils::types::NecessarySend;
use crate::{
    observable::Observable,
    observer::{Observer, Termination},
    subscription::Subscription,
};
use educe::Educe;
use std::convert::Infallible;

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
        Subscription::new_none_disposal()
    }
}
