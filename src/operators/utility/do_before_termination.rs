//! The [`DoBeforeTermination`] operator, behind
//! [`ObservableExt::do_before_termination`](crate::observable::ObservableExt::do_before_termination).

use crate::{
    observable::Subscription,
    observable::{Observable, ObservableTypes},
    observer::{Flow, Observer, Termination},
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
    /// Creates a [`DoBeforeTermination`] over `source`;
    /// [`ObservableExt::do_before_termination`](crate::observable::ObservableExt::do_before_termination) is the fluent form.
    pub fn new<T, E>(source: OE, callback: F) -> Self
    where
        OE: ObservableTypes<Item = T, Error = E>,
        F: FnOnce(&Termination<E>),
    {
        Self { source, callback }
    }
}

impl<T, E, OE, F> ObservableTypes for DoBeforeTermination<OE, F>
where
    OE: ObservableTypes<Item = T, Error = E>,
    F: FnOnce(&Termination<E>),
{
    type Item = T;
    type Error = E;
    type Mode = OE::Mode;
    type D = OE::D;
}

impl<T, E, OE, F, OR> Observable<OR> for DoBeforeTermination<OE, F>
where
    OR: Observer<T, E>,
    OE: Observable<DoBeforeTerminationObserver<OR, F>, Item = T, Error = E>,
    F: FnOnce(&Termination<E>),
{
    fn subscribe(self, observer: OR) -> Subscription<Self::D> {
        self.source.subscribe(DoBeforeTerminationObserver {
            observer,
            callback: self.callback,
        })
    }
}

pub struct DoBeforeTerminationObserver<OR, F> {
    observer: OR,
    callback: F,
}

impl<T, E, OR, F> Observer<T, E> for DoBeforeTerminationObserver<OR, F>
where
    OR: Observer<T, E>,
    F: FnOnce(&Termination<E>),
{
    fn on_next(&mut self, value: T) -> Flow {
        self.observer.on_next(value)
    }

    fn on_termination(self, termination: Termination<E>) {
        (self.callback)(&termination);
        self.observer.on_termination(termination);
    }
}
