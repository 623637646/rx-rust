use crate::utils::types::MaybeSend;
use crate::{
    disposable::Disposable,
    observable::{Observable, Subscription},
    observer::{Observer, boxed_observer::BoxedObserver},
};
use educe::Educe;

/// Creates an Observable from scratch by means of a producer function.
/// See <https://reactivex.io/documentation/operators/create.html>
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::Subscription,
///     observable::ObservableExt,
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
    pub fn new<'or, T, E, D>(builder: F) -> Self
    where
        // Using `Subscription` instead of FnOnce() to make `Create` more easy to wrap other observables. See more in `test_unsubscribe_wrap_observable`.
        D: Disposable,
        F: FnOnce(BoxedObserver<'or, T, E>) -> Subscription<D>,
    {
        Self(builder)
    }
}

impl<'or, T, E, F, D> Observable<'or, T, E> for Create<F>
where
    D: Disposable,
    F: FnOnce(BoxedObserver<'or, T, E>) -> Subscription<D>,
{
    type D = D;

    fn subscribe(self, observer: impl Observer<T, E> + MaybeSend + 'or) -> Subscription<Self::D> {
        self.0(BoxedObserver::new(observer))
    }
}
