use crate::utils::types::MaybeSend;
use crate::{
    observable::{Observable, Subscription},
    observer::{Observer, Termination},
};
use educe::Educe;
use std::convert::Infallible;

/// Creates an Observable that emits no items and terminates with an error.
/// See <https://reactivex.io/documentation/operators/empty-never-throw.html>
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt,
///     observer::Termination,
///     operators::creating::throw::Throw,
/// };
/// use std::convert::Infallible;
///
/// let mut terminations = Vec::new();
///
/// Throw::new("boom").subscribe_with_callback(
///     |value: Infallible| panic!("`Throw` should not emit values"),
///     |termination| terminations.push(termination),
/// );
///
/// assert_eq!(terminations, vec![Termination::Error("boom")]);
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct Throw<E>(E);

impl<E> Throw<E> {
    pub fn new(error: E) -> Self {
        Self(error)
    }
}

impl<'or, E> Observable<'or> for Throw<E> {
    type T = Infallible;
    type E = E;
    type D = ();

    fn subscribe(
        self,
        observer: impl Observer<Infallible, E> + MaybeSend + 'or,
    ) -> Subscription<Self::D> {
        observer.on_termination(Termination::Error(self.0));
        Subscription::default()
    }
}
