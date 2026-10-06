//! The [`TimeInterval`] operator, behind
//! [`ObservableExt::time_interval`](crate::observable::ObservableExt::time_interval).

use crate::{
    observable::Subscription,
    observable::{Observable, ObservableTypes},
    observer::{Flow, Observer, Termination},
};
use educe::Educe;
use std::time::{Duration, Instant};

/// Emits each item of the source Observable with the time elapsed since the previous one, or,
/// for the first, since the subscription.
/// See <https://reactivex.io/documentation/operators/timeinterval.html>
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt,
///     observer::{Observer, Termination},
///     operators::utility::time_interval::TimeInterval,
///     subject::publish_subject::PublishSubject,
/// };
/// use std::{convert::Infallible, thread::sleep, time::Duration};
///
/// let mut intervals = Vec::new();
/// let mut terminations = Vec::new();
/// let mut subject: PublishSubject<'_, i32, Infallible, rx_rust::thread_mode::Local> = PublishSubject::local();
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
    /// Creates a [`TimeInterval`] over `source`;
    /// [`ObservableExt::time_interval`](crate::observable::ObservableExt::time_interval) is the
    /// fluent form.
    pub fn new(source: OE) -> Self {
        Self { source }
    }
}

impl<T, E, OE> ObservableTypes for TimeInterval<OE>
where
    OE: ObservableTypes<Item = T, Error = E>,
{
    type Item = (T, Duration);
    type Error = E;
    type Mode = OE::Mode;
    type Disposal = OE::Disposal;
}

impl<T, E, OE, OR> Observable<OR> for TimeInterval<OE>
where
    OR: Observer<(T, Duration), E>,
    OE: Observable<TimeIntervalObserver<OR>, Item = T, Error = E>,
{
    fn subscribe(self, observer: OR) -> Subscription<Self::Disposal> {
        let observer = TimeIntervalObserver {
            observer,
            time_stamp: Instant::now(),
        };
        self.source.subscribe(observer)
    }
}

pub struct TimeIntervalObserver<OR> {
    observer: OR,
    time_stamp: Instant,
}

impl<T, E, OR> Observer<T, E> for TimeIntervalObserver<OR>
where
    OR: Observer<(T, Duration), E>,
{
    fn on_next(&mut self, value: T) -> Flow {
        let now = Instant::now();
        let time_span = now.saturating_duration_since(self.time_stamp);
        self.time_stamp = now;
        self.observer.on_next((value, time_span))
    }

    fn on_termination(self, termination: Termination<E>) {
        self.observer.on_termination(termination);
    }
}
