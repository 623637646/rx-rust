//! The [`Throttle`] operator, behind
//! [`ObservableExt::throttle`](crate::observable::ObservableExt::throttle).

use crate::observable::Subscription;
use crate::{
    observable::{Observable, ObservableTypes},
    observer::{Flow, Observer, Termination},
};
use educe::Educe;
use std::time::{Duration, Instant};

/// Emits an item from the source Observable, then ignores subsequent items for a particular time
/// span.
/// See <https://reactivex.io/documentation/operators/sample.html> (`throttleFirst`).
///
/// This is a purely synchronous, leading-edge throttle: it compares the arrival time of each item
/// against the last emission and needs no scheduler, so there is no timer to spawn, cancel, or
/// drift.
///
/// # Examples
/// ```rust
/// # #[cfg(not(feature = "tokio-scheduler"))]
/// # fn main() {}
/// # #[cfg(feature = "tokio-scheduler")]
/// #[tokio::main]
/// async fn main() {
///     use rx_rust::{
///         observable::ObservableExt,
///         observer::Termination,
///         operators::{
///             creating::from_iter::FromIter,
///             filtering::throttle::Throttle,
///         },
///     };
///     use std::sync::{Arc, Mutex};
///     use std::time::Duration;
///
///     let values = Arc::new(Mutex::new(Vec::new()));
///     let terminations = Arc::new(Mutex::new(Vec::new()));
///     let values_observer = Arc::clone(&values);
///     let terminations_observer = Arc::clone(&terminations);
///
///     let subscription = Throttle::new(
///         FromIter::new(vec![1, 2, 3]),
///         Duration::from_millis(5),
///     )
///     .subscribe_with_callback(
///         move |value| values_observer.lock().unwrap().push(value),
///         move |termination| terminations_observer
///             .lock()
///             .unwrap()
///             .push(termination),
///     );
///
///     drop(subscription);
///
///     // 1, 2 and 3 arrive back-to-back, so only the leading `1` passes.
///     assert_eq!(&*values.lock().unwrap(), &[1]);
///     assert_eq!(
///         &*terminations.lock().unwrap(),
///         &[Termination::Completed]
///     );
/// }
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct Throttle<OE> {
    source: OE,
    time_span: Duration,
}

impl<OE> Throttle<OE> {
    /// Creates a [`Throttle`] over `source`;
    /// [`ObservableExt::throttle`](crate::observable::ObservableExt::throttle) is the fluent form.
    pub fn new(source: OE, time_span: Duration) -> Self {
        Self { source, time_span }
    }
}

impl<T, E, OE> ObservableTypes for Throttle<OE>
where
    OE: ObservableTypes<Item = T, Error = E>,
{
    type Item = T;
    type Error = E;
    type Mode = OE::Mode;
    type Disposal = OE::Disposal;
}

impl<T, E, OE, OR> Observable<OR> for Throttle<OE>
where
    OR: Observer<T, E>,
    OE: Observable<ThrottleObserver<OR>, Item = T, Error = E>,
{
    fn subscribe(self, observer: OR) -> Subscription<Self::Disposal> {
        self.source.subscribe(ThrottleObserver {
            observer,
            time_span: self.time_span,
            last_emit: None,
        })
    }
}

pub struct ThrottleObserver<OR> {
    observer: OR,
    time_span: Duration,
    last_emit: Option<Instant>,
}

impl<T, E, OR> Observer<T, E> for ThrottleObserver<OR>
where
    OR: Observer<T, E>,
{
    fn on_next(&mut self, value: T) -> Flow {
        let now = Instant::now();
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
