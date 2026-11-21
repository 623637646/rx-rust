//! The [`Debounce`] operator, behind
//! [`ObservableExt::debounce`](crate::observable::ObservableExt::debounce).

#[doc(hidden)]
pub use super::internal::deadline::DeadlineObserver;
pub use super::internal::deadline::Disposal;
use super::internal::deadline::{self, DeadlineObserverOf, DeadlineTask};
use crate::{
    disposable::dispose_on_drop::DisposeOnDrop,
    observable::{JoinedSchedulerModeOf, Observable, ObservableTypes},
    observer::Observer,
    scheduler::{Scheduler, SchedulerTypes},
};
use educe::Educe;
use std::time::Duration;

/// Emits an item from the source observable only after a particular time span has passed without
/// another source emission.
/// See <https://reactivex.io/documentation/operators/debounce.html>
///
/// A `time_span` too long for an [`Instant`](std::time::Instant) to represent never passes: an
/// item is emitted only by the completion, which emits the last one.
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt,
///     observer::Observer,
///     operators::filtering::debounce::Debounce,
///     scheduler::virtual_time::VirtualTime,
///     subject::publish_subject::PublishSubject,
///     thread_mode::mutable::{MutableExt, MutableHelper},
/// };
/// use std::{convert::Infallible, sync::{Arc, Mutex}, time::Duration};
///
/// let time = VirtualTime::new();
/// let span = Duration::from_millis(5);
/// let values = Arc::new(Mutex::new(Vec::new()));
/// let values_observer = values.clone();
/// let mut source = PublishSubject::<_, Infallible, _>::shared();
/// let _subscription = Debounce::new(source.clone(), span, time.scheduler())
/// .subscribe_with_callback(
///     move |value| values_observer.with_mut(|values| values.push(value)),
///     |_| {},
/// );
///
/// let _ = source.on_next(1);
/// time.advance_by(Duration::from_millis(4));
/// let _ = source.on_next(2); // Restarts the quiet period.
/// time.advance_by(Duration::from_millis(4));
/// assert!(values.with_ref(Vec::is_empty));
/// time.advance_by(Duration::from_millis(1));
/// assert_eq!(values.clone_value(), [2]);
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct Debounce<OE, S> {
    source: OE,
    time_span: Duration,
    scheduler: S,
}

impl<OE, S> Debounce<OE, S> {
    /// Creates a [`Debounce`] over `source`;
    /// [`ObservableExt::debounce`](crate::observable::ObservableExt::debounce) is the fluent form.
    pub fn new(source: OE, time_span: Duration, scheduler: S) -> Self {
        Self {
            source,
            time_span,
            scheduler,
        }
    }
}

impl<T, E, OE, S> ObservableTypes for Debounce<OE, S>
where
    OE: ObservableTypes<Item = T, Error = E>,
    S: SchedulerTypes,
{
    type Item = T;
    type Error = E;
    /// The timer's thread emits the values, the source's the terminations, so the mode is both
    /// joined.
    type Mode = JoinedSchedulerModeOf<OE, S>;
    type Disposal = Disposal<JoinedSchedulerModeOf<OE, S>, T, E, S::Disposal, OE::Disposal>;
}

impl<T, E, OE, S, OR> Observable<OR> for Debounce<OE, S>
where
    OR: Observer<T, E>,
    OE: Observable<DeadlineObserverOf<OE, S, OR, true>, Item = T, Error = E>,
    S: Scheduler<DeadlineTask<OE, S, OR>>,
{
    fn subscribe(self, observer: OR) -> DisposeOnDrop<Self::Disposal> {
        deadline::subscribe(self.source, self.time_span, self.scheduler, observer)
    }
}
