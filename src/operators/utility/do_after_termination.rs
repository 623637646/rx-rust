use crate::utils::types::MaybeSend;
use crate::{
    observable::Observable,
    observable::Subscription,
    observer::{Observer, Termination},
};
use educe::Educe;

/// Invokes a callback when the source Observable terminates (either completes or errors), after the termination notification has been emitted to the downstream observer.
/// See <https://reactivex.io/documentation/operators/do.html>
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt,
///     observer::Termination,
///     operators::{
///         creating::from_iter::FromIter,
///         utility::do_after_termination::DoAfterTermination,
///     },
/// };
/// use std::sync::{Arc, Mutex};
///
/// let mut terminations = Vec::new();
/// let callback_terminations = Arc::new(Mutex::new(Vec::new()));
/// let callback_terminations_observer = Arc::clone(&callback_terminations);
///
/// DoAfterTermination::new(FromIter::new(vec![1, 2]), move |termination| {
///     callback_terminations_observer
///         .lock()
///         .unwrap()
///         .push(termination);
/// })
/// .subscribe_with_callback(
///     |_| {},
///     |termination| terminations.push(termination),
/// );
///
/// assert_eq!(terminations, vec![Termination::Completed]);
/// assert_eq!(
///     &*callback_terminations.lock().unwrap(),
///     &[Termination::Completed]
/// );
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct DoAfterTermination<OE, F> {
    source: OE,
    callback: F,
}

impl<OE, F> DoAfterTermination<OE, F> {
    pub fn new<'or, T, E>(source: OE, callback: F) -> Self
    where
        OE: Observable<'or, T, E>,
        F: FnOnce(Termination<E>),
    {
        Self { source, callback }
    }
}

impl<'or, T, E, OE, F> Observable<'or, T, E> for DoAfterTermination<OE, F>
where
    T: 'or,
    E: Clone + 'or,
    OE: Observable<'or, T, E>,
    F: FnOnce(Termination<E>) + MaybeSend + 'or,
{
    type D = OE::D;

    fn subscribe(self, observer: impl Observer<T, E> + MaybeSend + 'or) -> Subscription<Self::D> {
        self.source.subscribe(DoAfterTerminationObserver {
            observer,
            callback: self.callback,
        })
    }
}

struct DoAfterTerminationObserver<OR, F> {
    observer: OR,
    callback: F,
}

impl<T, E, OR, F> Observer<T, E> for DoAfterTerminationObserver<OR, F>
where
    E: Clone,
    OR: Observer<T, E>,
    F: FnOnce(Termination<E>),
{
    fn on_next(&mut self, value: T) {
        self.observer.on_next(value);
    }

    fn on_termination(self, termination: Termination<E>) {
        self.observer.on_termination(termination.clone());
        (self.callback)(termination);
    }
}
