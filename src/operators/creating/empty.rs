use crate::utils::types::MaybeSend;
use crate::{
    observable::{Observable, Subscription},
    observer::{Observer, Termination},
};
use std::convert::Infallible;

/// Creates an Observable that emits no items and then terminates normally.
/// See <https://reactivex.io/documentation/operators/empty-never-throw.html>
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt,
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
#[derive(Debug, Clone)]
pub struct Empty;

impl<'or> Observable<'or, Infallible, Infallible> for Empty {
    type D = ();

    fn subscribe(
        self,
        observer: impl Observer<Infallible, Infallible> + MaybeSend + 'or,
    ) -> Subscription<Self::D> {
        observer.on_termination(Termination::Completed);
        Subscription::default()
    }
}
