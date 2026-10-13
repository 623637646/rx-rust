//! The [`Throttle`] operator, behind
//! [`ObservableExt::throttle`](crate::observable::ObservableExt::throttle).

use crate::disposable::dispose_on_drop::DisposeOnDrop;
use crate::{
    observable::{Observable, ObservableTypes},
    observer::{Flow, Observer, Termination},
    scheduler::Clock,
};
use educe::Educe;
use std::time::{Duration, Instant};

/// Emits an item from the source Observable, then ignores subsequent items for a particular time
/// span.
/// See <https://reactivex.io/documentation/operators/sample.html> (`throttleFirst`).
///
/// This is a synchronous, leading-edge throttle: it compares the arrival time of each item, on
/// `clock`, against the last emission. It schedules nothing, so it takes a [`Clock`] rather than a
/// scheduler — [`SystemClock`](crate::scheduler::SystemClock), or a scheduler's
/// ([`SchedulerTypes::clock`](crate::scheduler::SchedulerTypes::clock)) — and there is no timer to
/// spawn, cancel, or drift.
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
/// let subscription = Throttle::new(subject.clone(), Duration::from_millis(5), time.clock())
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
pub struct Throttle<OE, C> {
    source: OE,
    time_span: Duration,
    clock: C,
}

impl<OE, C> Throttle<OE, C> {
    /// Creates a [`Throttle`] over `source`, timed by `clock`;
    /// [`ObservableExt::throttle`](crate::observable::ObservableExt::throttle) is the fluent form.
    pub fn new(source: OE, time_span: Duration, clock: C) -> Self {
        Self {
            source,
            time_span,
            clock,
        }
    }
}

impl<T, E, OE, C> ObservableTypes for Throttle<OE, C>
where
    OE: ObservableTypes<Item = T, Error = E>,
{
    type Item = T;
    type Error = E;
    type Mode = OE::Mode;
    type Disposal = OE::Disposal;
}

impl<T, E, OE, C, OR> Observable<OR> for Throttle<OE, C>
where
    OR: Observer<T, E>,
    OE: Observable<ThrottleObserver<OR, C>, Item = T, Error = E>,
    C: Clock,
{
    fn subscribe(self, observer: OR) -> DisposeOnDrop<Self::Disposal> {
        self.source.subscribe(ThrottleObserver {
            observer,
            time_span: self.time_span,
            clock: self.clock,
            last_emit: None,
        })
    }
}

pub struct ThrottleObserver<OR, C> {
    observer: OR,
    time_span: Duration,
    clock: C,
    last_emit: Option<Instant>,
}

impl<T, E, OR, C> Observer<T, E> for ThrottleObserver<OR, C>
where
    OR: Observer<T, E>,
    C: Clock,
{
    fn on_next(&mut self, value: T) -> Flow {
        let now = self.clock.now();
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
