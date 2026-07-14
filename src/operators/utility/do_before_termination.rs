use crate::utils::types::MaybeSend;
use crate::{
    observable::Observable,
    observable::Subscription,
    observer::{Observer, Termination},
};
use educe::Educe;

/// Invokes a callback when the source Observable terminates (either completes or errors), before the termination notification has been emitted to the downstream observer.
/// See <https://reactivex.io/documentation/operators/do.html>
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt,
///     observer::Termination,
///     operators::{
///         creating::from_iter::FromIter,
///         utility::do_before_termination::DoBeforeTermination,
///     },
/// };
/// use std::sync::{Arc, Mutex};
///
/// let mut terminations = Vec::new();
/// let callback_terminations = Arc::new(Mutex::new(Vec::new()));
/// let callback_terminations_observer = Arc::clone(&callback_terminations);
///
/// DoBeforeTermination::new(FromIter::new(vec![1, 2]), move |termination| {
///     callback_terminations_observer
///         .lock()
///         .unwrap()
///         .push(termination.clone());
/// })
/// .subscribe_with_callback(
///     |_| {},
///     |termination| terminations.push(termination),
/// );
///
/// assert_eq!(
///     &*callback_terminations.lock().unwrap(),
///     &[Termination::Completed]
/// );
/// assert_eq!(terminations, vec![Termination::Completed]);
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct DoBeforeTermination<OE, F> {
    source: OE,
    callback: F,
}

impl<OE, F> DoBeforeTermination<OE, F> {
    pub fn new<'or, T, E>(source: OE, callback: F) -> Self
    where
        OE: Observable<'or, T, E>,
        F: FnOnce(&Termination<E>),
    {
        Self { source, callback }
    }
}

impl<'or, T, E, OE, F> Observable<'or, T, E> for DoBeforeTermination<OE, F>
where
    T: 'or,
    E: 'or,
    OE: Observable<'or, T, E>,
    F: FnOnce(&Termination<E>) + MaybeSend + 'or,
{
    type D = OE::D;

    fn subscribe(self, observer: impl Observer<T, E> + MaybeSend + 'or) -> Subscription<Self::D> {
        self.source.subscribe(DoBeforeTerminationObserver {
            observer,
            callback: self.callback,
        })
    }
}

struct DoBeforeTerminationObserver<OR, F> {
    observer: OR,
    callback: F,
}

impl<T, E, OR, F> Observer<T, E> for DoBeforeTerminationObserver<OR, F>
where
    OR: Observer<T, E>,
    F: FnOnce(&Termination<E>),
{
    fn on_next(&mut self, value: T) {
        self.observer.on_next(value);
    }

    fn on_termination(self, termination: Termination<E>) {
        (self.callback)(&termination);
        self.observer.on_termination(termination);
    }
}
