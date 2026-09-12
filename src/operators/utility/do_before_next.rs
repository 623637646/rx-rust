use crate::utils::types::MaybeSend;
use crate::{
    observable::Observable,
    observable::Subscription,
    observer::{Flow, Observer},
};
use educe::Educe;

/// Invokes a callback for each item emitted by the source Observable before the item is emitted to the downstream observer.
/// See <https://reactivex.io/documentation/operators/do.html>
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt,
///     observer::Termination,
///     operators::{
///         creating::from_iter::FromIter,
///         utility::do_before_next::DoBeforeNext,
///     },
/// };
/// use std::sync::{Arc, Mutex};
///
/// let mut values = Vec::new();
/// let mut terminations = Vec::new();
/// let side_effects = Arc::new(Mutex::new(Vec::new()));
/// let side_effects_observer = Arc::clone(&side_effects);
///
/// DoBeforeNext::new(FromIter::new(vec![1, 2]), move |value: &i32| {
///     side_effects_observer.lock().unwrap().push(*value * 10);
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
pub struct DoBeforeNext<OE, F> {
    source: OE,
    callback: F,
}

impl<OE, F> DoBeforeNext<OE, F> {
    pub fn new<'or, T, E>(source: OE, callback: F) -> Self
    where
        OE: Observable<'or, T, E>,
        F: FnMut(&T),
    {
        Self { source, callback }
    }
}

impl<'or, T, E, OE, F> Observable<'or, T, E> for DoBeforeNext<OE, F>
where
    OE: Observable<'or, T, E>,
    F: FnMut(&T) + MaybeSend + 'or,
{
    type D = OE::D;

    fn subscribe(self, observer: impl Observer<T, E> + MaybeSend + 'or) -> Subscription<Self::D> {
        self.source.subscribe(DoBeforeNextObserver {
            observer,
            callback: self.callback,
        })
    }
}

struct DoBeforeNextObserver<OR, F> {
    observer: OR,
    callback: F,
}

impl<T, E, OR, F> Observer<T, E> for DoBeforeNextObserver<OR, F>
where
    OR: Observer<T, E>,
    F: FnMut(&T),
{
    fn on_next(&mut self, value: T) -> Flow {
        (self.callback)(&value);
        self.observer.on_next(value)
    }

    fn on_termination(self, termination: crate::observer::Termination<E>) {
        self.observer.on_termination(termination);
    }
}
