use crate::utils::types::NecessarySendSync;
use crate::{
    disposable::subscription::Subscription,
    observable::Observable,
    observer::{Observer, boxed_observer::BoxedObserver},
};
use educe::Educe;

/// Creates an Observable from scratch by means of a producer function.
/// See <https://reactivex.io/documentation/operators/create.html>
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     disposable::subscription::Subscription,
///     observable::observable_ext::ObservableExt,
///     observer::{boxed_observer::BoxedObserver, Observer, Termination},
///     operators::creating::create::Create,
/// };
///
/// let mut values = Vec::new();
/// let mut terminations = Vec::new();
///
/// let observable = Create::new(|mut observer: BoxedObserver<'_, i32, ()>| {
///     observer.on_next(42);
///     observer.on_termination(Termination::Completed);
///     Subscription::default()
/// });
///
/// observable.subscribe_with_callback(
///     |value| values.push(value),
///     |termination| terminations.push(termination),
/// );
///
/// assert_eq!(values, vec![42]);
/// assert_eq!(terminations, vec![Termination::Completed]);
/// ```
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
    fn subscribe(self, observer: impl Observer<T, E> + NecessarySendSync + 'or) -> Subscription<'sub> {
        self.0(BoxedObserver::new(observer))
    }
}
