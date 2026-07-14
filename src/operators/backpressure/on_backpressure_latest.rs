use crate::{
    observable::{Observable, Subscription},
    observer::Observer,
    operators::backpressure::on_backpressure::{
        BackpressureCollection, OnBackpressure, RequestToken,
    },
    utils::{subscribe_with_shared_model, types::MaybeSend},
};
use educe::Educe;

/// Keeps only the most recent upstream item while the downstream observer is still processing
/// the previous one. Once [`RequestToken::request`] is called, the latest buffered value is
/// emitted and the cycle repeats. This mirrors `onBackpressureLatest` in other ReactiveX stacks.
/// See <https://reactivex.io/documentation/operators/backpressure.html>.
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     disposable::Disposable,
///     observable::ObservableExt,
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
///         request_callback.request(); // immediately accept the next value
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
    pub fn new<'or, T, E>(source: OE) -> Self
    where
        OE: Observable<'or, T = T, E = E>,
    {
        Self { source }
    }
}

impl<'or, T, E, OE> Observable<'or> for OnBackpressureLatest<OE>
where
    T: MaybeSend + 'or,
    E: MaybeSend + 'or,
    OE: Observable<'or, T = T, E = E>,
    OE::D: MaybeSend + 'or,
{
    type T = (T, RequestToken<'or>);
    type E = E;
    type D = subscribe_with_shared_model::Disposal<'or>;

    fn subscribe(
        self,
        observer: impl Observer<(T, RequestToken<'or>), E> + MaybeSend + 'or,
    ) -> Subscription<Self::D> {
        OnBackpressure::new(self.source, Collection(None)).subscribe(observer)
    }
}

struct Collection<T>(Option<T>);

impl<T> BackpressureCollection for Collection<T> {
    type Input = T;
    type Output = T;

    fn extend_one(&mut self, item: Self::Input) {
        self.0 = Some(item);
    }

    fn take_next_value(&mut self) -> Option<Self::Output> {
        self.0.take()
    }
}
