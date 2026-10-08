//! The [`TimeInterval`] operator, behind
//! [`ObservableExt::time_interval`](crate::observable::ObservableExt::time_interval).

use crate::{
    observable::Subscription,
    observable::{Observable, ObservableTypes},
    observer::{Flow, Observer, Termination},
    scheduler::SchedulerTypes,
};
use educe::Educe;
use std::time::{Duration, Instant};

/// Emits each item of the source Observable with the time elapsed since the previous one, or,
/// for the first, since the subscription, measured on the clock of a scheduler
/// ([`SchedulerTypes::now`]).
/// See <https://reactivex.io/documentation/operators/timeinterval.html>
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt,
///     observer::{Observer, Termination},
///     operators::utility::time_interval::TimeInterval,
///     scheduler::virtual_time::VirtualTime,
///     subject::publish_subject::PublishSubject,
/// };
/// use std::{convert::Infallible, time::Duration};
///
/// let time = VirtualTime::new();
/// let mut intervals = Vec::new();
/// let mut subject: PublishSubject<'_, i32, Infallible, rx_rust::thread_mode::Local> =
///     PublishSubject::local();
///
/// let subscription = TimeInterval::new(subject.clone(), time.scheduler())
///     .subscribe_with_callback(|interval| intervals.push(interval), |_| {});
///
/// time.advance_by(Duration::from_millis(2));
/// subject.on_next(1);
/// time.advance_by(Duration::from_millis(5));
/// subject.on_next(2);
/// subject.on_termination(Termination::Completed);
/// drop(subscription);
///
/// assert_eq!(
///     intervals,
///     [(1, Duration::from_millis(2)), (2, Duration::from_millis(5))]
/// );
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct TimeInterval<OE, S> {
    source: OE,
    scheduler: S,
}

impl<OE, S> TimeInterval<OE, S> {
    /// Creates a [`TimeInterval`] over `source`, on the clock of `scheduler`;
    /// [`ObservableExt::time_interval`](crate::observable::ObservableExt::time_interval) is the
    /// fluent form.
    pub fn new(source: OE, scheduler: S) -> Self {
        Self { source, scheduler }
    }
}

impl<T, E, OE, S> ObservableTypes for TimeInterval<OE, S>
where
    OE: ObservableTypes<Item = T, Error = E>,
{
    type Item = (T, Duration);
    type Error = E;
    type Mode = OE::Mode;
    type Disposal = OE::Disposal;
}

impl<T, E, OE, S, OR> Observable<OR> for TimeInterval<OE, S>
where
    OR: Observer<(T, Duration), E>,
    OE: Observable<TimeIntervalObserver<OR, S>, Item = T, Error = E>,
    S: SchedulerTypes,
{
    fn subscribe(self, observer: OR) -> Subscription<Self::Disposal> {
        let observer = TimeIntervalObserver {
            observer,
            time_stamp: self.scheduler.now(),
            scheduler: self.scheduler,
        };
        self.source.subscribe(observer)
    }
}

pub struct TimeIntervalObserver<OR, S> {
    observer: OR,
    scheduler: S,
    time_stamp: Instant,
}

impl<T, E, OR, S> Observer<T, E> for TimeIntervalObserver<OR, S>
where
    OR: Observer<(T, Duration), E>,
    S: SchedulerTypes,
{
    fn on_next(&mut self, value: T) -> Flow {
        let now = self.scheduler.now();
        let time_span = now.saturating_duration_since(self.time_stamp);
        self.time_stamp = now;
        self.observer.on_next((value, time_span))
    }

    fn on_termination(self, termination: Termination<E>) {
        self.observer.on_termination(termination);
    }
}
