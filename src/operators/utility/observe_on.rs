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
    thread_mode::ThreadMode,
};
use educe::Educe;
use inbox::{AfterValue, InboxHandle, Pushed};

/// Specifies the `Scheduler` on which an observer will observe this observable: every event is
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
///         operators::{creating::from_iter::FromIter, utility::observe_on::ObserveOn},
///         scheduler::runtime::tokio::TokioScheduler,
///     };
///     use futures::StreamExt;
///
///     let values = ObserveOn::new(FromIter::new([1, 2, 3]), TokioScheduler::current())
///         .into_stream()
///         .collect::<Vec<_>>()
///         .await;
///     assert_eq!(values, [1, 2, 3]);
/// }
/// ```
///
/// The source is subscribed with an observer that only queues the events and wakes the task of
/// `scheduler`, which is started before subscribing to the source and owns the downstream observer.
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
/// A disposal stops queueing and cancels the task. A notification that has already passed the
/// task's inbox check may still run concurrently with the disposal; disposing from `on_next`
/// prevents the next notification. The task owns the downstream observer and releases it when the
/// scheduler drops the cancelled task, on the scheduler's thread — also when the source's
/// subscription panics.
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

mod inbox {
    use crate::{
        disposable::Disposable,
        observer::{Event, Termination},
        thread_mode::ThreadMode,
        utils::{flagged_lock::FlaggedLock, take_leaving_room},
    };
    use educe::Educe;

    /// What the source's observer, the task and the disposal share: the events not delivered yet.
    /// It holds no observer, so it is `Send` as soon as the events are.
    struct Inbox<T, E> {
        values: Vec<T>,
        termination: Option<Termination<E>>,
    }

    /// What [`InboxHandle::push`] did with an event.
    pub(super) enum Pushed {
        /// Queued in an empty inbox: the task must be woken.
        Wake,
        /// Queued behind events that a task already woken has not taken yet, and takes this one
        /// too.
        Behind,
        /// The delivery has stopped: the event was dropped, outside the lock.
        Stopped,
    }

    /// What [`InboxHandle::check_after_value`] tells the task after each value it delivers: what
    /// the next event of the step is.
    pub(super) enum AfterValue<E> {
        /// Nothing preempts the next event: deliver it.
        Continue,
        /// An error queued since the step took its values, taken: it is delivered instead of those
        /// left.
        Preempted(Termination<E>),
        /// The delivery has stopped.
        Stopped,
    }

    /// A handle on an [`Inbox`], the only place its lock is taken. Disposing it stops queueing and
    /// tells the task to stop at its next inbox check. Its own module keeps the inbox out of reach
    /// of the rest of the operator, so that nothing else can take that lock.
    ///
    /// The inbox is `None` once the delivery has stopped: disposed, or ended by downstream or by a
    /// termination. The source's observer then answers [`Flow::Stop`](crate::observer::Flow::Stop),
    /// and nothing more is queued.
    ///
    /// The flag of the lock is raised as the delivery stops and as an error is queued: whatever
    /// the task must not deliver an event past. Until then, the task goes from one value to the
    /// next without taking the lock, so that it does not contend with the source pushing the next
    /// ones.
    #[derive(Educe)]
    #[educe(Clone(bound()))]
    pub(super) struct InboxHandle<M: ThreadMode, T, E>(FlaggedLock<M, Option<Inbox<T, E>>>);

    impl<M: ThreadMode, T, E> InboxHandle<M, T, E> {
        pub(super) fn new() -> Self {
            Self(FlaggedLock::new(Some(Inbox {
                values: Vec::new(),
                termination: None,
            })))
        }

        /// Queues `event`, unless the delivery has stopped: `event` is then dropped, outside the
        /// lock.
        pub(super) fn push(&self, event: Event<T, E>) -> Pushed {
            // An error preempts the values queued before it, so queuing one raises the flag, which
            // makes the task take the lock after the value it is delivering. Raising it on a
            // stopped inbox, which drops the error, changes nothing: the stop has raised it.
            let preempts = matches!(event, Event::Termination(Termination::Error(_)));
            let queue = |slot: &mut Option<Inbox<T, E>>| {
                let Some(inbox) = slot else {
                    return Err(event);
                };
                let pushed = if inbox.values.is_empty() && inbox.termination.is_none() {
                    Pushed::Wake
                } else {
                    Pushed::Behind
                };
                match event {
                    Event::Next(value) => inbox.values.push(value),
                    Event::Termination(termination) => inbox.termination = Some(termination),
                }
                Ok(pushed)
            };
            let queued = if preempts {
                self.0.raise_with(queue)
            } else {
                self.0.with_mut(queue)
            };
            queued.unwrap_or(Pushed::Stopped)
        }

        /// Takes everything queued, or `None` once the delivery has stopped. The inbox is left with
        /// room for as many values, so that those queued meanwhile do not grow a new `Vec` from
        /// nothing. A termination leaves no room: there is no next batch to reserve space for.
        pub(super) fn take(&self) -> Option<(Vec<T>, Option<Termination<E>>)> {
            self.0.with_mut(|slot| {
                slot.as_mut().map(|inbox| {
                    let room = if inbox.termination.is_some() {
                        0
                    } else {
                        inbox.values.len()
                    };
                    (
                        take_leaving_room(&mut inbox.values, room),
                        inbox.termination.take(),
                    )
                })
            })
        }

        /// Whether the delivery has stopped. A stop raises the flag, so this takes the lock only
        /// once the flag is raised.
        pub(super) fn is_stopped(&self) -> bool {
            self.0.if_raised(|slot| slot.is_none()).unwrap_or(false)
        }

        /// What the task checks after each value it delivers, before the next event. An error
        /// queued is taken, since it preempts the values not delivered yet; a completion does not,
        /// so it is left queued.
        ///
        /// Takes the lock only once the flag is raised: until then, the delivery has not stopped
        /// and no error is queued, so the next event is delivered. A stop or an error that lands
        /// right after the flag is read is seen after the next value, as one that lands right after
        /// the lock would be.
        pub(super) fn check_after_value(&self) -> AfterValue<E> {
            self.0
                .if_raised(|slot| match slot {
                    None => AfterValue::Stopped,
                    Some(inbox) => inbox
                        .termination
                        .take_if(|termination| matches!(termination, Termination::Error(_)))
                        .map_or(AfterValue::Continue, AfterValue::Preempted),
                })
                .unwrap_or(AfterValue::Continue)
        }

        /// Marks the delivery stopped, and drops what is still queued, outside the lock.
        pub(super) fn stop(&self) {
            drop(self.0.raise_with(Option::take));
        }
    }

    impl<M: ThreadMode, T, E> Disposable for InboxHandle<M, T, E> {
        fn dispose(self) {
            self.stop();
        }
    }

    #[cfg(test)]
    mod tests {
        use super::InboxHandle;
        use crate::{
            observer::{Event, Termination},
            thread_mode::Local,
        };

        #[test]
        fn test_take_after_termination_leaves_no_buffer() {
            for termination in [Termination::Completed, Termination::Error("error")] {
                let inbox = InboxHandle::<Local, _, _>::new();
                inbox.push(Event::Next(111));
                inbox.push(Event::Next(222));
                inbox.push(Event::Termination(termination.clone()));

                let (values, taken_termination) = inbox.take().unwrap();
                assert_eq!(values, [111, 222]);
                assert_eq!(taken_termination, Some(termination));
                inbox.0.with_mut(|slot| {
                    assert_eq!(slot.as_ref().unwrap().values.capacity(), 0);
                });
            }
        }
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
        let termination = match self.deliver(values) {
            AfterValue::Continue => termination,
            AfterValue::Preempted(error) => Some(error),
            AfterValue::Stopped => return Next::Finished,
        };
        if let Some(termination) = termination {
            // Dropping values preempted by an error may itself dispose the subscription.
            if !self.inbox.is_stopped()
                && let Some(observer) = self.observer.take()
            {
                observer.on_termination(termination);
            }
            Next::Finished
        } else if source_closed {
            // Released without a termination (decision 0004).
            Next::Finished
        } else {
            // A value that arrived while this step delivered found the inbox empty, and woke the
            // task, which registered its waker before the step: parking loses nothing.
            Next::Pending
        }
    }

    /// Delivers `values` in order, and answers what the inbox says after the last one: whether the
    /// termination the step took may follow, or an error preempts it.
    ///
    /// The first value follows the `take` that brought it, each other a check after the value
    /// before it. A disposal from a callback stops the next notification; one from another thread
    /// may race a notification that passed its check. An error queued since the step took
    /// `values` preempts those left once the next check sees it.
    fn deliver(&mut self, values: Vec<T>) -> AfterValue<E> {
        let Some(observer) = self.observer.as_mut() else {
            return AfterValue::Stopped;
        };
        for value in values {
            if observer.on_next(value).is_stop() {
                return AfterValue::Stopped;
            }
            match self.inbox.check_after_value() {
                AfterValue::Continue => {}
                stopped_or_preempted => return stopped_or_preempted,
            }
        }
        AfterValue::Continue
    }
}

/// The only place the delivery stops on the task's side: the task is dropped once it finishes,
/// in the same poll as its last step, and also when it is dropped before — cancelled, dropped
/// with its runtime, or unwound by a panic of the observer — so that the source is never left
/// sending into an inbox nobody takes from. The observer is dropped after the source is disposed,
/// with the fields, so that its release tells downstream the source is gone, as a stopped
/// `SerializedDelivery` releases them.
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
        // Start delivery before a synchronous source begins sending. Keep the whole disposal on
        // the stack so a panicking subscription stops the inbox and cancels the task too. The
        // task holds the source's subscription only through the slot, and the runtime owns the
        // task: no cycle.
        let subscription = self
            .scheduler
            .run_task(task, None)
            .preceded_by(inbox)
            .then(source_subscription.clone())
            .map_inner_into();
        // The task may stop while the source is still subscribing: the slot then disposes the
        // source's subscription as soon as it is returned.
        source_subscription.replace_with(|| self.source.subscribe(source_observer));
        subscription
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
            Pushed::Wake => {
                self.waker.wake();
                Flow::Continue
            }
            Pushed::Behind => Flow::Continue,
            Pushed::Stopped => Flow::Stop,
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
