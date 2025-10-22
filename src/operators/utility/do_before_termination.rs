use crate::utils::types::NecessarySend;
use crate::{
    disposable::subscription::Subscription,
    observable::{Observable, observable_ext::ObservableExt},
    observer::{Observer, Termination},
};
use educe::Educe;

/// Invokes a callback when the source Observable terminates (either completes or errors), before the termination notification has been emitted to the downstream observer.
/// See <https://reactivex.io/documentation/operators/do.html>
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::observable_ext::ObservableExt,
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
    pub fn new<'or, 'sub, T, E>(source: OE, callback: F) -> Self
    where
        OE: Observable<'or, 'sub, T, E>,
        F: FnOnce(&Termination<E>),
    {
        Self { source, callback }
    }
}

impl<'or, 'sub, T, E, OE, F> Observable<'or, 'sub, T, E> for DoBeforeTermination<OE, F>
where
    T: 'or,
    E: 'or,
    OE: Observable<'or, 'sub, T, E>,
    F: FnOnce(&Termination<E>) + NecessarySend + 'or,
{
    fn subscribe(self, observer: impl Observer<T, E> + NecessarySend + 'or) -> Subscription<'sub> {
        self.source
            .hook_on_termination(move |observer, termination| {
                (self.callback)(&termination);
                observer.on_termination(termination)
            })
            .subscribe(observer)
    }
}
