use crate::{
    disposable::subscription::Subscription,
    observable::Observable,
    observer::{Observer, Termination},
    utils::types::MaybeSend,
};
use educe::Educe;
use std::time::{Duration, Instant};

/// Emits the time elapsed between consecutive emissions from the source Observable.
/// See <https://reactivex.io/documentation/operators/timeinterval.html>
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::observable_ext::ObservableExt,
///     observer::{Observer, Termination},
///     operators::utility::time_interval::TimeInterval,
///     subject::publish_subject::PublishSubject,
/// };
/// use std::{convert::Infallible, thread::sleep, time::Duration};
///
/// let mut intervals = Vec::new();
/// let mut terminations = Vec::new();
/// let mut subject: PublishSubject<'_, i32, Infallible> = PublishSubject::default();
///
/// let subscription = TimeInterval::new(subject.clone()).subscribe_with_callback(
///     |(value, span)| intervals.push((value, span)),
///     |termination| terminations.push(termination),
/// );
///
/// subject.on_next(1);
/// sleep(Duration::from_millis(5));
/// subject.on_next(2);
/// subject.on_termination(Termination::Completed);
/// drop(subscription);
///
/// assert_eq!(intervals.len(), 2);
/// assert_eq!(intervals[0].0, 1);
/// assert_eq!(intervals[1].0, 2);
/// assert!(intervals[1].1 >= Duration::from_millis(5));
/// assert_eq!(terminations, vec![Termination::Completed]);
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct TimeInterval<OE> {
    source: OE,
}

impl<OE> TimeInterval<OE> {
    pub fn new(source: OE) -> Self {
        Self { source }
    }
}

impl<'or, 'sub, T, E, OE> Observable<'or, 'sub, (T, Duration), E> for TimeInterval<OE>
where
    OE: Observable<'or, 'sub, T, E>,
{
    fn subscribe(
        self,
        observer: impl Observer<(T, Duration), E> + MaybeSend + 'or,
    ) -> Subscription<'sub> {
        let observer = TimeIntervalObserver {
            observer,
            time_stamp: Instant::now(),
        };
        self.source.subscribe(observer)
    }
}

struct TimeIntervalObserver<OR> {
    observer: OR,
    time_stamp: Instant,
}

impl<T, E, OR> Observer<T, E> for TimeIntervalObserver<OR>
where
    OR: Observer<(T, Duration), E>,
{
    fn on_next(&mut self, value: T) {
        let time_span = self.time_stamp.elapsed();
        self.time_stamp = Instant::now();
        self.observer.on_next((value, time_span));
    }

    fn on_termination(self, termination: Termination<E>) {
        self.observer.on_termination(termination);
    }
}
