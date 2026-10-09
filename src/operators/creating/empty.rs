//! The [`Empty`] source.

use crate::thread_mode::Local;
use crate::{
    disposable::dispose_on_drop::DisposeOnDrop,
    observable::{Observable, ObservableTypes},
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
///     |_: Infallible| -> () { unreachable!() },
///     |termination| terminations.push(termination),
/// );
///
/// assert_eq!(terminations, vec![Termination::Completed]);
/// ```
#[derive(Debug, Clone)]
pub struct Empty;

impl ObservableTypes for Empty {
    type Item = Infallible;
    type Error = Infallible;
    type Mode = Local;
    type Disposal = ();
}

impl<OR> Observable<OR> for Empty
where
    OR: Observer<Infallible, Infallible>,
{
    fn subscribe(self, observer: OR) -> DisposeOnDrop<Self::Disposal> {
        observer.on_termination(Termination::Completed);
        DisposeOnDrop::default()
    }
}
