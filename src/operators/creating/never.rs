use crate::utils::types::MaybeSend;
use crate::{
    observable::{Observable, Subscription},
    observer::Observer,
};
use educe::Educe;
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
///     |value: Infallible| panic!("`Never` should not emit values"),
///     |_| panic!("`Never` should not terminate"),
/// );
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct Never;

impl<'or> Observable<'or> for Never {
    type T = Infallible;
    type E = Infallible;
    type D = ();

    fn subscribe(
        self,
        _: impl Observer<Infallible, Infallible> + MaybeSend + 'or,
    ) -> Subscription<Self::D> {
        Subscription::default()
    }
}
