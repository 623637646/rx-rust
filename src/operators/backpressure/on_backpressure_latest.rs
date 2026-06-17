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

/// Keeps only the most recent upstream item while the downstream observer is still processing
/// the previous one. Once the [`RequestCallbackType`] is invoked, the latest buffered value is
/// emitted and the cycle repeats. This mirrors `onBackpressureLatest` in other ReactiveX stacks.
/// See <https://reactivex.io/documentation/operators/backpressure.html>.
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     disposable::Disposable,
///     observable::observable_ext::ObservableExt,
///     observer::Observer,
///     operators::backpressure::on_backpressure_latest::OnBackpressureLatest,
///     subject::publish_subject::PublishSubject,
/// };
/// use std::convert::Infallible;
///
/// let mut received = Vec::new();
/// let mut subject = PublishSubject::<_, Infallible>::new();
/// let observable = OnBackpressureLatest::new(subject.clone());
///
/// let subscription = observable.subscribe_with_callback(
///     |(value, request_callback)| {
///         received.push(value);
///         request_callback(); // immediately accept the next value
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
/// assert_eq!(received, vec![1, 2, 3]);
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct OnBackpressureLatest<OE> {
    source: OE,
}

impl<OE> OnBackpressureLatest<OE> {
    pub fn new<'or, 'sub, T, E>(source: OE) -> Self
    where
        OE: Observable<'or, 'sub, T, E>,
    {
        Self { source }
    }
}

impl<'or, 'sub, T, E, OE> Observable<'or, 'sub, (T, RequestCallbackType<'or>), E>
    for OnBackpressureLatest<OE>
where
    'or: 'sub,
    T: NecessarySend + 'or,
    E: NecessarySend + 'or,
    OE: Observable<'or, 'sub, T, E>,
{
    fn subscribe(
        self,
        observer: impl Observer<(T, RequestCallbackType<'or>), E> + NecessarySend + 'or,
    ) -> Subscription<'sub> {
        OnBackpressure::new(self.source, Collection(None)).subscribe(observer)
    }
}

struct Collection<T>(Option<T>);

impl<T> BackpressureCollection<T, T> for Collection<T> {
    fn extend_one(&mut self, item: T) {
        self.0 = Some(item);
    }

    fn take_next_value(&mut self) -> Option<T> {
        self.0.take()
    }
}
