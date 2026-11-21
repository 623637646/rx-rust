//! The groups opened every `time_shift`, each closed `time_span` after it opened: the state
//! machine of
//! [`BufferWithTimeAndShift`](super::super::buffer_with_time_and_shift::BufferWithTimeAndShift)
//! and [`WindowWithTimeAndShift`](super::super::window_with_time_and_shift::WindowWithTimeAndShift),
//! and, with a `time_shift` equal to `time_span`, of
//! [`BufferWithTime`](super::super::buffer_with_time::BufferWithTime) and
//! [`WindowWithTime`](super::super::window_with_time::WindowWithTime) (decision 0014).
//!
//! One context serializes the source against the timer: its model holds the [`Groups`] and the
//! [`Timetable`], and its events are those of the groups. The timer is one long-lived scheduler
//! task, which takes one instant of the timetable per step.

use super::{
    group::{Events, Groups, update_groups},
    timetable::{Timetable, Transition},
};
use crate::{
    disposable::{Disposable, dispose_on_drop::DisposeOnDrop},
    observable::{ChainSchedulerDisposalOf, JoinedSchedulerModeOf, Observable, ObservableTypes},
    observer::{Flow, Observer, Termination},
    scheduler::{Clock, Next, Scheduler, SchedulerTypes, Task},
    thread_mode::ThreadMode,
    utils::{
        serialized_delivery::Updated,
        subscribe_with_context::{
            SubscriptionContext, TwoStepsDisposal, subscribe_with_context_two_steps,
        },
    },
};
use std::time::Duration;

/// The state of a time-and-shift subscription.
pub(crate) struct Model<G> {
    /// The groups open, the oldest, the first to close, first.
    groups: G,
    /// When they close, and when the next one opens.
    timetable: Timetable,
}

/// The context of a time-and-shift subscription, whose groups `G` emit `Ev`, delivered to `OR`.
type Context<M, G, Ev, E, OR, D> = SubscriptionContext<M, Ev, E, OR, Model<G>, D>;

/// The task of the timer of a time-and-shift subscription over `OE` on `S`. It holds the context
/// strongly, so that the groups keep being opened and closed after the source has let go of its
/// observer without a termination, until the subscription is disposed. A disposal still releases
/// the observer at once: disposing the source makes it drop its own handle, and a handle dropped
/// once the context has stopped releases the observer.
pub(crate) type TimeShiftTask<OE, S, G, Ev, E, OR> =
    Context<JoinedSchedulerModeOf<OE, S>, G, Ev, E, OR, ChainSchedulerDisposalOf<S, OE>>;

/// The observer a time-and-shift operator over `OE` on `S` gives its source.
pub(crate) type TimeShiftObserverOf<OE, S, G, Ev, E, OR> =
    TimeShiftObserver<JoinedSchedulerModeOf<OE, S>, G, Ev, E, OR, ChainSchedulerDisposalOf<S, OE>>;

/// The disposal of a time-and-shift subscription over `OE` on `S`, before the operator names it.
pub(crate) type TimeShiftDisposal<OE, S, G, Ev, E> = TwoStepsDisposal<
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
/// Panics if `time_span` or `time_shift` is zero.
pub(crate) fn subscribe<T, E, OE, S, G, OR>(
    source: OE,
    time_span: Duration,
    time_shift: Duration,
    scheduler: S,
    mut groups: G,
    mut observer: OR,
) -> DisposeOnDrop<TimeShiftDisposal<OE, S, G, G::Event, E>>
where
    G: Groups<E, Item = T>,
    OR: Observer<G::Event, E>,
    OE: Observable<TimeShiftObserverOf<OE, S, G, G::Event, E, OR>, Item = T, Error = E>,
    S: Scheduler<TimeShiftTask<OE, S, G, G::Event, E, OR>>,
{
    // The times are counted from before the first window is emitted and the source is subscribed,
    // so that a slow observer or subscription does not push them back.
    let timetable = Timetable::starting_at(scheduler.clock().now(), time_span, time_shift);
    // The first group opens before the source is subscribed, so that synchronous values have
    // somewhere to go, and before the context exists, which nothing else can reach yet: without a
    // lock. An observer that stops on the first window keeps the source and timer from starting.
    let flow = groups.open(&mut observer);
    subscribe_with_context_two_steps(
        observer,
        Model { groups, timetable },
        |context| {
            if flow.is_stop() {
                return None;
            }
            Some(source.subscribe(TimeShiftObserver(context.clone())))
        },
        |context| Some(setup_timer(context, scheduler)),
    )
}

#[doc(hidden)]
pub struct TimeShiftObserver<M: ThreadMode, G, Ev, E, OR, D: Disposable>(
    Context<M, G, Ev, E, OR, D>,
);

impl<M, T, E, G, Ev, OR, D> Observer<T, E> for TimeShiftObserver<M, G, Ev, E, OR, D>
where
    M: ThreadMode,
    G: Groups<E, Item = T, Event = Ev>,
    OR: Observer<Ev, E>,
    D: Disposable,
{
    fn on_next(&mut self, value: T) -> Flow {
        // Kept outside the update, so that an item no group takes is dropped outside the lock, as
        // is the original if cloning it panics.
        let mut value = Some(value);
        update_groups(&self.0, |model, events| {
            let _ = model.groups.put(&mut value, events);
        })
        .flow()
    }

    fn on_termination(self, termination: Termination<E>) {
        let _ = self
            .0
            .update(|model| Events::ending(&mut model.groups, termination));
    }
}

/// Opens and closes the groups with one long-lived scheduler task, one instant of the timetable
/// per step. The task finishes once the timetable has no deadline left, at its first step when
/// both first deadlines are out of range: nothing more comes from it, so it no longer holds the
/// observer (decision 0004).
fn setup_timer<M, E, G, OR, D, S>(
    context: &Context<M, G, G::Event, E, OR, D>,
    scheduler: S,
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
        let updated = update_groups(context, |model, events| {
            let (transition, next) = model.timetable.step(now);
            let _ = match transition {
                Some(Transition::Close) => model.groups.close_oldest(events),
                Some(Transition::Open) => model.groups.open(events),
                // A group that closes as the next opens closes first.
                Some(Transition::Rotate) => model.groups.rotate(events),
                None => Flow::Continue,
            };
            next
        });
        match updated {
            Updated::Ran {
                result,
                flow: Flow::Continue,
            } => result,
            _ => Next::Finished,
        }
    });
    scheduler.run_task(task, None)
}
