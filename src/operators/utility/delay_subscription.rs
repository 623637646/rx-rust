//! Subscribes to the source after a delay.
//!
//! The [`DelaySubscription`] operator, behind
//! [`ObservableExt::delay_subscription`](crate::observable::ObservableExt::delay_subscription).

use crate::{
    disposable::dispose_on_drop::DisposeOnDrop,
    observable::{JoinedSchedulerModeOf, Observable, ObservableTypes},
    observer::Observer,
    operators::utility::subscribe_on::{self, SubscribeOn, SubscribeOnTask},
    scheduler::{Scheduler, SchedulerTypes},
};
use educe::Educe;
use std::time::Duration;

/// Subscribes to the source only once a span of time has passed, on a scheduler.
/// See <https://reactivex.io/documentation/operators/delay.html> (`delaySubscription`).
///
/// Where [`Delay`](crate::operators::utility::delay::Delay) subscribes at once and shifts every
/// item, this shifts the subscription itself: a cold source starts later, and a hot one is joined
/// later, so what it emits in the meantime is not seen. A disposal before the delay is over
/// cancels the subscription, which then never happens.
///
/// It is a [`SubscribeOn`] whose task waits `delay` first, so the source is subscribed from a task
/// of `scheduler`, and the mode joins the source's and the scheduler's, as there.
///
/// It is also what a [`Retry`](crate::operators::error_handling::retry::Retry) with a back-off
/// retries with; see its documentation.
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt,
///     observer::Termination,
///     operators::{creating::just::Just, utility::delay_subscription::DelaySubscription},
///     scheduler::virtual_time::VirtualTime,
///     thread_mode::mutable::{MutableExt, MutableHelper},
/// };
/// use std::{sync::{Arc, Mutex}, time::Duration};
///
/// let time = VirtualTime::new();
/// let values = Arc::new(Mutex::new(Vec::new()));
/// let values_observer = values.clone();
///
/// let _subscription =
///     DelaySubscription::new(Just::new(1), Duration::from_millis(5), time.scheduler())
///         .subscribe_with_callback(
///             move |value| values_observer.with_mut(|values| values.push(value)),
///             |termination| assert_eq!(termination, Termination::Completed),
///         );
///
/// time.advance_by(Duration::from_millis(4));
/// assert!(values.with_ref(Vec::is_empty)); // Not subscribed yet.
/// time.advance_by(Duration::from_millis(1));
/// assert_eq!(values.clone_value(), [1]);
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct DelaySubscription<OE, S> {
    source: OE,
    delay: Duration,
    scheduler: S,
}

impl<OE, S> DelaySubscription<OE, S> {
    /// Creates a [`DelaySubscription`] over `source`;
    /// [`ObservableExt::delay_subscription`](crate::observable::ObservableExt::delay_subscription)
    /// is the fluent form.
    pub fn new(source: OE, delay: Duration, scheduler: S) -> Self {
        Self {
            source,
            delay,
            scheduler,
        }
    }
}

impl<T, E, OE, S> ObservableTypes for DelaySubscription<OE, S>
where
    OE: ObservableTypes<Item = T, Error = E>,
    S: SchedulerTypes,
{
    type Item = T;
    type Error = E;
    /// The mode of the [`SubscribeOn`] it is, with a delay.
    type Mode = JoinedSchedulerModeOf<OE, S>;
    /// The disposal of the [`SubscribeOn`] it is, with a delay.
    type Disposal = subscribe_on::Disposal<JoinedSchedulerModeOf<OE, S>, S::Disposal, OE::Disposal>;
}

impl<T, E, OE, S, OR> Observable<OR> for DelaySubscription<OE, S>
where
    OR: Observer<T, E>,
    OE: Observable<OR, Item = T, Error = E>,
    S: Scheduler<SubscribeOnTask<OE, OR, S>>,
{
    fn subscribe(self, observer: OR) -> DisposeOnDrop<Self::Disposal> {
        SubscribeOn::delayed(self.source, self.scheduler, self.delay).subscribe(observer)
    }
}
