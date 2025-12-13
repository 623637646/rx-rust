use crate::utils::types::NecessarySend;
use crate::{
    disposable::subscription::Subscription,
    observable::Observable,
    observer::{Observer, boxed_observer::BoxedObserver},
};
use educe::Educe;

/// Invokes a callback when the Observable is subscribed to.
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::observable_ext::ObservableExt,
///     observer::Termination,
///     operators::{
///         creating::from_iter::FromIter,
///         others::hook_on_subscription::HookOnSubscription,
///     },
/// };
/// use rx_rust::observable::Observable;
/// use std::cell::Cell;
/// use std::rc::Rc;
///
/// let mut values = Vec::new();
/// let mut terminations = Vec::new();
/// let subscribed = Rc::new(Cell::new(false));
/// let subscribed_flag = Rc::clone(&subscribed);
///
/// let observable = HookOnSubscription::new(FromIter::new(vec![1, 2]), move |source, observer| {
///     subscribed_flag.set(true);
///     source.subscribe(observer)
/// });
/// observable.subscribe_with_callback(
///     |value| values.push(value),
///     |termination| terminations.push(termination),
/// );
///
/// assert!(subscribed.get());
/// assert_eq!(values, vec![1, 2]);
/// assert_eq!(terminations, vec![Termination::Completed]);
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct HookOnSubscription<OE, F> {
    source: OE,
    callback: F,
}

impl<OE, F> HookOnSubscription<OE, F> {
    pub fn new<'or, 'sub, T, E>(source: OE, callback: F) -> Self
    where
        OE: Observable<'or, 'sub, T, E>,
        F: FnOnce(OE, BoxedObserver<'or, T, E>) -> Subscription<'sub>,
    {
        Self { source, callback }
    }
}

impl<'or, 'sub, T, E, OE, F> Observable<'or, 'sub, T, E> for HookOnSubscription<OE, F>
where
    OE: Observable<'or, 'sub, T, E>,
    F: FnOnce(OE, BoxedObserver<'or, T, E>) -> Subscription<'sub>,
{
    fn subscribe(
        self,
        observer: impl Observer<T, E> + NecessarySend + 'or,
    ) -> Subscription<'sub> {
        (self.callback)(self.source, BoxedObserver::new(observer))
    }
}
