use crate::utils::types::{MarkerType, MaybeSend};
use crate::{
    disposable::Disposable,
    observable::{Observable, Subscription},
    observer::{Observer, boxed_observer::BoxedObserver},
};
use educe::Educe;
use std::marker::PhantomData;

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
// `T` and `E` are struct parameters (not only impl parameters) because they appear
// solely in argument position of `F`'s `FnOnce` bound, which cannot constrain
// impl-level type parameters (E0207) now that they are associated types of `Observable`.
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct Create<T, E, F>(F, MarkerType<(T, E)>);

impl<T, E, F> Create<T, E, F> {
    pub fn new<'or, D>(builder: F) -> Self
    where
        // Using `Subscription` instead of FnOnce() to make `Create` more easy to wrap other observables. See more in `test_unsubscribe_wrap_observable`.
        D: Disposable,
        F: FnOnce(BoxedObserver<'or, T, E>) -> Subscription<D>,
    {
        Self(builder, PhantomData)
    }
}

impl<'or, T, E, F, D> Observable<'or> for Create<T, E, F>
where
    D: Disposable,
    F: FnOnce(BoxedObserver<'or, T, E>) -> Subscription<D>,
{
    type T = T;
    type E = E;
    type D = D;

    fn subscribe(self, observer: impl Observer<T, E> + MaybeSend + 'or) -> Subscription<Self::D> {
        self.0(BoxedObserver::new(observer))
    }
}
