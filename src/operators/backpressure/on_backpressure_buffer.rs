use crate::{
    observable::{Observable, Subscription},
    observer::Observer,
    operators::backpressure::on_backpressure::{
        BackpressureCollection, OnBackpressure, RequestToken,
    },
    utils::{subscribe_with_shared_model, types::MaybeSend},
};
use educe::Educe;

/// Buffers every upstream item into a `Vec<T>` and only delivers the collected batch once
/// the downstream observer requests another batch with the accompanying [`RequestToken`]. This mirrors the
/// behavior of `onBackpressureBuffer` from other ReactiveX implementations.
/// See <https://reactivex.io/documentation/operators/backpressure.html>.
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     disposable::Disposable,
///     observable::ObservableExt,
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
///         request_callback.request(); // immediately allow the next batch
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
    pub fn new<'or, T, E>(source: OE) -> Self
    where
        OE: Observable<'or, T = T, E = E>,
    {
        Self { source }
    }
}

impl<'or, T, E, OE> Observable<'or> for OnBackpressureBuffer<OE>
where
    T: MaybeSend + 'or,
    E: MaybeSend + 'or,
    OE: Observable<'or, T = T, E = E>,
    OE::D: MaybeSend + 'or,
{
    type T = (Vec<T>, RequestToken<'or>);
    type E = E;
    type D = subscribe_with_shared_model::Disposal<'or>;

    fn subscribe(
        self,
        observer: impl Observer<(Vec<T>, RequestToken<'or>), E> + MaybeSend + 'or,
    ) -> Subscription<Self::D> {
        OnBackpressure::new(self.source, Collection(Vec::new())).subscribe(observer)
    }
}

struct Collection<T>(Vec<T>);

impl<T> BackpressureCollection for Collection<T> {
    type Input = T;
    type Output = Vec<T>;

    fn extend_one(&mut self, item: Self::Input) {
        self.0.push(item);
    }

    fn take_next_value(&mut self) -> Option<Self::Output> {
        if self.0.is_empty() {
            None
        } else {
            Some(std::mem::take(&mut self.0))
        }
    }
}
