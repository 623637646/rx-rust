//! The [`Timestamp`] operator, behind
//! [`ObservableExt::timestamp`](crate::observable::ObservableExt::timestamp).

use crate::{
    observable::Subscription,
    observable::{Observable, ObservableTypes},
    observer::{Flow, Observer, Termination},
    scheduler::SchedulerTypes,
};
use educe::Educe;
use std::time::Instant;

/// Attaches the [`Instant`] it arrived at, on the clock of a scheduler
/// ([`SchedulerTypes::now`]), to each item emitted by an Observable.
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
/// let subscription = Timestamp::new(subject.clone(), time.scheduler())
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
pub struct Timestamp<OE, S> {
    source: OE,
    scheduler: S,
}

impl<OE, S> Timestamp<OE, S> {
    /// Creates a [`Timestamp`] over `source`, on the clock of `scheduler`;
    /// [`ObservableExt::timestamp`](crate::observable::ObservableExt::timestamp) is the fluent
    /// form.
    pub fn new(source: OE, scheduler: S) -> Self {
        Self { source, scheduler }
    }
}

impl<T, E, OE, S> ObservableTypes for Timestamp<OE, S>
where
    OE: ObservableTypes<Item = T, Error = E>,
{
    type Item = (T, Instant);
    type Error = E;
    type Mode = OE::Mode;
    type Disposal = OE::Disposal;
}

impl<T, E, OE, S, OR> Observable<OR> for Timestamp<OE, S>
where
    OR: Observer<(T, Instant), E>,
    OE: Observable<TimestampObserver<OR, S>, Item = T, Error = E>,
    S: SchedulerTypes,
{
    fn subscribe(self, observer: OR) -> Subscription<Self::Disposal> {
        let observer = TimestampObserver {
            observer,
            scheduler: self.scheduler,
        };
        self.source.subscribe(observer)
    }
}

pub struct TimestampObserver<OR, S> {
    observer: OR,
    scheduler: S,
}

impl<T, E, OR, S> Observer<T, E> for TimestampObserver<OR, S>
where
    OR: Observer<(T, Instant), E>,
    S: SchedulerTypes,
{
    fn on_next(&mut self, value: T) -> Flow {
        self.observer.on_next((value, self.scheduler.now()))
    }

    fn on_termination(self, termination: Termination<E>) {
        self.observer.on_termination(termination);
    }
}
