//! The [`Timestamp`] operator, behind
//! [`ObservableExt::timestamp`](crate::observable::ObservableExt::timestamp).

use crate::{
    disposable::dispose_on_drop::DisposeOnDrop,
    observable::{Observable, ObservableTypes},
    observer::{Flow, Observer, Termination},
    scheduler::Clock,
};
use educe::Educe;
use std::time::Instant;

/// Attaches the [`Instant`] it arrived at, on a [`Clock`], to each item emitted by an Observable:
/// [`SystemClock`](crate::scheduler::SystemClock), or a scheduler's
/// ([`SchedulerTypes::clock`](crate::scheduler::SchedulerTypes::clock)).
/// See <https://reactivex.io/documentation/operators/timestamp.html>
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt,
///     observer::{Observer, Termination},
///     operators::utility::timestamp::Timestamp,
///     scheduler::virtual_time::VirtualTime,
///     subject::publish_subject::PublishSubject,
/// };
/// use std::{convert::Infallible, time::Duration};
///
/// let time = VirtualTime::new();
/// let start = time.now();
/// let mut timestamped = Vec::new();
/// let mut subject: PublishSubject<'_, i32, Infallible, rx_rust::thread_mode::Local> =
///     PublishSubject::local();
///
/// let subscription = Timestamp::new(subject.clone(), time.clock())
///     .subscribe_with_callback(|timestamped_value| timestamped.push(timestamped_value), |_| {});
///
/// subject.on_next(1);
/// time.advance_by(Duration::from_millis(5));
/// subject.on_next(2);
/// subject.on_termination(Termination::Completed);
/// drop(subscription);
///
/// assert_eq!(timestamped, [(1, start), (2, start + Duration::from_millis(5))]);
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct Timestamp<OE, C> {
    source: OE,
    clock: C,
}

impl<OE, C> Timestamp<OE, C> {
    /// Creates a [`Timestamp`] over `source`, on `clock`;
    /// [`ObservableExt::timestamp`](crate::observable::ObservableExt::timestamp) is the fluent
    /// form.
    pub fn new(source: OE, clock: C) -> Self {
        Self { source, clock }
    }
}

impl<T, E, OE, C> ObservableTypes for Timestamp<OE, C>
where
    OE: ObservableTypes<Item = T, Error = E>,
{
    type Item = (T, Instant);
    type Error = E;
    type Mode = OE::Mode;
    type Disposal = OE::Disposal;
}

impl<T, E, OE, C, OR> Observable<OR> for Timestamp<OE, C>
where
    OR: Observer<(T, Instant), E>,
    OE: Observable<TimestampObserver<OR, C>, Item = T, Error = E>,
    C: Clock,
{
    fn subscribe(self, observer: OR) -> DisposeOnDrop<Self::Disposal> {
        let observer = TimestampObserver {
            observer,
            clock: self.clock,
        };
        self.source.subscribe(observer)
    }
}

#[doc(hidden)]
pub struct TimestampObserver<OR, C> {
    observer: OR,
    clock: C,
}

impl<T, E, OR, C> Observer<T, E> for TimestampObserver<OR, C>
where
    OR: Observer<(T, Instant), E>,
    C: Clock,
{
    fn on_next(&mut self, value: T) -> Flow {
        self.observer.on_next((value, self.clock.now()))
    }

    fn on_termination(self, termination: Termination<E>) {
        self.observer.on_termination(termination);
    }
}
