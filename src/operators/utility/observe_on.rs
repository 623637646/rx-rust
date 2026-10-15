//! The [`ObserveOn`] operator, behind
//! [`ObservableExt::observe_on`](crate::observable::ObservableExt::observe_on).

use crate::{
    delegate_disposal,
    disposable::{
        Disposable, chain_disposal::ChainDisposal, dispose_on_drop::DisposeOnDrop,
        replaceable_disposal::ReplaceableDisposal,
    },
    observable::{JoinedSchedulerModeOf, Observable, ObservableTypes},
    observer::{Event, Flow, Observer, Termination},
    scheduler::{Next, Scheduler, SchedulerTypes, Task, TaskWaker, WakeableContext},
    thread_mode::{ThreadMode, mutable::MutableHelper},
    utils::serialized_delivery::DeliveryStopped,
};
use educe::Educe;

/// Specifies the `Scheduler` on which an observer will observe this Observable: every event is
/// delivered from a task of that scheduler, in order.
/// See <https://reactivex.io/documentation/operators/observeon.html>
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
///             utility::observe_on::ObserveOn,
///         },
///     };
///     use std::sync::{Arc, Mutex};
///     use tokio::time::{sleep, Duration};
///
///     let scheduler = rx_rust::scheduler::runtime::tokio::TokioScheduler::current();
///     let values = Arc::new(Mutex::new(Vec::new()));
///     let terminations = Arc::new(Mutex::new(Vec::new()));
///     let values_observer = Arc::clone(&values);
///     let terminations_observer = Arc::clone(&terminations);
///
///     let subscription = ObserveOn::new(FromIter::new(vec![1, 2, 3]), scheduler.clone())
///         .subscribe_with_callback(
///             move |value| values_observer.lock().unwrap().push(value),
///             move |termination| terminations_observer
///                 .lock()
///                 .unwrap()
///                 .push(termination),
///         );
///
///     sleep(Duration::from_millis(10)).await;
///     drop(subscription);
///
///     assert_eq!(&*values.lock().unwrap(), &[1, 2, 3]);
///     assert_eq!(
///         &*terminations.lock().unwrap(),
///         &[Termination::Completed]
///     );
/// }
/// ```
///
/// The source is subscribed with an observer that only queues the events and wakes the task of
/// `scheduler`, which is started when the operator is subscribed and owns the downstream observer.
/// So a source that emits from another thread can be observed on a `Local` scheduler, bound to the
/// thread it was made on, by an observer that is not `Send` — only the events cross the threads:
///
/// ```no_run
/// use rx_rust::{
///     observable::ObservableExt, operators::creating::from_future::FromFuture,
///     scheduler::runtime::tokio::{TokioLocalScheduler, TokioScheduler},
/// };
/// use std::rc::Rc;
///
/// let ui = Rc::new(());
/// let _subscription = FromFuture::new(async { 1 }, TokioScheduler::current())
///     .observe_on(TokioLocalScheduler::ambient())
///     .subscribe_with_callback(move |_| drop(ui.clone()), |_| {});
/// ```
///
/// On a `Shared` scheduler the task, and the observer it owns, may run on any thread, so that
/// observer must be `Send`:
///
/// ```compile_fail
/// use rx_rust::{
///     observable::ObservableExt, operators::creating::from_future::FromFuture,
///     scheduler::runtime::tokio::{TokioLocalScheduler, TokioScheduler},
/// };
/// use std::rc::Rc;
///
/// let ui = Rc::new(());
/// let _subscription = FromFuture::new(async { 1 }, TokioScheduler::current())
///     .observe_on(TokioScheduler::current())
///     .subscribe_with_callback(move |_| drop(ui.clone()), |_| {});
/// ```
///
/// The task owns the downstream observer, so a disposal stops the delivery at once but releases
/// that observer only when the scheduler drops the cancelled task, on the scheduler's thread.
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct ObserveOn<OE, S> {
    source: OE,
    scheduler: S,
}

impl<OE, S> ObserveOn<OE, S> {
    /// Creates an [`ObserveOn`] over `source`;
    /// [`ObservableExt::observe_on`](crate::observable::ObservableExt::observe_on) is the fluent
    /// form.
    pub fn new(source: OE, scheduler: S) -> Self {
        Self { source, scheduler }
    }
}

/// What the source's observer, the task and the disposal share: the events not delivered yet, and
/// whether the delivery has stopped. It holds no observer, so it is `Send` as soon as the events
/// are.
struct Inbox<T, E> {
    values: Vec<T>,
    termination: Option<Termination<E>>,
    /// The delivery stopped: disposed, or ended by downstream or by a termination. The source's
    /// observer then answers [`Flow::Stop`], and nothing more is queued.
    stopped: bool,
}

/// A handle on an [`Inbox`], the only place its lock is taken. Disposing it stops the delivery, so
/// that nothing is delivered once the subscription is disposed, even while the task is running.
#[derive(Educe)]
#[educe(Clone(bound()))]
struct InboxHandle<M: ThreadMode, T, E>(M::Ptr<Inbox<T, E>>);

impl<M: ThreadMode, T, E> InboxHandle<M, T, E> {
    fn new() -> Self {
        Self(M::ptr(Inbox {
            values: Vec::new(),
            termination: None,
            stopped: false,
        }))
    }

    /// Queues `event`, and returns whether the inbox was empty: the task must then be woken, while
    /// a task woken for an earlier event that it has not taken yet takes this one too. Once the
    /// delivery has stopped, drops `event` instead, outside the lock.
    fn push(&self, event: Event<T, E>) -> Result<bool, DeliveryStopped> {
        self.0
            .with_mut(|inbox| {
                if inbox.stopped {
                    return Err(event);
                }
                let was_empty = inbox.values.is_empty() && inbox.termination.is_none();
                match event {
                    Event::Next(value) => inbox.values.push(value),
                    Event::Termination(termination) => inbox.termination = Some(termination),
                }
                Ok(was_empty)
            })
            .map_err(|_| DeliveryStopped)
    }

    /// Takes everything queued, or `None` once the delivery has stopped.
    fn take(&self) -> Option<(Vec<T>, Option<Termination<E>>)> {
        self.0.with_mut(|inbox| {
            (!inbox.stopped).then(|| (std::mem::take(&mut inbox.values), inbox.termination.take()))
        })
    }

    fn is_stopped(&self) -> bool {
        self.0.with_ref(|inbox| inbox.stopped)
    }

    /// What the task checks before each value it delivers: `Err` once the delivery has stopped,
    /// the error once one is queued, which it takes, since it preempts the values not delivered
    /// yet, and `None` otherwise. A completion does not preempt them, so it is left queued.
    fn take_preempting_error(&self) -> Result<Option<Termination<E>>, DeliveryStopped> {
        self.0.with_mut(|inbox| {
            if inbox.stopped {
                Err(DeliveryStopped)
            } else if let Some(Termination::Error(_)) = inbox.termination {
                Ok(inbox.termination.take())
            } else {
                Ok(None)
            }
        })
    }

    /// Marks the delivery stopped, and drops what is still queued, outside the lock.
    fn stop(&self) {
        drop(self.0.with_mut(|inbox| {
            inbox.stopped = true;
            (std::mem::take(&mut inbox.values), inbox.termination.take())
        }));
    }
}

impl<M: ThreadMode, T, E> Disposable for InboxHandle<M, T, E> {
    fn dispose(self) {
        self.stop();
    }
}

/// The state of the [`ObserveOn`] task, one wakeable task for the whole subscription, which the
/// source's observer wakes.
///
/// The task owns the downstream observer: only the task touches it, so it never leaves the
/// scheduler's thread, and it needs to be `Send` only where the scheduler requires its tasks to
/// be. The observer is therefore released when the scheduler drops the task, promptly after a
/// disposal rather than within it.
struct Delivery<M: ThreadMode, T, E, OR, D: Disposable> {
    inbox: InboxHandle<M, T, E>,
    /// The source's subscription, disposed as the task stops (decision 0005).
    source_subscription: ReplaceableDisposal<M, D>,
    /// `None` once the termination is delivered.
    observer: Option<OR>,
}

impl<M: ThreadMode, T, E, OR, D: Disposable> Delivery<M, T, E, OR, D>
where
    OR: Observer<T, E>,
{
    /// One step of the task: delivers the events queued since the last one, then waits for the
    /// source to send more. `source_closed` is the `closed` of [`Task::wakeable`]: the source let
    /// go of its observer, so what this step took is all there is. Every [`Next::Finished`] stops
    /// the delivery, through the `Drop` of [`Delivery`].
    fn step(&mut self, source_closed: bool) -> Next {
        // `None`: disposed.
        let Some((mut values, termination)) = self.inbox.take() else {
            return Next::Finished;
        };
        // An error preempts the values buffered before it, unlike a completion.
        if let Some(Termination::Error(_)) = termination {
            values.clear();
        }
        let idle = values.is_empty();
        if !self.deliver(values) {
            return Next::Finished;
        }
        if let Some(termination) = termination {
            self.terminate(termination);
            Next::Finished
        } else if source_closed {
            // Released without a termination (decision 0004).
            Next::Finished
        } else if idle {
            Next::Pending
        } else {
            // Yield rather than park, so that the values arriving while this batch was delivered
            // are taken by the next pass without waiting for a wake.
            Next::Yield
        }
    }

    /// Delivers `values` in order, and returns whether the delivery may go on. Before each value
    /// it honors a disposal, which may come from the observer itself or from another thread, and
    /// an error queued since the step took `values`: the error is delivered instead of the values
    /// left, as it would have been had it come before the step.
    fn deliver(&mut self, values: Vec<T>) -> bool {
        for value in values {
            match self.inbox.take_preempting_error() {
                Err(DeliveryStopped) => return false,
                Ok(Some(error)) => {
                    self.terminate(error);
                    return false;
                }
                Ok(None) => {}
            }
            let Some(observer) = self.observer.as_mut() else {
                return false;
            };
            if observer.on_next(value).is_stop() {
                return false;
            }
        }
        true
    }

    /// Delivers `termination`, unless the delivery was disposed.
    fn terminate(&mut self, termination: Termination<E>) {
        if !self.inbox.is_stopped()
            && let Some(observer) = self.observer.take()
        {
            observer.on_termination(termination);
        }
    }
}

/// The only place the delivery stops on the task's side: the task is dropped once it finishes,
/// in the same poll as its last step, and also when it is dropped before — cancelled, dropped
/// with its runtime, or unwound by a panic of the observer — so that the source is never left
/// sending into an inbox nobody takes from. The observer is dropped after the source is disposed.
impl<M: ThreadMode, T, E, OR, D: Disposable> Drop for Delivery<M, T, E, OR, D> {
    fn drop(&mut self) {
        self.inbox.stop();
        self.source_subscription.clone().dispose();
    }
}

/// The task of an [`ObserveOn`].
type ObserveOnTask<M, T, E, OR, D> = WakeableContext<Delivery<M, T, E, OR, D>, M>;

delegate_disposal!(
    Disposal<M, T, E, SD, D>,
    ChainDisposal<ChainDisposal<InboxHandle<M, T, E>, SD>, ReplaceableDisposal<M, D>>,
    where M: ThreadMode, SD: Disposable, D: Disposable
);

impl<T, E, OE, S> ObservableTypes for ObserveOn<OE, S>
where
    OE: ObservableTypes<Item = T, Error = E>,
    S: SchedulerTypes,
{
    type Item = T;
    type Error = E;
    /// Every event is delivered by the scheduler's task, so the mode is the scheduler's.
    type Mode = S::Mode;
    /// First the delivery is stopped, then the task cancelled, then the source's subscription
    /// disposed. The inbox and the source's subscription are shared between the source's thread
    /// and the scheduler's, so their mode is both joined.
    type Disposal = Disposal<JoinedSchedulerModeOf<OE, S>, T, E, S::Disposal, OE::Disposal>;
}

impl<T, E, OE, S, OR> Observable<OR> for ObserveOn<OE, S>
where
    OR: Observer<T, E>,
    OE: Observable<ObserveOnObserver<JoinedSchedulerModeOf<OE, S>, T, E>, Item = T, Error = E>,
    S: Scheduler<
        ObserveOnTask<JoinedSchedulerModeOf<OE, S>, T, E, OR, <OE as ObservableTypes>::Disposal>,
    >,
{
    fn subscribe(self, observer: OR) -> DisposeOnDrop<Self::Disposal> {
        let inbox = InboxHandle::new();
        let source_subscription = ReplaceableDisposal::default();
        let (task, waker) = Task::wakeable(
            Delivery {
                inbox: inbox.clone(),
                source_subscription: source_subscription.clone(),
                observer: Some(observer),
            },
            |delivery, source_closed, _| delivery.step(source_closed),
        );
        let source_observer = ObserveOnObserver {
            inbox: inbox.clone(),
            waker,
        };
        source_subscription.replace_with(|| self.source.subscribe(source_observer));
        // The task holds the source's subscription only through the slot, and the runtime owns
        // the task: no cycle.
        self.scheduler
            .run_task(task, None)
            .preceded_by(inbox)
            .then(source_subscription)
            .map_inner_into()
    }
}

/// The observer an [`ObserveOn`] gives its source. It queues the events and wakes the task, and
/// holds neither the scheduler, which may be `Local` while the source sends from another thread,
/// nor the downstream observer, which the task owns: it is `Send` as soon as the events are.
#[doc(hidden)]
pub struct ObserveOnObserver<M: ThreadMode, T, E> {
    inbox: InboxHandle<M, T, E>,
    waker: TaskWaker<M>,
}

impl<M: ThreadMode, T, E> ObserveOnObserver<M, T, E> {
    /// Queues `event` for the observing scheduler, and wakes its task if nothing was queued.
    ///
    /// Returns [`Flow::Stop`] once the delivery has stopped: the event is then dropped, and so
    /// would every later one be. What downstream itself answers is only known once the task
    /// delivers, so this is otherwise [`Flow::Continue`].
    fn queue_event(&self, event: Event<T, E>) -> Flow {
        match self.inbox.push(event) {
            Ok(was_empty) => {
                if was_empty {
                    self.waker.wake();
                }
                Flow::Continue
            }
            Err(DeliveryStopped) => Flow::Stop,
        }
    }
}

impl<M: ThreadMode, T, E> Observer<T, E> for ObserveOnObserver<M, T, E> {
    fn on_next(&mut self, value: T) -> Flow {
        self.queue_event(Event::Next(value))
    }

    fn on_termination(self, termination: Termination<E>) {
        // The termination is the last event, so what the inbox answers is of no use here.
        let _ = self.queue_event(Event::Termination(termination));
    }
}
