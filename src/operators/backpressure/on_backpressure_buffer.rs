use crate::{
    disposable::subscription::Subscription,
    observable::Observable,
    observer::Observer,
    operators::backpressure::on_backpressure::{
        BackpressureCollection, OnBackpressure, RequestCallbackType,
    },
    utils::types::NecessarySend,
};
use educe::Educe;

/// Buffers every upstream item into a `Vec<T>` and only delivers the collected batch once
/// the downstream observer calls the accompanying [`RequestCallbackType`]. This mirrors the
/// behavior of `onBackpressureBuffer` from other ReactiveX implementations.
/// See <https://reactivex.io/documentation/operators/backpressure.html>.
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     disposable::Disposable,
///     observable::observable_ext::ObservableExt,
///     observer::Observer,
///     operators::backpressure::on_backpressure_buffer::OnBackpressureBuffer,
///     subject::publish_subject::PublishSubject,
/// };
/// use std::convert::Infallible;
///
/// let mut received = Vec::new();
/// let mut subject = PublishSubject::<_, Infallible>::new();
/// let observable = OnBackpressureBuffer::new(subject.clone());
///
/// let subscription = observable.subscribe_with_callback(
///     |(values, request_callback)| {
///         received.push(values);
///         request_callback(); // immediately allow the next batch
///     },
///     |_| {},
/// );
///
/// subject.on_next(1);
/// subject.on_next(2);
/// subject.on_next(3);
///
/// subscription.dispose();
/// drop(subject);
///
/// assert_eq!(received, vec![vec![1], vec![2], vec![3]]);
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct OnBackpressureBuffer<OE> {
    source: OE,
}

impl<OE> OnBackpressureBuffer<OE> {
    pub fn new<'or, 'sub, T, E>(source: OE) -> Self
    where
        OE: Observable<'or, 'sub, T, E>,
    {
        Self { source }
    }
}

impl<'or, 'sub, T, E, OE> Observable<'or, 'sub, (Vec<T>, RequestCallbackType<'or>), E>
    for OnBackpressureBuffer<OE>
where
    'or: 'sub,
    T: NecessarySend + 'or,
    E: NecessarySend + 'or,
    OE: Observable<'or, 'sub, T, E>,
{
    fn subscribe(
        self,
        observer: impl Observer<(Vec<T>, RequestCallbackType<'or>), E> + NecessarySend + 'or,
    ) -> Subscription<'sub> {
        OnBackpressure::new(self.source, Collection(Vec::new())).subscribe(observer)
    }
}

struct Collection<T>(Vec<T>);

impl<T> BackpressureCollection<T, Vec<T>> for Collection<T> {
    fn extend_one(&mut self, item: T) {
        self.0.push(item);
    }

    fn take_next_value(&mut self) -> Option<Vec<T>> {
        if self.0.is_empty() {
            None
        } else {
            Some(std::mem::take(&mut self.0))
        }
    }
}
