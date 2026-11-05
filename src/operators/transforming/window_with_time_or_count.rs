//! The [`WindowWithTimeOrCount`] operator, behind
//! [`ObservableExt::window_with_time_or_count`](crate::observable::ObservableExt::window_with_time_or_count).

use super::window_core::{Action, DelegateObserver, WindowContext, terminate};
use crate::{
    delegate_disposal,
    disposable::{Disposable, chain_disposal::ChainDisposal, dispose_on_drop::DisposeOnDrop},
    observable::{ChainSchedulerDisposalOf, JoinedSchedulerModeOf, Observable, ObservableTypes},
    observer::{EventBatch, Flow, Observer, Termination, boxed_observer::ObserverMode},
    scheduler::{Clock, Next, Scheduler, SchedulerTypes, Task},
    subject::unicast_subject::BoxedUnicastObservable,
    thread_mode::ThreadMode,
    utils::{
        MarkerType,
        serialized_delivery::UpdateOutcome,
        subscribe_with_context::{self, subscribe_with_context_two_steps},
    },
};
use educe::Educe;
use std::{
    marker::PhantomData,
    num::NonZeroUsize,
    time::{Duration, Instant},
};

/// Subdivides the items of an observable into observable windows, a new one either when the
/// current window reaches `count` items or every `time_span`, whichever happens first.
/// See <https://reactivex.io/documentation/operators/window.html>
///
/// The first window opens at the subscription, and the timer first fires one `time_span` later, as
/// ReactiveX does. A window completed because it is full opens the next one at once and restarts
/// the timer, so the next timed rotation comes `time_span` after it. A late timer catches up: the
/// rotations it missed follow back to back, with empty windows, unless a full window reset it. A
/// termination of the source terminates the current window, then the outer observable, so a
/// completion right after a full window completes the empty window opened after it.
///
/// A time too far out for an `Instant` to represent never comes: with a `time_span` that long, the
/// windows are rotated by count only.
///
/// Each window is a single-consumer pipe: it can be subscribed to once, it buffers the items that
/// arrive while it has no subscriber, and dropping it without subscribing discards its items. A
/// window is serialized on its own rather than together with the outer observable, so the events
/// of a window keep their order among themselves, but they are not ordered against the emission of
/// a later window. Disposing the outer subscription ends the open window without a termination:
/// what it had buffered is still delivered to its subscriber, even a later one, which is then
/// dropped without being notified.
///
/// Disposing the subscription of a single window does not necessarily release its observer where
/// it happens: the window releases it on its next item, when it ends, or when the outer
/// subscription is disposed, whichever comes first. See
/// [the unicast subject](crate::subject::unicast_subject) each window is built on.
///
/// # Panics
///
/// Subscribing panics if `time_span` is zero.
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt,
///     observer::Observer,
///     operators::transforming::window_with_time_or_count::WindowWithTimeOrCount,
///     scheduler::virtual_time::VirtualTime,
///     subject::publish_subject::PublishSubject,
///     thread_mode::mutable::{MutableExt, MutableHelper},
/// };
/// use std::{convert::Infallible, num::NonZeroUsize, sync::{Arc, Mutex}, time::Duration};
///
/// let time = VirtualTime::new();
/// let span = Duration::from_millis(5);
/// let windows = Arc::new(Mutex::new(Vec::new()));
/// let windows_observer = windows.clone();
/// let mut source = PublishSubject::<_, Infallible, _>::shared();
/// let _subscription = WindowWithTimeOrCount::new(
///     source.clone(), NonZeroUsize::new(2).unwrap(), span, time.scheduler(),
/// )
/// .concat_map(|window| window.to_vec())
/// .subscribe_with_callback(
///     move |values| windows_observer.with_mut(|windows| windows.push(values)),
///     |_| {},
/// );
///
/// let _ = source.on_next(1);
/// let _ = source.on_next(2); // Full: completed at once.
/// let _ = source.on_next(3);
/// assert_eq!(windows.clone_value(), [vec![1, 2]]);
/// time.advance_by(span);
/// assert_eq!(windows.clone_value(), [vec![1, 2], vec![3]]);
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct WindowWithTimeOrCount<'a, OE, S> {
    source: OE,
    count: NonZeroUsize,
    time_span: Duration,
    scheduler: S,
    /// The observer of a window may borrow for `'a`. Unlike the `_boxed` hooks this cannot be left
    /// to an unboxed default: the window is the `Item`, which [`ObservableTypes`] names without any
    /// observer, and its observer arrives only later, so the window boxes it.
    _marker: MarkerType<&'a ()>,
}

impl<OE, S> WindowWithTimeOrCount<'_, OE, S> {
    /// Creates a [`WindowWithTimeOrCount`] over `source`;
    /// [`ObservableExt::window_with_time_or_count`](crate::observable::ObservableExt::window_with_time_or_count)
    /// is the fluent form.
    pub fn new(source: OE, count: NonZeroUsize, time_span: Duration, scheduler: S) -> Self {
        Self {
            source,
            count,
            time_span,
            scheduler,
            _marker: PhantomData,
        }
    }
}

/// The task of a [`WindowWithTimeOrCount`] timer. It holds the context strongly, so that the
/// windows keep being rotated after the source has let go of its observer without a termination,
/// until the subscription is disposed. A disposal still releases the observer at once: disposing
/// the source makes it drop its own handle, and a handle dropped once the context has stopped
/// releases the observer.
type WindowWithTimeOrCountTask<'a, T, E, OR, OE, S> = WindowContext<
    'a,
    JoinedSchedulerModeOf<OE, S>,
    T,
    E,
    OR,
    Model,
    ChainSchedulerDisposalOf<S, OE>,
>;

delegate_disposal!(
    Disposal<M, T, E, SD, D>,
    subscribe_with_context::Disposal<M, Action<T, E>, E, Model, ChainDisposal<SD, D>>,
    where M: ThreadMode, SD: Disposable, D: Disposable
);

impl<'a, T, E, OE, S> ObservableTypes for WindowWithTimeOrCount<'a, OE, S>
where
    JoinedSchedulerModeOf<OE, S>: ObserverMode,
    OE: ObservableTypes<Item = T, Error = E>,
    S: SchedulerTypes,
{
    type Item = BoxedUnicastObservable<'a, T, E, JoinedSchedulerModeOf<OE, S>>;
    type Error = E;
    /// The timer's thread and the source's both rotate the windows, so the mode is both joined.
    type Mode = JoinedSchedulerModeOf<OE, S>;
    type Disposal = Disposal<JoinedSchedulerModeOf<OE, S>, T, E, S::Disposal, OE::Disposal>;
}

impl<'a, T, E, OE, S, OR> Observable<OR> for WindowWithTimeOrCount<'a, OE, S>
where
    JoinedSchedulerModeOf<OE, S>: ObserverMode,
    OR: Observer<BoxedUnicastObservable<'a, T, E, JoinedSchedulerModeOf<OE, S>>, E>,
    E: Clone,
    OE: Observable<
            WindowWithTimeOrCountObserver<
                'a,
                JoinedSchedulerModeOf<OE, S>,
                T,
                E,
                OR,
                ChainSchedulerDisposalOf<S, OE>,
                <S as SchedulerTypes>::Clock,
            >,
            Item = T,
            Error = E,
        >,
    S: Scheduler<WindowWithTimeOrCountTask<'a, T, E, OR, OE, S>>,
{
    fn subscribe(self, observer: OR) -> DisposeOnDrop<Self::Disposal> {
        assert!(!self.time_span.is_zero(), "time_span must be non-zero");
        let model = Model {
            sent_count: 0,
            deadline: self.scheduler.clock().now().checked_add(self.time_span),
            time_span: self.time_span,
            count: self.count,
        };
        let clock = self.scheduler.clock();
        subscribe_with_context_two_steps(
            DelegateObserver::new(observer),
            model,
            |context| {
                // Open the first window before subscribing, so synchronous values have somewhere
                // to go. An observer that stops on it keeps the source and timer from starting.
                if context.send_next(Action::EmitWindow).is_stop() {
                    return None;
                }
                Some(self.source.subscribe(WindowWithTimeOrCountObserver {
                    context: context.clone(),
                    clock,
                }))
            },
            |context| setup_rotation_timer(context, self.scheduler, self.time_span),
        )
        .map_inner_into()
    }
}

/// The state of a [`WindowWithTimeOrCount`] subscription. The windows own their buffered items, so
/// the model only counts them.
struct Model {
    /// How many values the current window has received.
    sent_count: usize,
    /// When the next timed rotation is due: `time_span` after the subscription, then after the
    /// rotation before, timed or full. `None` when that is too far out for an [`Instant`] to
    /// represent: it never comes, nor does any later one, since the clock only moves forward, and
    /// the windows are rotated by count only.
    deadline: Option<Instant>,
    /// Fixed; here for the timer task, whose handler is a plain `fn`, and for the source's
    /// observer, which restarts the timer.
    time_span: Duration,
    count: NonZeroUsize,
}

/// The observer a [`WindowWithTimeOrCount`] gives its source. It holds the scheduler's clock rather
/// than the scheduler, which may be `Local` while the source sends from another thread.
#[doc(hidden)]
pub struct WindowWithTimeOrCountObserver<'a, M: ObserverMode, T, E, OR, D: Disposable, C> {
    context: WindowContext<'a, M, T, E, OR, Model, D>,
    clock: C,
}

impl<'a, M, T, E, OR, D, C> Observer<T, E> for WindowWithTimeOrCountObserver<'a, M, T, E, OR, D, C>
where
    M: ObserverMode,
    E: Clone,
    OR: Observer<BoxedUnicastObservable<'a, T, E, M>, E>,
    D: Disposable,
    C: Clock,
{
    fn on_next(&mut self, value: T) -> Flow {
        // The value is queued as an action, so that it reaches the window outside the lock.
        self.context
            .update(|model| {
                model.sent_count += 1;
                if model.sent_count < model.count.get() {
                    return UpdateOutcome::empty().with_next_event(Action::ForwardValue(value));
                }
                // This value fills the window: it is completed, and the next one opened. A full
                // window restarts the timer: the next timed rotation comes `time_span` after it,
                // which is never before the one already due, itself at most `time_span` after the
                // last tick. Out of range means never, as it already did for the deadline due.
                model.sent_count = 0;
                model.deadline = self.clock.now().checked_add(model.time_span);
                let [complete, open] = Action::rotation();
                UpdateOutcome::empty().with_events(EventBatch::NextBatch(vec![
                    Action::ForwardValue(value),
                    complete,
                    open,
                ]))
            })
            .flow()
    }

    fn on_termination(self, termination: Termination<E>) {
        terminate(self.context, termination);
    }
}

/// Drives the timed rotations with one long-lived scheduler task, as `buffer_with_time_or_count`
/// does.
///
/// A full window only moves `deadline` (see `WindowWithTimeOrCountObserver::on_next`): if the task
/// wakes at an obsolete deadline, it sleeps on to the current one. No scheduler task is spawned or
/// cancelled per full window, and no `Duration` is subtracted, so a tick that fires late can never
/// panic on underflow.
fn setup_rotation_timer<'a, M, T, E, OR, D, S>(
    context: &WindowContext<'a, M, T, E, OR, Model, D>,
    scheduler: S,
    time_span: Duration,
) -> Option<DisposeOnDrop<S::Disposal>>
where
    M: ObserverMode,
    OR: Observer<BoxedUnicastObservable<'a, T, E, M>, E>,
    D: Disposable,
    S: Scheduler<WindowContext<'a, M, T, E, OR, Model, D>>,
{
    // The source terminated synchronously while it was being subscribed: no window is left to
    // rotate.
    let deadline = context
        .update(|model| UpdateOutcome::new(model.deadline).without_events())
        .result()?;

    // The context owns only the task's disposal, through its source subscription, never the task,
    // which its runtime owns: holding the context strongly forms no cycle. Stopping the context
    // disposes its source subscription, which cancels the task.
    let task = Task::new(context.clone(), |context, _, _, now| {
        context
            .update(|model| match model.deadline {
                Some(deadline) if now < deadline => {
                    UpdateOutcome::new(Next::SleepUntil(deadline)).without_events()
                }
                Some(deadline) => {
                    // Fixed-rate: the next deadline is counted from this one, not from `now`, so
                    // that a late tick does not push the next ones back.
                    model.deadline = deadline.checked_add(model.time_span);
                    model.sent_count = 0;
                    let state = model.deadline.map_or(Next::Finished, Next::SleepUntil);
                    UpdateOutcome::new(state)
                        .with_events(EventBatch::NextBatch(Action::rotation().into()))
                }
                // A full window moved the deadline out of range: no timed rotation is left.
                None => UpdateOutcome::new(Next::Finished).without_events(),
            })
            .result()
            .unwrap_or(Next::Finished)
    });
    // Let the handler wait for the absolute deadline, rather than starting a relative delay
    // on its first poll. An unrepresentable deadline keeps the task parked until disposal.
    let delay = deadline.is_none().then_some(time_span);
    Some(scheduler.run_task(task, delay))
}
