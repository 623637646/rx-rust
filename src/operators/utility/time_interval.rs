//! The [`TimeInterval`] operator, behind
//! [`ObservableExt::time_interval`](crate::observable::ObservableExt::time_interval).

use crate::{
    disposable::dispose_on_drop::DisposeOnDrop,
    observable::{Observable, ObservableTypes},
    observer::{Flow, Observer, Termination},
    scheduler::Clock,
};
use educe::Educe;
use std::time::{Duration, Instant};

/// Emits each item of the source observable with the time elapsed since the previous one, or,
/// for the first, since the subscription, measured on a [`Clock`]:
/// [`SystemClock`](crate::scheduler::SystemClock), or a scheduler's
/// ([`SchedulerTypes::clock`](crate::scheduler::SchedulerTypes::clock)).
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
/// let mut subject = PublishSubject::<_, Infallible, _>::local();
///
/// let subscription = TimeInterval::new(subject.clone(), time.clock())
///     .subscribe_with_callback(|interval| intervals.push(interval), |_| {});
///
/// time.advance_by(Duration::from_millis(2));
/// let _ = subject.on_next(1);
/// time.advance_by(Duration::from_millis(5));
/// let _ = subject.on_next(2);
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
pub struct TimeInterval<OE, C> {
    source: OE,
    clock: C,
}

impl<OE, C> TimeInterval<OE, C> {
    /// Creates a [`TimeInterval`] over `source`, on `clock`;
    /// [`ObservableExt::time_interval`](crate::observable::ObservableExt::time_interval) is the
    /// fluent form.
    pub fn new(source: OE, clock: C) -> Self {
        Self { source, clock }
    }
}

impl<T, E, OE, C> ObservableTypes for TimeInterval<OE, C>
where
    OE: ObservableTypes<Item = T, Error = E>,
{
    type Item = (T, Duration);
    type Error = E;
    type Mode = OE::Mode;
    type Disposal = OE::Disposal;
}

impl<T, E, OE, C, OR> Observable<OR> for TimeInterval<OE, C>
where
    OR: Observer<(T, Duration), E>,
    OE: Observable<TimeIntervalObserver<OR, C>, Item = T, Error = E>,
    C: Clock,
{
    fn subscribe(self, observer: OR) -> DisposeOnDrop<Self::Disposal> {
        let observer = TimeIntervalObserver {
            observer,
            previous_timestamp: self.clock.now(),
            clock: self.clock,
        };
        self.source.subscribe(observer)
    }
}

#[doc(hidden)]
pub struct TimeIntervalObserver<OR, C> {
    observer: OR,
    clock: C,
    /// When the previous value arrived, or the subscription for the first one.
    previous_timestamp: Instant,
}

impl<T, E, OR, C> Observer<T, E> for TimeIntervalObserver<OR, C>
where
    OR: Observer<(T, Duration), E>,
    C: Clock,
{
    fn on_next(&mut self, value: T) -> Flow {
        let now = self.clock.now();
        let time_span = now.saturating_duration_since(self.previous_timestamp);
        self.previous_timestamp = now;
        self.observer.on_next((value, time_span))
    }

    fn on_termination(self, termination: Termination<E>) {
        self.observer.on_termination(termination);
    }
}
