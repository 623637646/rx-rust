use crate::disposable::Disposable;
use crate::disposable::boxed_disposal::BoxedDisposal;
use crate::utils::types::{Mutable, MutableHelper, NecessarySend, Shared};
use crate::{
    disposable::subscription::Subscription,
    observable::Observable,
    observer::{Observer, Termination},
    scheduler::Scheduler,
};
use crate::{
    safe_lock, safe_lock_option, safe_lock_option_disposable, safe_lock_option_observer,
    safe_lock_vec,
};
use educe::Educe;
use std::time::Duration;

/// Periodically gathers items from an Observable into bundles and emits these bundles as `Vec<T>`, after a specified time interval.
/// See <https://reactivex.io/documentation/operators/buffer.html>
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
///         observable::observable_ext::ObservableExt,
///         observer::Termination,
///         operators::{
///             creating::from_iter::FromIter,
///             transforming::buffer_with_time::BufferWithTime,
///         },
///     };
///     use std::sync::{Arc, Mutex};
///     use std::time::Duration;
///     use tokio::time::sleep;
///
///     let handle = tokio::runtime::Handle::current();
///     let values = Arc::new(Mutex::new(Vec::new()));
///     let terminations = Arc::new(Mutex::new(Vec::new()));
///     let values_observer = Arc::clone(&values);
///     let terminations_observer = Arc::clone(&terminations);
///
///     let subscription = BufferWithTime::new(
///         FromIter::new(vec![1, 2, 3]),
///         Duration::from_millis(5),
///         handle.clone(),
///         None,
///     )
///     .subscribe_with_callback(
///         move |value| values_observer.lock().unwrap().push(value),
///         move |termination| terminations_observer
///             .lock()
///             .unwrap()
///             .push(termination),
///     );
///
///     sleep(Duration::from_millis(10)).await;
///     drop(subscription);
///
///     assert_eq!(&*values.lock().unwrap(), &[vec![1, 2, 3]]);
///     assert_eq!(
///         &*terminations.lock().unwrap(),
///         &[Termination::Completed]
///     );
/// }
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct BufferWithTime<OE, S> {
    source: OE,
    time_span: Duration,
    scheduler: S,
    delay: Option<Duration>,
}

impl<OE, S> BufferWithTime<OE, S> {
    pub fn new(source: OE, time_span: Duration, scheduler: S, delay: Option<Duration>) -> Self {
        Self {
            source,
            time_span,
            scheduler,
            delay,
        }
    }
}

impl<'or, 'sub, T, E, OE, S> Observable<'static, 'sub, Vec<T>, E> for BufferWithTime<OE, S>
where
    T: NecessarySend + 'static,
    OE: Observable<'or, 'sub, T, E>,
    S: Scheduler,
{
    fn subscribe(
        self,
        observer: impl Observer<Vec<T>, E> + NecessarySend + 'static,
    ) -> Subscription<'sub> {
        let observer = Shared::new(Mutable::new(Some(observer)));
        let context = Shared::new(Mutable::new(BufferWithTimeContext {
            values: Vec::new(),
            timer: None,
        }));
        let sub = self.source.subscribe(BufferWithTimeObserver {
            observer: observer.clone(),
            context: context.clone(),
        });
        let context_cloned = context.clone();
        let disposal = self.scheduler.schedule_periodically(
            move |_| {
                let values = safe_lock!(mem_take: context_cloned, values);
                !safe_lock_option_observer!(on_next: observer, values)
            },
            self.time_span,
            self.delay,
        );
        safe_lock_option!(replace: context, timer, BoxedDisposal::new(disposal));
        sub + context
    }
}

struct BufferWithTimeContext<T> {
    values: Vec<T>,
    timer: Option<BoxedDisposal<'static>>,
}

// TODO: Disposable should not be Cloneable
impl<T> Disposable for Shared<Mutable<BufferWithTimeContext<T>>> {
    fn dispose(self) {
        safe_lock_option_disposable!(dispose: self, timer);
    }
}

struct BufferWithTimeObserver<T, OR> {
    observer: Shared<Mutable<Option<OR>>>,
    context: Shared<Mutable<BufferWithTimeContext<T>>>,
}

impl<T, E, OR> Observer<T, E> for BufferWithTimeObserver<T, OR>
where
    OR: Observer<Vec<T>, E>,
{
    fn on_next(&mut self, value: T) {
        safe_lock_vec!(push: self.context, values, value);
    }

    fn on_termination(self, termination: Termination<E>) {
        let values = self.context.lock_mut(|mut lock| {
            if let Some(timer) = lock.timer.take() {
                timer.dispose();
            }
            std::mem::take(&mut lock.values)
        });
        match termination {
            Termination::Completed => {
                if !values.is_empty() {
                    safe_lock_option_observer!(on_next_and_termination: self.observer, values, termination);
                } else {
                    safe_lock_option_observer!(on_termination: self.observer, termination);
                }
            }
            Termination::Error(_) => {
                safe_lock_option_observer!(on_termination: self.observer, termination);
            }
        }
    }
}
