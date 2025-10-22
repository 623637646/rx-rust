use crate::utils::types::NecessarySend;
use crate::{
    disposable::subscription::Subscription,
    observable::Observable,
    observer::{Observer, Termination},
};
use educe::Educe;
use std::convert::Infallible;

/// Creates an Observable that emits no items and then terminates normally.
/// See <https://reactivex.io/documentation/operators/empty-never-throw.html>
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::observable_ext::ObservableExt,
///     observer::Termination,
///     operators::creating::empty::Empty,
/// };
/// use std::convert::Infallible;
///
/// let mut terminations = Vec::new();
///
/// Empty.subscribe_with_callback(
///     |_: Infallible| unreachable!(),
///     |termination| terminations.push(termination),
/// );
///
/// assert_eq!(terminations, vec![Termination::Completed]);
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct Empty;

impl<'or, 'sub> Observable<'or, 'sub, Infallible, Infallible> for Empty {
    fn subscribe(
        self,
        observer: impl Observer<Infallible, Infallible> + NecessarySend + 'or,
    ) -> Subscription<'sub> {
        observer.on_termination(Termination::Completed);
        Subscription::default()
    }
}
