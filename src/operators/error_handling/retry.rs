use crate::disposable::subscription::Subscription;
use crate::safe_lock_option;
use crate::utils::types::{Mutable, NecessarySend, Shared};
use crate::{
    observable::Observable,
    observer::{Observer, Termination},
};
use educe::Educe;

#[derive(Educe)]
#[educe(Debug, Clone, PartialEq, Eq)]
pub enum RetryAction<E, OE1> {
    Retry(OE1),
    Stop(E),
}

/// Retries an Observable in case of an error, based on a retry policy.
/// See <https://reactivex.io/documentation/operators/retry.html>
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::observable_ext::ObservableExt,
///     observer::Termination,
///     operators::{
///         creating::{just::Just, throw::Throw},
///         error_handling::retry::{Retry, RetryAction},
///     },
/// };
///
/// let mut values = Vec::new();
/// let mut terminations = Vec::new();
///
/// let observable = Retry::new(Throw::new("boom").map_infallible_to_value(), |_| RetryAction::Retry(Just::new(42).map_infallible_to_error()));
/// observable.subscribe_with_callback(
///     |value| values.push(value),
///     |termination| terminations.push(termination),
/// );
///
/// assert_eq!(values, vec![42]);
/// assert_eq!(terminations, vec![Termination::Completed]);
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct Retry<OE, F> {
    source: OE,
    callback: F,
}

impl<OE, F> Retry<OE, F> {
    pub fn new<'or, 'sub, T, E, OE1>(source: OE, callback: F) -> Self
    where
        OE: Observable<'or, 'sub, T, E>,
        OE1: Observable<'or, 'sub, T, E>,
        F: FnMut(E) -> RetryAction<E, OE1>,
    {
        Self { source, callback }
    }
}

impl<'or, 'sub, T, E, OE, OE1, F> Observable<'or, 'sub, T, E> for Retry<OE, F>
where
    OE: Observable<'or, 'sub, T, E>,
    OE1: Observable<'or, 'sub, T, E>,
    F: FnMut(E) -> RetryAction<E, OE1> + NecessarySend + 'or,
    'sub: 'or,
{
    fn subscribe(self, observer: impl Observer<T, E> + NecessarySend + 'or) -> Subscription<'sub> {
        let sub = Shared::new(Mutable::new(None));
        let observer = RetryObserver {
            observer,
            callback: self.callback,
            sub: sub.clone(),
        };
        self.source.subscribe(observer) + sub
    }
}

struct RetryObserver<'sub, OR, F> {
    observer: OR,
    callback: F,
    sub: Shared<Mutable<Option<Subscription<'sub>>>>,
}

impl<'or, 'sub, T, E, OR, OE1, F> Observer<T, E> for RetryObserver<'sub, OR, F>
where
    OR: Observer<T, E> + NecessarySend + 'or,
    OE1: Observable<'or, 'sub, T, E>,
    F: FnMut(E) -> RetryAction<E, OE1> + NecessarySend + 'or,
    'sub: 'or,
{
    fn on_next(&mut self, value: T) {
        self.observer.on_next(value);
    }

    fn on_termination(mut self, termination: Termination<E>) {
        match termination {
            Termination::Completed => self.observer.on_termination(Termination::Completed),
            Termination::Error(error) => {
                let action = (self.callback)(error);
                match action {
                    RetryAction::Retry(observable) => {
                        let sub = self.sub.clone();
                        safe_lock_option!(replace: sub, observable.subscribe(self));
                    }
                    RetryAction::Stop(error) => {
                        self.observer.on_termination(Termination::Error(error))
                    }
                }
            }
        }
    }
}
