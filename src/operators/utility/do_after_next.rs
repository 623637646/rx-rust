use crate::utils::types::NecessarySendSync;
use crate::{
    disposable::subscription::Subscription,
    observable::{Observable, observable_ext::ObservableExt},
    observer::Observer,
};
use educe::Educe;

/// Invokes a callback for each item emitted by the source Observable after the item has been emitted to the downstream observer.
/// See <https://reactivex.io/documentation/operators/do.html>
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::observable_ext::ObservableExt,
///     observer::Termination,
///     operators::{
///         creating::from_iter::FromIter,
///         utility::do_after_next::DoAfterNext,
///     },
/// };
/// use std::sync::{Arc, Mutex};
///
/// let mut values = Vec::new();
/// let mut terminations = Vec::new();
/// let side_effects = Arc::new(Mutex::new(Vec::new()));
/// let side_effects_observer = Arc::clone(&side_effects);
///
/// DoAfterNext::new(FromIter::new(vec![1, 2]), move |value| {
///     side_effects_observer.lock().unwrap().push(value * 10);
/// })
/// .subscribe_with_callback(
///     |value| values.push(value),
///     |termination| terminations.push(termination),
/// );
///
/// assert_eq!(values, vec![1, 2]);
/// assert_eq!(terminations, vec![Termination::Completed]);
/// assert_eq!(&*side_effects.lock().unwrap(), &[10, 20]);
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct DoAfterNext<OE, F> {
    source: OE,
    callback: F,
}

impl<OE, F> DoAfterNext<OE, F> {
    pub fn new<'or, 'sub, T, E>(source: OE, callback: F) -> Self
    where
        OE: Observable<'or, 'sub, T, E>,
        F: FnMut(T),
    {
        Self { source, callback }
    }
}

impl<'or, 'sub, T, E, OE, F> Observable<'or, 'sub, T, E> for DoAfterNext<OE, F>
where
    T: Clone,
    OE: Observable<'or, 'sub, T, E>,
    F: FnMut(T) + NecessarySendSync + 'or,
{
    fn subscribe(
        mut self,
        observer: impl Observer<T, E> + NecessarySendSync + 'or,
    ) -> Subscription<'sub> {
        self.source
            .hook_on_next(move |observer, value| {
                observer.on_next(value.clone());
                (self.callback)(value);
            })
            .subscribe(observer)
    }
}
