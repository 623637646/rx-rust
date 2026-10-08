//! The [`Throttle`] operator, behind
//! [`ObservableExt::throttle`](crate::observable::ObservableExt::throttle).

use crate::observable::Subscription;
use crate::{
    observable::{Observable, ObservableTypes},
    observer::{Flow, Observer, Termination},
    scheduler::SchedulerTypes,
};
use educe::Educe;
use std::time::{Duration, Instant};

/// Emits an item from the source Observable, then ignores subsequent items for a particular time
/// span.
/// See <https://reactivex.io/documentation/operators/sample.html> (`throttleFirst`).
///
/// This is a synchronous, leading-edge throttle: it compares the arrival time of each item, on the
/// clock of `scheduler` ([`SchedulerTypes::now`]), against the last emission. It runs nothing on
/// the scheduler, so there is no timer to spawn, cancel, or drift.
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt,
///     observer::{Observer, Termination},
///     operators::filtering::throttle::Throttle,
///     scheduler::virtual_time::VirtualTime,
///     subject::publish_subject::PublishSubject,
/// };
/// use std::{convert::Infallible, time::Duration};
///
/// let time = VirtualTime::new();
/// let mut values = Vec::new();
/// let mut subject: PublishSubject<'_, i32, Infallible, rx_rust::thread_mode::Local> =
///     PublishSubject::local();
///
/// let subscription = Throttle::new(subject.clone(), Duration::from_millis(5), time.scheduler())
///     .subscribe_with_callback(|value| values.push(value), |_| {});
///
/// subject.on_next(1);
/// subject.on_next(2); // within 5 ms of `1`: dropped
/// time.advance_by(Duration::from_millis(5));
/// subject.on_next(3);
/// subject.on_termination(Termination::Completed);
/// drop(subscription);
///
/// assert_eq!(values, [1, 3]);
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct Throttle<OE, S> {
    source: OE,
    time_span: Duration,
    scheduler: S,
}

impl<OE, S> Throttle<OE, S> {
    /// Creates a [`Throttle`] over `source`, timed by the clock of `scheduler`;
    /// [`ObservableExt::throttle`](crate::observable::ObservableExt::throttle) is the fluent form.
    pub fn new(source: OE, time_span: Duration, scheduler: S) -> Self {
        Self {
            source,
            time_span,
            scheduler,
        }
    }
}

impl<T, E, OE, S> ObservableTypes for Throttle<OE, S>
where
    OE: ObservableTypes<Item = T, Error = E>,
{
    type Item = T;
    type Error = E;
    type Mode = OE::Mode;
    type Disposal = OE::Disposal;
}

impl<T, E, OE, S, OR> Observable<OR> for Throttle<OE, S>
where
    OR: Observer<T, E>,
    OE: Observable<ThrottleObserver<OR, S>, Item = T, Error = E>,
    S: SchedulerTypes,
{
    fn subscribe(self, observer: OR) -> Subscription<Self::Disposal> {
        self.source.subscribe(ThrottleObserver {
            observer,
            time_span: self.time_span,
            scheduler: self.scheduler,
            last_emit: None,
        })
    }
}

pub struct ThrottleObserver<OR, S> {
    observer: OR,
    time_span: Duration,
    scheduler: S,
    last_emit: Option<Instant>,
}

impl<T, E, OR, S> Observer<T, E> for ThrottleObserver<OR, S>
where
    OR: Observer<T, E>,
    S: SchedulerTypes,
{
    fn on_next(&mut self, value: T) -> Flow {
        let now = self.scheduler.now();
        let should_emit = match self.last_emit {
            None => true,
            Some(last) => now.duration_since(last) >= self.time_span,
        };
        if should_emit {
            self.last_emit = Some(now);
            self.observer.on_next(value)
        } else {
            Flow::Continue
        }
    }

    fn on_termination(self, termination: Termination<E>) {
        self.observer.on_termination(termination);
    }
}
