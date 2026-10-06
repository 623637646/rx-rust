//! The [`DoAfterTermination`] operator, behind
//! [`ObservableExt::do_after_termination`](crate::observable::ObservableExt::do_after_termination).

use crate::{
    observable::Subscription,
    observable::{Observable, ObservableTypes},
    observer::{Flow, Observer, Termination},
};
use educe::Educe;

/// Invokes a callback when the source Observable terminates, after the termination has been
/// emitted to the downstream observer.
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
    /// Creates a [`DoAfterTermination`] over `source`;
    /// [`ObservableExt::do_after_termination`](crate::observable::ObservableExt::do_after_termination)
    /// is the fluent form.
    pub fn new<T, E>(source: OE, callback: F) -> Self
    where
        OE: ObservableTypes<Item = T, Error = E>,
        F: FnOnce(Termination<E>),
    {
        Self { source, callback }
    }
}

impl<T, E, OE, F> ObservableTypes for DoAfterTermination<OE, F>
where
    E: Clone,
    OE: ObservableTypes<Item = T, Error = E>,
    F: FnOnce(Termination<E>),
{
    type Item = T;
    type Error = E;
    type Mode = OE::Mode;
    type Disposal = OE::Disposal;
}

impl<T, E, OE, F, OR> Observable<OR> for DoAfterTermination<OE, F>
where
    OR: Observer<T, E>,
    E: Clone,
    OE: Observable<DoAfterTerminationObserver<OR, F>, Item = T, Error = E>,
    F: FnOnce(Termination<E>),
{
    fn subscribe(self, observer: OR) -> Subscription<Self::Disposal> {
        self.source.subscribe(DoAfterTerminationObserver {
            observer,
            callback: self.callback,
        })
    }
}

pub struct DoAfterTerminationObserver<OR, F> {
    observer: OR,
    callback: F,
}

impl<T, E, OR, F> Observer<T, E> for DoAfterTerminationObserver<OR, F>
where
    E: Clone,
    OR: Observer<T, E>,
    F: FnOnce(Termination<E>),
{
    fn on_next(&mut self, value: T) -> Flow {
        self.observer.on_next(value)
    }

    fn on_termination(self, termination: Termination<E>) {
        self.observer.on_termination(termination.clone());
        (self.callback)(termination);
    }
}
