//! The [`Skip`] operator, behind [`ObservableExt::skip`](crate::observable::ObservableExt::skip).

use crate::{
    observable::Subscription,
    observable::{Observable, ObservableTypes},
    observer::{Flow, Observer, Termination},
};
use educe::Educe;

/// Suppresses the first N items emitted by an Observable.
/// See <https://reactivex.io/documentation/operators/skip.html>
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt,
///     observer::Termination,
///     operators::{
///         creating::from_iter::FromIter,
///         filtering::skip::Skip,
///     },
/// };
///
/// let mut values = Vec::new();
/// let mut terminations = Vec::new();
///
/// let observable = Skip::new(FromIter::new(vec![1, 2, 3, 4]), 2);
/// observable.subscribe_with_callback(
///     |value| values.push(value),
///     |termination| terminations.push(termination),
/// );
///
/// assert_eq!(values, vec![3, 4]);
/// assert_eq!(terminations, vec![Termination::Completed]);
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct Skip<OE> {
    source: OE,
    count: usize,
}

impl<OE> Skip<OE> {
    /// Creates a [`Skip`] over `source`;
    /// [`ObservableExt::skip`](crate::observable::ObservableExt::skip) is the fluent form.
    pub fn new(source: OE, count: usize) -> Self {
        Self { source, count }
    }
}

impl<T, E, OE> ObservableTypes for Skip<OE>
where
    OE: ObservableTypes<Item = T, Error = E>,
{
    type Item = T;
    type Error = E;
    type Mode = OE::Mode;
    type D = OE::D;
}

impl<T, E, OE, OR> Observable<OR> for Skip<OE>
where
    OR: Observer<T, E>,
    OE: Observable<SkipObserver<OR>, Item = T, Error = E>,
{
    fn subscribe(self, observer: OR) -> Subscription<Self::D> {
        self.source.subscribe(SkipObserver {
            observer,
            count: self.count,
        })
    }
}

pub struct SkipObserver<OR> {
    observer: OR,
    count: usize,
}

impl<T, E, OR> Observer<T, E> for SkipObserver<OR>
where
    OR: Observer<T, E>,
{
    fn on_next(&mut self, value: T) -> Flow {
        if self.count > 0 {
            self.count -= 1;
            Flow::Continue
        } else {
            self.observer.on_next(value)
        }
    }

    fn on_termination(self, termination: Termination<E>) {
        self.observer.on_termination(termination);
    }
}
