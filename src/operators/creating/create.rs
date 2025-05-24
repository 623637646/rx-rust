use crate::{
    observable::Observable,
    observer::{Observer, boxed_observer::BoxedObserver},
    subscription::Subscription,
};
use educe::Educe;

/// The `Create` struct is an implementation of the `Observable` trait that allows creating an observable
/// from a custom subscription function. The subscription function is provided by the user and is responsible
/// for emitting values and termination events to the observer.
///
/// # Type Parameters
///
/// * `F` - The type of the subscription function.
///
/// The `builder` function is called when an observer subscribes to the observable. It receives
/// a `BoxedObserver` which it can use to emit values and termination events. The function should return a
/// `Subscription` which can be used to manage the subscription.
///
/// # Example
/// ```rust
/// use rx_rust::observable::;
/// use rx_rust::observer::Observer;
/// use rx_rust::subscription::Subscription;
/// use rx_rust::operators::creating::create::Create;
/// use rx_rust::observer::Termination;
/// let observable = Create::new(|mut observer| {
///     observer.on_next(1);
///     observer.on_next(2);
///     observer.on_next(3);
///     observer.on_termination(Termination::Completed);
///     Subscription::new_none_disposal()
/// });
/// observable.subscribe_with_callback(
///     |value| println!("value: {}", value),
///     |termination: Termination<String>| println!("termination: {:?}", termination),
/// );
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct Create<F>(F);

impl<F> Create<F> {
    /// Creates a new `Create` observable.
    ///
    /// # Arguments
    ///
    /// * `builder` - The subscription builder function. It receives a `BoxedObserver` which it can use to emit values and termination events. The function should return a `Subscription` which can be used to manage the subscription.
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
    fn subscribe(self, observer: impl Observer<T, E> + Send + 'or) -> Subscription<'sub> {
        self.0(BoxedObserver::new(observer))
    }
}
