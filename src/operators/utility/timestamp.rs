use crate::{
    observable::Observable,
    observable::Subscription,
    observer::{Observer, Termination},
    utils::types::MaybeSend,
};
use educe::Educe;
use std::time::Instant;

/// Attaches a timestamp to each item emitted by an Observable.
/// See <https://reactivex.io/documentation/operators/timestamp.html>
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt,
///     observer::{Observer, Termination},
///     operators::utility::timestamp::Timestamp,
///     subject::publish_subject::PublishSubject,
/// };
/// use std::{convert::Infallible, time::{Duration, Instant}};
///
/// let mut timestamped = Vec::new();
/// let mut terminations = Vec::new();
/// let mut subject: PublishSubject<'_, i32, Infallible> = PublishSubject::default();
/// let start = Instant::now();
///
/// let subscription = Timestamp::new(subject.clone()).subscribe_with_callback(
///     |(value, instant)| timestamped.push((value, instant)),
///     |termination| terminations.push(termination),
/// );
///
/// subject.on_next(1);
/// subject.on_next(2);
/// subject.on_termination(Termination::Completed);
/// drop(subscription);
///
/// assert_eq!(
///     timestamped.iter().map(|(value, _)| *value).collect::<Vec<_>>(),
///     vec![1, 2]
/// );
/// assert!(timestamped[0].1.duration_since(start) >= Duration::from_millis(0));
/// assert!(timestamped[1].1.duration_since(timestamped[0].1) >= Duration::from_millis(0));
/// assert_eq!(terminations, vec![Termination::Completed]);
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct Timestamp<OE> {
    source: OE,
}

impl<OE> Timestamp<OE> {
    pub fn new(source: OE) -> Self {
        Self { source }
    }
}

impl<'or, T, E, OE> Observable<'or, (T, Instant), E> for Timestamp<OE>
where
    OE: Observable<'or, T, E>,
{
    type D = OE::D;

    fn subscribe(
        self,
        observer: impl Observer<(T, Instant), E> + MaybeSend + 'or,
    ) -> Subscription<Self::D> {
        let observer = TimestampObserver { observer };
        self.source.subscribe(observer)
    }
}

struct TimestampObserver<OR> {
    observer: OR,
}

impl<T, E, OR> Observer<T, E> for TimestampObserver<OR>
where
    OR: Observer<(T, Instant), E>,
{
    fn on_next(&mut self, value: T) {
        self.observer.on_next((value, Instant::now()));
    }

    fn on_termination(self, termination: Termination<E>) {
        self.observer.on_termination(termination);
    }
}
