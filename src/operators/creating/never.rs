//! The [`Never`] source.

use crate::thread_mode::Local;
use crate::{
    observable::{Observable, ObservableTypes, Subscription},
    observer::Observer,
};
use std::convert::Infallible;

/// Creates an Observable that emits no items and never terminates.
/// See <https://reactivex.io/documentation/operators/empty-never-throw.html>
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt,
///     operators::creating::never::Never,
/// };
/// use std::convert::Infallible;
///
/// let _subscription = Never.subscribe_with_callback(
///     |_: Infallible| -> () { panic!("`Never` should not emit values") },
///     |_| panic!("`Never` should not terminate"),
/// );
/// ```
#[derive(Debug, Clone)]
pub struct Never;

impl ObservableTypes for Never {
    type Item = Infallible;
    type Error = Infallible;
    type Mode = Local;
    type D = ();
}

impl<OR> Observable<OR> for Never
where
    OR: Observer<Infallible, Infallible>,
{
    fn subscribe(self, _: OR) -> Subscription<Self::D> {
        Subscription::default()
    }
}
