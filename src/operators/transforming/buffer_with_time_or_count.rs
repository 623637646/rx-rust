use crate::disposable::Disposable;
use crate::disposable::boxed_disposal::BoxedDisposal;
use crate::disposable::subscription::Subscription;
use crate::utils::types::{MutGuard, Mutable, MutableHelper, NecessarySendSync, Shared};
use crate::{
    observable::Observable,
    observer::{Observer, Termination},
    scheduler::Scheduler,
};
use crate::{safe_lock, safe_lock_option, safe_lock_option_disposable, safe_lock_option_observer};
use educe::Educe;
use std::{num::NonZeroUsize, time::Duration};

/// Periodically gathers items from an Observable into bundles and emits these bundles as `Vec<T>`, either when the bundle reaches a specified size or after a specified time interval, whichever happens first.
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
///             transforming::buffer_with_time_or_count::BufferWithTimeOrCount,
///         },
///     };
///     use std::{
///         num::NonZeroUsize,
///         sync::{Arc, Mutex},
///         time::Duration,
///     };
///     use tokio::time::sleep;
///
///     let handle = tokio::runtime::Handle::current();
///     let values = Arc::new(Mutex::new(Vec::new()));
///     let terminations = Arc::new(Mutex::new(Vec::new()));
///     let values_observer = Arc::clone(&values);
///     let terminations_observer = Arc::clone(&terminations);
///
///     let subscription = BufferWithTimeOrCount::new(
///         FromIter::new(vec![1, 2, 3]),
///         NonZeroUsize::new(2).unwrap(),
///         Duration::from_millis(10),
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
///     sleep(Duration::from_millis(20)).await;
///     drop(subscription);
///
///     assert_eq!(&*values.lock().unwrap(), &[vec![1, 2], vec![3]]);
///     assert_eq!(
///         &*terminations.lock().unwrap(),
///         &[Termination::Completed]
///     );
/// }
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct BufferWithTimeOrCount<OE, S> {
    source: OE,
    count: NonZeroUsize,
    time_span: Duration,
    scheduler: S,
    delay: Option<Duration>,
}

impl<OE, S> BufferWithTimeOrCount<OE, S> {
    pub fn new(
        source: OE,
        count: NonZeroUsize,
        time_span: Duration,
        scheduler: S,
        delay: Option<Duration>,
    ) -> Self {
        Self {
            source,
            count,
            time_span,
            scheduler,
            delay,
        }
    }
}

impl<'sub, T, E, OE, S> Observable<'static, 'sub, Vec<T>, E> for BufferWithTimeOrCount<OE, S>
where
    T: NecessarySendSync + 'static,
    OE: Observable<'static, 'sub, T, E>,
    S: Scheduler,
{
    fn subscribe(
        self,
        observer: impl Observer<Vec<T>, E> + NecessarySendSync + 'static,
    ) -> Subscription<'sub> {
        let context = Shared::new(Mutable::new(BufferWithTimeOrCountContext {
            values: Vec::default(),
            timer: None,
        }));
        let buffer_observer = BufferWithTimeOrCountObserver {
            observer: Shared::new(Mutable::new(Some(observer))),
            context: context.clone(),
            count: self.count,
            time_span: self.time_span,
            scheduler: self.scheduler,
        };
        let observer = buffer_observer.observer.clone();
        let scheduler = buffer_observer.scheduler.clone();
        let delay = self.delay;
        let time_span = buffer_observer.time_span;
        let sub = self.source.subscribe(buffer_observer) + context.clone();
        setup_emit_timer(None, observer, context, scheduler, delay, time_span);
        sub
    }
}

struct BufferWithTimeOrCountContext<T> {
    values: Vec<T>,
    timer: Option<BoxedDisposal<'static>>,
}

impl<T> Disposable for Shared<Mutable<BufferWithTimeOrCountContext<T>>> {
    fn dispose(self) {
        safe_lock_option_disposable!(dispose: self, timer);
    }
}

struct BufferWithTimeOrCountObserver<T, OR, S> {
    observer: Shared<Mutable<Option<OR>>>,
    context: Shared<Mutable<BufferWithTimeOrCountContext<T>>>,
    count: NonZeroUsize,
    time_span: Duration,
    scheduler: S,
}

fn setup_emit_timer<T, E, OR, S>(
    mut lock: Option<MutGuard<'_, BufferWithTimeOrCountContext<T>>>,
    observer: Shared<Mutable<Option<OR>>>,
    context: Shared<Mutable<BufferWithTimeOrCountContext<T>>>,
    scheduler: S,
    delay: Option<Duration>,
    time_span: Duration,
) where
    T: NecessarySendSync + 'static,
    OR: Observer<Vec<T>, E> + NecessarySendSync + 'static,
    S: Scheduler,
{
    let context_cloned = context.clone();
    let disposal = scheduler.schedule_periodically(
        move |_| {
            let values = safe_lock!(mem_take: context_cloned, values);
            !safe_lock_option_observer!(on_next: observer, values)
        },
        time_span,
        delay,
    );
    let old_timer = if let Some(lock) = lock.as_mut() {
        lock.timer.replace(BoxedDisposal::new(disposal))
    } else {
        safe_lock_option!(replace: context, timer, BoxedDisposal::new(disposal))
    };
    drop(lock);
    if let Some(timer) = old_timer {
        timer.dispose();
    }
}

impl<T, E, OR, S> Observer<T, E> for BufferWithTimeOrCountObserver<T, OR, S>
where
    T: NecessarySendSync + 'static,
    OR: Observer<Vec<T>, E> + NecessarySendSync + 'static,
    S: Scheduler,
{
    fn on_next(&mut self, value: T) {
        self.context.lock_mut(|mut lock| {
            lock.values.push(value);
            if lock.values.len() >= self.count.get() {
                let values = std::mem::take(&mut lock.values);
                let observer = self.observer.clone();
                let context = self.context.clone();
                let scheduler = self.scheduler.clone();
                let time_span = self.time_span;
                setup_emit_timer(
                    Some(lock),
                    observer,
                    context,
                    scheduler,
                    Some(self.time_span),
                    time_span,
                );
                safe_lock_option_observer!(on_next: self.observer, values);
            }
        });
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
