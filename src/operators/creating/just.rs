use crate::{
    observable::{Observable, observable_ext::ObservableExt},
    observer::{Observer, Termination},
    subscription::Subscription,
};
use educe::Educe;
use std::convert::Infallible;

/// This is an observable that emits a single value then completes.
///
/// # Example
/// ```rust
/// use rx_rust::operators::creating::just::Just;
/// use rx_rust::observable::observable_ext::ObservableExt;
/// use std::convert::Infallible;
/// use rx_rust::observer::Termination;
/// let observable = Just::new(123);
/// observable.subscribe_with_callback(
///     |value| println!("Next value: {}", value),
///     |termination| println!("Termination event: {:?}", termination)
/// );
/// ```
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
        mut observer: impl Observer<T, Infallible> + Send + 'or,
    ) -> Subscription<'sub> {
        observer.on_next(self.0);
        observer.on_termination(Termination::Completed);
        Subscription::new_none_disposal()
    }
}

impl<T> ObservableExt for Just<T> {}
