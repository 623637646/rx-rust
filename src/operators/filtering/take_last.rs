//! The [`TakeLast`] operator, behind
//! [`ObservableExt::take_last`](crate::observable::ObservableExt::take_last).

use crate::{
    disposable::dispose_on_drop::DisposeOnDrop,
    observable::{Observable, ObservableTypes},
    observer::{Flow, Observer, Termination},
};
use educe::Educe;
use std::collections::VecDeque;

/// Emits only the last N items emitted by an Observable.
/// See <https://reactivex.io/documentation/operators/takelast.html>
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt,
///     observer::Termination,
///     operators::{
///         creating::from_iter::FromIter,
///         filtering::take_last::TakeLast,
///     },
/// };
///
/// let mut values = Vec::new();
/// let mut terminations = Vec::new();
///
/// let observable = TakeLast::new(FromIter::new(vec![1, 2, 3, 4]), 2);
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
pub struct TakeLast<OE> {
    source: OE,
    count: usize,
}

impl<OE> TakeLast<OE> {
    /// Creates a [`TakeLast`] over `source`;
    /// [`ObservableExt::take_last`](crate::observable::ObservableExt::take_last) is the fluent
    /// form.
    pub fn new(source: OE, count: usize) -> Self {
        Self { source, count }
    }
}

impl<T, E, OE> ObservableTypes for TakeLast<OE>
where
    OE: ObservableTypes<Item = T, Error = E>,
{
    type Item = T;
    type Error = E;
    type Mode = OE::Mode;
    type Disposal = OE::Disposal;
}

impl<T, E, OE, OR> Observable<OR> for TakeLast<OE>
where
    OR: Observer<T, E>,
    OE: Observable<TakeLastObserver<T, OR>, Item = T, Error = E>,
{
    fn subscribe(self, observer: OR) -> DisposeOnDrop<Self::Disposal> {
        self.source.subscribe(TakeLastObserver {
            observer,
            buffer: VecDeque::default(),
            count: self.count,
        })
    }
}

pub struct TakeLastObserver<T, OR> {
    observer: OR,
    buffer: VecDeque<T>,
    count: usize,
}

impl<T, E, OR> Observer<T, E> for TakeLastObserver<T, OR>
where
    OR: Observer<T, E>,
{
    fn on_next(&mut self, value: T) -> Flow {
        // The buffer is only replayed on completion, so the source must run to its end even when
        // nothing is kept: stopping it here would leave downstream without its termination.
        if self.count == 0 {
            return Flow::Continue;
        }
        self.buffer.push_back(value);
        if self.buffer.len() > self.count {
            self.buffer.pop_front();
        }
        Flow::Continue
    }

    fn on_termination(mut self, termination: Termination<E>) {
        if matches!(termination, Termination::Completed) {
            for value in std::mem::take(&mut self.buffer) {
                if self.observer.on_next(value).is_stop() {
                    // The replay ended the stream downstream, so it is not completed on top of
                    // that: it has already ended itself.
                    return;
                }
            }
        }
        self.observer.on_termination(termination);
    }
}
