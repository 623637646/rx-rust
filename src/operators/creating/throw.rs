use crate::utils::types::NecessarySend;
use crate::{
    disposable::subscription::Subscription,
    observable::Observable,
    observer::{Observer, Termination},
};
use educe::Educe;
use std::convert::Infallible;

/// Creates an Observable that emits no items and terminates with an error.
/// See <https://reactivex.io/documentation/operators/empty-never-throw.html>
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct Throw<E>(E);

impl<E> Throw<E> {
    pub fn new(error: E) -> Self {
        Self(error)
    }
}

impl<'or, 'sub, E> Observable<'or, 'sub, Infallible, E> for Throw<E> {
    fn subscribe(
        self,
        observer: impl Observer<Infallible, E> + NecessarySend + 'or,
    ) -> Subscription<'sub> {
        observer.on_termination(Termination::Error(self.0));
        Subscription::default()
    }
}
