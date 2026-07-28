use crate::observable::Subscription;
use crate::utils::types::MaybeSend;
use crate::{
    observable::Observable,
    observer::{Observer, Termination},
};
use educe::Educe;
use std::time::{Duration, Instant};

/// Emits an item from the source Observable then ignores subsequent items for a particular time span.
/// See <https://reactivex.io/documentation/operators/debounce.html>
///
/// This is a purely synchronous, leading-edge throttle: it compares the arrival
/// time of each item against the last emission and needs no scheduler. Dropping
/// items during the cooldown window is decided by an [`Instant`] comparison, so
/// there is no timer to spawn, cancel, or drift.
///
/// # Examples
/// ```rust
/// # #[cfg(not(feature = "tokio-scheduler"))]
/// # fn main() {
/// #     panic!("Use tokio-scheduler feature to run tests.");
/// # }
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
    pub fn new(source: OE, time_span: Duration) -> Self {
        Self { source, time_span }
    }
}

impl<'or, T, E, OE> Observable<'or, T, E> for Throttle<OE>
where
    OE: Observable<'or, T, E>,
{
    type D = OE::D;

    fn subscribe(self, observer: impl Observer<T, E> + MaybeSend + 'or) -> Subscription<Self::D> {
        self.source.subscribe(ThrottleObserver {
            observer,
            time_span: self.time_span,
            last_emit: None,
        })
    }
}

struct ThrottleObserver<OR> {
    observer: OR,
    time_span: Duration,
    last_emit: Option<Instant>,
}

impl<T, E, OR> Observer<T, E> for ThrottleObserver<OR>
where
    OR: Observer<T, E>,
{
    fn on_next(&mut self, value: T) {
        let now = Instant::now();
        // `on_next` takes `&mut self`, so it is called exclusively — a plain
        // field suffices, no shared/atomic state is needed.
        let should_emit = match self.last_emit {
            None => true,
            Some(last) => now.duration_since(last) >= self.time_span,
        };
        if should_emit {
            self.last_emit = Some(now);
            self.observer.on_next(value);
        }
    }

    fn on_termination(self, termination: Termination<E>) {
        self.observer.on_termination(termination);
    }
}
