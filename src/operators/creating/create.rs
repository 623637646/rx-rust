use crate::utils::types::NecessarySend;
use crate::{
    disposable::subscription::Subscription,
    observable::Observable,
    observer::{Observer, boxed_observer::BoxedObserver},
};
use educe::Educe;

/// Creates an Observable from scratch by means of a producer function.
/// See <https://reactivex.io/documentation/operators/create.html>
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct Create<F>(F);

impl<F> Create<F> {
    pub fn new<'or, 'sub, T, E>(builder: F) -> Self
    where
        // Using `Subscription` instead of FnOnce() to make `Create` more easy to wrap other observables. See more in `test_unsubscribe_wrap_observable`.
        F: FnOnce(BoxedObserver<'or, T, E>) -> Subscription<'sub>,
    {
        Self(builder)
    }
}

impl<'or, 'sub, T, E, F> Observable<'or, 'sub, T, E> for Create<F>
where
    F: FnOnce(BoxedObserver<'or, T, E>) -> Subscription<'sub>,
{
    fn subscribe(self, observer: impl Observer<T, E> + NecessarySend + 'or) -> Subscription<'sub> {
        self.0(BoxedObserver::new(observer))
    }
}
