use crate::{
    observable::{Observable, observable_ext::ObservableExt},
    observer::{Observer, Termination},
    subscription::Subscription,
};
use educe::Educe;
use std::convert::Infallible;

/// This is an observable that emits an error.
///
/// # Example
/// ```rust
/// use rx_rust::operators::creating::throw::Throw;
/// use rx_rust::observable::observable_ext::ObservableExt;
/// use std::convert::Infallible;
/// use rx_rust::observer::Termination;
/// let observable = Throw::new("My error");
/// observable.subscribe_with_callback(
///     |_| {},
///     |termination| println!("Termination event: {:?}", termination)
/// );
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct Throw<E>(E);

impl<E> Throw<E> {
    pub fn new(error: E) -> Self {
        Self(error)
    }
}

impl<'or, 'sub, E> Observable<'or, 'sub, Infallible, E> for Throw<E> {
    fn subscribe(self, observer: impl Observer<Infallible, E> + Send + 'or) -> Subscription<'sub> {
        observer.on_termination(Termination::Error(self.0));
        Subscription::new_none_disposal()
    }
}

impl<E> ObservableExt for Throw<E> {}
