//! One group at a time, rotated when it reaches `count` items or every `time_span`, whichever comes
//! first: the state machine of
//! [`BufferWithTimeOrCount`](super::super::buffer_with_time_or_count::BufferWithTimeOrCount) and
//! [`WindowWithTimeOrCount`](super::super::window_with_time_or_count::WindowWithTimeOrCount)
//! (decision 0014).
//!
//! One context serializes the source against the timer: its model holds the [`Groups`], the count
//! of items in the open one and the next deadline, and its events are those of the groups. A full
//! group restarts the timer: it only moves the deadline, and the one long-lived task, waking at an
//! obsolete deadline, sleeps on to the current one, so no task is spawned or cancelled per full
//! group, and no `Duration` is subtracted, which a late tick could underflow.

use super::group::{Events, Groups, update_groups};
use crate::{
    disposable::{Disposable, dispose_on_drop::DisposeOnDrop},
    observable::{ChainSchedulerDisposalOf, JoinedSchedulerModeOf, Observable, ObservableTypes},
    observer::{Flow, Observer, Termination},
    scheduler::{Clock, Next, Scheduler, SchedulerTypes, Task},
    thread_mode::ThreadMode,
    utils::subscribe_with_context::{
        SubscriptionContext, TwoStepsDisposal, subscribe_with_context_two_steps,
    },
};
use std::{
    num::NonZeroUsize,
    time::{Duration, Instant},
};

/// The state of a time-or-count subscription.
pub(crate) struct Model<G> {
    /// The one open group, which the machine replaces as it rotates.
    groups: G,
    /// How many items the open group has received.
    received: usize,
    /// When the next timed rotation is due: `time_span` after the subscription, then after the
    /// rotation before, timed or full. `None` when that is too far out for an [`Instant`] to
    /// represent: it never comes, nor does any later one, since the clock only moves forward, and
    /// the groups are rotated by count only.
    deadline: Option<Instant>,
    /// Fixed; here for the timer task, whose handler is a plain `fn`, and for the source's
    /// observer, which restarts the timer.
    time_span: Duration,
    count: NonZeroUsize,
}

/// The context of a time-or-count subscription, whose groups `G` emit `Ev`, delivered to `OR`.
type Context<M, G, Ev, E, OR, D> = SubscriptionContext<M, Ev, E, OR, Model<G>, D>;

/// The task of the timer of a time-or-count subscription over `OE` on `S`. It holds the context
/// strongly, so that the groups keep being rotated after the source has let go of its observer
/// without a termination, until the subscription is disposed. A disposal still releases the
/// observer at once: disposing the source makes it drop its own handle, and a handle dropped once
/// the context has stopped releases the observer.
pub(crate) type TimeOrCountTask<OE, S, G, Ev, E, OR> =
    Context<JoinedSchedulerModeOf<OE, S>, G, Ev, E, OR, ChainSchedulerDisposalOf<S, OE>>;

/// The observer a time-or-count operator over `OE` on `S` gives its source.
pub(crate) type TimeOrCountObserverOf<OE, S, G, Ev, E, OR> = TimeOrCountObserver<
    JoinedSchedulerModeOf<OE, S>,
    G,
    Ev,
    E,
    OR,
    ChainSchedulerDisposalOf<S, OE>,
    <S as SchedulerTypes>::Clock,
>;

/// The disposal of a time-or-count subscription over `OE` on `S`, before the operator names it.
pub(crate) type TimeOrCountDisposal<OE, S, G, Ev, E> = TwoStepsDisposal<
    JoinedSchedulerModeOf<OE, S>,
    Ev,
    E,
    Model<G>,
    <OE as ObservableTypes>::Disposal,
    <S as SchedulerTypes>::Disposal,
>;

/// Subscribes `source` with the groups `groups`, whose events go to `observer`: the first group
/// opens now, and the timer is started after the source is subscribed.
///
/// # Panics
///
/// Panics if `time_span` is zero.
pub(crate) fn subscribe<T, E, OE, S, G, OR>(
    source: OE,
    count: NonZeroUsize,
    time_span: Duration,
    scheduler: S,
    mut groups: G,
    mut observer: OR,
) -> DisposeOnDrop<TimeOrCountDisposal<OE, S, G, G::Event, E>>
where
    G: Groups<E, Item = T>,
    OR: Observer<G::Event, E>,
    OE: Observable<TimeOrCountObserverOf<OE, S, G, G::Event, E, OR>, Item = T, Error = E>,
    S: Scheduler<TimeOrCountTask<OE, S, G, G::Event, E, OR>>,
{
    assert!(!time_span.is_zero(), "time_span must be non-zero");
    let clock = scheduler.clock();
    // The first deadline is counted from before the first window is emitted, so that a slow
    // observer does not push it back.
    let deadline = clock.now().checked_add(time_span);
    // The timer starts at once and its handler sleeps until the absolute deadline; with none to
    // represent, it is delayed by `time_span`, out of range too, and stays parked until disposed.
    // Known now, without a lock: the clock only moves forward, so a deadline that is `None` stays
    // so, and one that a full group moves out of range meanwhile ends the task at its first step.
    let delay = deadline.is_none().then_some(time_span);
    // The first group opens before the source is subscribed, so that synchronous values have
    // somewhere to go, and before the context exists, which nothing else can reach yet: without a
    // lock. An observer that stops on the first window keeps the source and timer from starting.
    let flow = groups.open(&mut observer);
    let model = Model {
        groups,
        received: 0,
        deadline,
        time_span,
        count,
    };
    subscribe_with_context_two_steps(
        observer,
        model,
        |context| {
            if flow.is_stop() {
                return None;
            }
            Some(source.subscribe(TimeOrCountObserver {
                context: context.clone(),
                clock,
            }))
        },
        |context| Some(setup_timer(context, scheduler, delay)),
    )
}

/// The observer a time-or-count operator gives its source. It holds the scheduler's clock rather
/// than the scheduler, which may be `Local` while the source sends from another thread.
#[doc(hidden)]
pub struct TimeOrCountObserver<M: ThreadMode, G, Ev, E, OR, D: Disposable, C> {
    context: Context<M, G, Ev, E, OR, D>,
    clock: C,
}

impl<M, T, E, G, Ev, OR, D, C> Observer<T, E> for TimeOrCountObserver<M, G, Ev, E, OR, D, C>
where
    M: ThreadMode,
    G: Groups<E, Item = T, Event = Ev>,
    OR: Observer<Ev, E>,
    D: Disposable,
    C: Clock,
{
    fn on_next(&mut self, value: T) -> Flow {
        // Kept outside the update, so that an item it does not take — once the stream is over — is
        // dropped outside the lock.
        let mut value = Some(value);
        update_groups(&self.context, |model, events| {
            let _ = model.groups.put(&mut value, events);
            model.received += 1;
            if model.received == model.count.get() {
                // A full group restarts the timer: the next timed rotation comes `time_span` after
                // it, never before the one already due. Out of range means never.
                model.deadline = self.clock.now().checked_add(model.time_span);
                model.received = 0;
                let _ = model.groups.rotate(events);
            }
        })
        .flow()
    }

    fn on_termination(self, termination: Termination<E>) {
        let _ = self
            .context
            .update(|model| Events::ending(&mut model.groups, termination));
    }
}

/// Drives the timed rotations with one long-lived scheduler task, as `timeout` does, started after
/// `delay`.
fn setup_timer<M, E, G, OR, D, S>(
    context: &Context<M, G, G::Event, E, OR, D>,
    scheduler: S,
    delay: Option<Duration>,
) -> DisposeOnDrop<S::Disposal>
where
    M: ThreadMode,
    G: Groups<E>,
    OR: Observer<G::Event, E>,
    D: Disposable,
    S: Scheduler<Context<M, G, G::Event, E, OR, D>>,
{
    // The context owns only the task's disposal, through its source subscription, never the task,
    // which its runtime owns: holding the context strongly forms no cycle. Stopping the context
    // disposes its source subscription, which cancels the task.
    let task = Task::new(context.clone(), |context, _, _, now| {
        update_groups(context, |model, events| match model.deadline {
            Some(deadline) if now < deadline => Next::SleepUntil(deadline),
            Some(deadline) => {
                // Fixed-rate: the next deadline is counted from this one, not from `now`, so that
                // a late tick does not push the next ones back.
                model.deadline = deadline.checked_add(model.time_span);
                model.received = 0;
                let _ = model.groups.rotate(events);
                model.deadline.map_or(Next::Finished, Next::SleepUntil)
            }
            // A full group moved the deadline out of range: no timed rotation is left.
            None => Next::Finished,
        })
        .result()
        .unwrap_or(Next::Finished)
    });
    scheduler.run_task(task, delay)
}
