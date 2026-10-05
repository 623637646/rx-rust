//! The [`BufferWithCount`] operator, behind
//! [`ObservableExt::buffer_with_count`](crate::observable::ObservableExt::buffer_with_count).

use crate::{
    observable::Subscription,
    observable::{Observable, ObservableTypes},
    observer::{Flow, Observer, Termination},
};
use educe::Educe;
use std::num::NonZeroUsize;

/// Periodically gathers items from an Observable into bundles and emits these bundles as `Vec<T>`, when the bundle reaches a specified size.
/// See <https://reactivex.io/documentation/operators/buffer.html>
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt,
///     observer::Termination,
///     operators::{
///         creating::from_iter::FromIter,
///         transforming::buffer_with_count::BufferWithCount,
///     },
/// };
/// use std::num::NonZeroUsize;
///
/// let mut values = Vec::new();
/// let mut terminations = Vec::new();
///
/// let observable = BufferWithCount::new(FromIter::new(vec![1, 2, 3, 4]), NonZeroUsize::new(3).unwrap());
/// observable.subscribe_with_callback(
///     |value| values.push(value),
///     |termination| terminations.push(termination),
/// );
///
/// assert_eq!(values, vec![vec![1, 2, 3], vec![4]]);
/// assert_eq!(terminations, vec![Termination::Completed]);
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct BufferWithCount<OE> {
    source: OE,
    count: NonZeroUsize,
}

impl<OE> BufferWithCount<OE> {
    /// Creates a [`BufferWithCount`] over `source`;
    /// [`ObservableExt::buffer_with_count`](crate::observable::ObservableExt::buffer_with_count) is the fluent form.
    pub fn new(source: OE, count: NonZeroUsize) -> Self {
        Self { source, count }
    }
}

impl<T, E, OE> ObservableTypes for BufferWithCount<OE>
where
    OE: ObservableTypes<Item = T, Error = E>,
{
    type Item = Vec<T>;
    type Error = E;
    type Mode = OE::Mode;
    type D = OE::D;
}

impl<T, E, OE, OR> Observable<OR> for BufferWithCount<OE>
where
    OR: Observer<Vec<T>, E>,
    OE: Observable<BufferWithCountObserver<T, OR>, Item = T, Error = E>,
{
    fn subscribe(self, observer: OR) -> Subscription<Self::D> {
        let observer = BufferWithCountObserver {
            observer,
            values: Vec::default(),
            count: self.count,
        };
        self.source.subscribe(observer)
    }
}

pub struct BufferWithCountObserver<T, OR> {
    observer: OR,
    values: Vec<T>,
    count: NonZeroUsize,
}

impl<T, E, OR> Observer<T, E> for BufferWithCountObserver<T, OR>
where
    OR: Observer<Vec<T>, E>,
{
    fn on_next(&mut self, value: T) -> Flow {
        self.values.push(value);
        if self.values.len() >= self.count.get() {
            self.observer.on_next(std::mem::take(&mut self.values))
        } else {
            Flow::Continue
        }
    }

    fn on_termination(mut self, termination: Termination<E>) {
        // The final value ends the stream, so a downstream that stopped on it is not completed
        // on top of that: it has already ended itself.
        if matches!(termination, Termination::Completed)
            && !self.values.is_empty()
            && self
                .observer
                .on_next(std::mem::take(&mut self.values))
                .is_stop()
        {
            return;
        }
        self.observer.on_termination(termination);
    }
}
