use crate::utils::types::MaybeSend;
use crate::{
    disposable::subscription::Subscription,
    observable::Observable,
    observer::{Observer, Termination, boxed_observer::BoxedObserver},
};
use educe::Educe;

/// Invokes a callback when the source Observable terminates.
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::observable_ext::ObservableExt,
///     observer::Termination,
///     operators::{
///         creating::from_iter::FromIter,
///         others::hook_on_termination::HookOnTermination,
///     },
/// };
/// use rx_rust::observer::Observer;
/// use std::cell::Cell;
/// use std::rc::Rc;
///
/// let mut values = Vec::new();
/// let mut terminations = Vec::new();
///
/// let observable =
///     HookOnTermination::new(FromIter::new(vec![1]), move |observer, termination| {
///         // Do whatever you want here
///         observer.on_termination(termination);
///     });
/// observable.subscribe_with_callback(
///     |value| values.push(value),
///     |termination| terminations.push(termination),
/// );
///
/// assert_eq!(values, vec![1]);
/// assert_eq!(terminations, vec![Termination::Completed]);
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct HookOnTermination<OE, F> {
    source: OE,
    callback: F,
}

impl<OE, F> HookOnTermination<OE, F> {
    pub fn new<'or, 'sub, T, E>(source: OE, callback: F) -> Self
    where
        OE: Observable<'or, 'sub, T, E>,
        F: FnOnce(BoxedObserver<'or, T, E>, Termination<E>),
    {
        Self { source, callback }
    }
}

impl<'or, 'sub, T, E, OE, F> Observable<'or, 'sub, T, E> for HookOnTermination<OE, F>
where
    T: 'or,
    E: 'or,
    OE: Observable<'or, 'sub, T, E>,
    F: FnOnce(BoxedObserver<'or, T, E>, Termination<E>) + MaybeSend + 'or,
{
    fn subscribe(self, observer: impl Observer<T, E> + MaybeSend + 'or) -> Subscription<'sub> {
        let observer = HookOnTerminationObserver {
            observer: BoxedObserver::new(observer),
            callback: self.callback,
        };
        self.source.subscribe(observer)
    }
}

struct HookOnTerminationObserver<OR, F> {
    observer: OR,
    callback: F,
}

impl<T, E, OR, F> Observer<T, E> for HookOnTerminationObserver<OR, F>
where
    OR: Observer<T, E>,
    F: FnOnce(OR, Termination<E>),
{
    fn on_next(&mut self, value: T) {
        self.observer.on_next(value);
    }

    fn on_termination(self, termination: Termination<E>) {
        (self.callback)(self.observer, termination);
    }
}
