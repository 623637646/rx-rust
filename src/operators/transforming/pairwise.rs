//! The [`Pairwise`] operator, behind
//! [`ObservableExt::pairwise`](crate::observable::ObservableExt::pairwise).

use crate::{
    disposable::dispose_on_drop::DisposeOnDrop,
    observable::{Observable, ObservableTypes},
    observer::{Flow, Observer, Termination},
};
use educe::Educe;

/// Emits each item together with the one before it, as a `(previous, current)` pair.
/// See <https://reactivex.io/documentation/operators/buffer.html>
///
/// The first item has no predecessor, so it is only kept, and a source of fewer than two items
/// emits nothing. Each item after the first is cloned once: one copy is emitted as the current
/// item, and the other is kept for the next pair. The first item is kept without cloning.
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt,
///     observer::Termination,
///     operators::{creating::from_iter::FromIter, transforming::pairwise::Pairwise},
/// };
///
/// let mut values = Vec::new();
///
/// let observable = Pairwise::new(FromIter::new([1, 2, 3]));
/// observable.subscribe_with_callback(
///     |value| values.push(value),
///     |termination| assert_eq!(termination, Termination::Completed),
/// );
///
/// assert_eq!(values, vec![(1, 2), (2, 3)]);
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct Pairwise<OE> {
    source: OE,
}

impl<OE> Pairwise<OE> {
    /// Creates a [`Pairwise`] over `source`;
    /// [`ObservableExt::pairwise`](crate::observable::ObservableExt::pairwise) is the fluent form.
    pub fn new(source: OE) -> Self {
        Self { source }
    }
}

impl<T, E, OE> ObservableTypes for Pairwise<OE>
where
    OE: ObservableTypes<Item = T, Error = E>,
{
    type Item = (T, T);
    type Error = E;
    type Mode = OE::Mode;
    type Disposal = OE::Disposal;
}

impl<T, E, OE, OR> Observable<OR> for Pairwise<OE>
where
    OR: Observer<(T, T), E>,
    OE: Observable<PairwiseObserver<T, OR>, Item = T, Error = E>,
    T: Clone,
{
    fn subscribe(self, observer: OR) -> DisposeOnDrop<Self::Disposal> {
        let observer = PairwiseObserver {
            observer,
            previous: None,
        };
        self.source.subscribe(observer)
    }
}

#[doc(hidden)]
pub struct PairwiseObserver<T, OR> {
    observer: OR,
    previous: Option<T>,
}

impl<T, E, OR> Observer<T, E> for PairwiseObserver<T, OR>
where
    OR: Observer<(T, T), E>,
    T: Clone,
{
    fn on_next(&mut self, value: T) -> Flow {
        match &mut self.previous {
            Some(previous) => {
                let previous = std::mem::replace(previous, value.clone());
                self.observer.on_next((previous, value))
            }
            empty @ None => {
                *empty = Some(value);
                Flow::Continue(())
            }
        }
    }

    fn on_termination(self, termination: Termination<E>) {
        self.observer.on_termination(termination)
    }
}
