use crate::utils::types::MaybeSend;
use crate::{
    observable::Observable,
    observable::Subscription,
    observer::{Flow, Observer, Termination},
};
use educe::Educe;
use std::collections::VecDeque;

/// Suppresses the last N items emitted by an Observable.
/// See <https://reactivex.io/documentation/operators/skiplast.html>
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt,
///     observer::Termination,
///     operators::{
///         creating::from_iter::FromIter,
///         filtering::skip_last::SkipLast,
///     },
/// };
///
/// let mut values = Vec::new();
/// let mut terminations = Vec::new();
///
/// let observable = SkipLast::new(FromIter::new(vec![1, 2, 3, 4]), 1);
/// observable.subscribe_with_callback(
///     |value| values.push(value),
///     |termination| terminations.push(termination),
/// );
///
/// assert_eq!(values, vec![1, 2, 3]);
/// assert_eq!(terminations, vec![Termination::Completed]);
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct SkipLast<OE> {
    source: OE,
    count: usize,
}

impl<OE> SkipLast<OE> {
    pub fn new(source: OE, count: usize) -> Self {
        Self { source, count }
    }
}

impl<'or, T, E, OE> Observable<'or, T, E> for SkipLast<OE>
where
    T: MaybeSend + 'or,
    OE: Observable<'or, T, E>,
{
    type D = OE::D;

    fn subscribe(self, observer: impl Observer<T, E> + MaybeSend + 'or) -> Subscription<Self::D> {
        self.source.subscribe(SkipLastObserver {
            observer,
            count: self.count,
            buffer: VecDeque::new(),
        })
    }
}

struct SkipLastObserver<T, OR> {
    observer: OR,
    count: usize,
    buffer: VecDeque<T>,
}

impl<T, E, OR> Observer<T, E> for SkipLastObserver<T, OR>
where
    OR: Observer<T, E>,
{
    fn on_next(&mut self, value: T) -> Flow {
        self.buffer.push_back(value);
        if self.buffer.len() > self.count {
            self.observer.on_next(self.buffer.pop_front().unwrap())
        } else {
            Flow::Continue
        }
    }

    fn on_termination(self, termination: Termination<E>) {
        self.observer.on_termination(termination);
    }
}
