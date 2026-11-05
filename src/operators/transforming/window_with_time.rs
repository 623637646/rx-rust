//! The [`WindowWithTime`] operator, behind
//! [`ObservableExt::window_with_time`](crate::observable::ObservableExt::window_with_time).

use super::window_core::{Action, DelegateObserver, WindowContext, rotate, terminate};
use crate::{
    delegate_disposal,
    disposable::{Disposable, chain_disposal::ChainDisposal, dispose_on_drop::DisposeOnDrop},
    observable::{ChainSchedulerDisposalOf, JoinedSchedulerModeOf, Observable, ObservableTypes},
    observer::{Flow, Observer, Termination, boxed_observer::ObserverMode},
    scheduler::{Clock, PeriodicContext, Scheduler, SchedulerTypes, Task},
    subject::unicast_subject::BoxedUnicastObservable,
    thread_mode::ThreadMode,
    utils::{
        MarkerType,
        subscribe_with_context::{self, subscribe_with_context_two_steps},
    },
};
use educe::Educe;
use std::{
    marker::PhantomData,
    time::{Duration, Instant},
};

/// Subdivides the items of an observable into observable windows, a new one every `time_span`.
/// See <https://reactivex.io/documentation/operators/window.html>
///
/// The first window opens at the subscription and is completed one `time_span` later, as
/// ReactiveX does; every following one opens as the one before is completed, and lasts
/// `time_span` too, at a fixed rate, empty or not. A late timer catches up: the rotations it missed
/// follow back to back, with empty windows, keeping the original rate. A termination of the source
/// terminates the current window, then the outer observable.
///
/// A time too far out for an `Instant` to represent never comes: with a `time_span` that long, the
/// first window stays open until the source terminates.
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
///     operators::transforming::window_with_time::WindowWithTime,
///     scheduler::virtual_time::VirtualTime,
///     subject::publish_subject::PublishSubject,
///     thread_mode::mutable::{MutableExt, MutableHelper},
/// };
/// use std::{convert::Infallible, sync::{Arc, Mutex}, time::Duration};
///
/// let time = VirtualTime::new();
/// let span = Duration::from_millis(5);
/// let windows = Arc::new(Mutex::new(Vec::new()));
/// let windows_observer = windows.clone();
/// let mut source = PublishSubject::<_, Infallible, _>::shared();
/// let _subscription = WindowWithTime::new(source.clone(), span, time.scheduler())
///     .concat_map(|window| window.to_vec())
///     .subscribe_with_callback(
///         move |values| windows_observer.with_mut(|windows| windows.push(values)),
///         |_| {},
///     );
///
/// let _ = source.on_next(1);
/// let _ = source.on_next(2);
/// time.advance_by(span); // Completes the first window and opens the second.
/// let _ = source.on_next(3);
/// assert_eq!(windows.clone_value(), [vec![1, 2]]);
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct WindowWithTime<'a, OE, S> {
    source: OE,
    time_span: Duration,
    scheduler: S,
    /// The observer of a window may borrow for `'a`. Unlike the `_boxed` hooks this cannot be left
    /// to an unboxed default: the window is the `Item`, which [`ObservableTypes`] names without any
    /// observer, and its observer arrives only later, so the window boxes it.
    _marker: MarkerType<&'a ()>,
}

impl<OE, S> WindowWithTime<'_, OE, S> {
    /// Creates a [`WindowWithTime`] over `source`;
    /// [`ObservableExt::window_with_time`](crate::observable::ObservableExt::window_with_time) is
    /// the fluent form.
    pub fn new(source: OE, time_span: Duration, scheduler: S) -> Self {
        Self {
            source,
            time_span,
            scheduler,
            _marker: PhantomData,
        }
    }
}

/// The task of a [`WindowWithTime`] timer. It holds the context strongly, so that the windows keep
/// being rotated after the source has let go of its observer without a termination, until the
/// subscription is disposed. A disposal still releases the observer at once: disposing the source
/// makes it drop its own handle, and a handle dropped once the context has stopped releases the
/// observer.
type WindowWithTimeTask<'a, T, E, OR, OE, S> = PeriodicContext<
    WindowContext<'a, JoinedSchedulerModeOf<OE, S>, T, E, OR, (), ChainSchedulerDisposalOf<S, OE>>,
>;

delegate_disposal!(
    Disposal<M, T, E, SD, D>,
    subscribe_with_context::Disposal<M, Action<T, E>, E, (), ChainDisposal<SD, D>>,
    where M: ThreadMode, SD: Disposable, D: Disposable
);

impl<'a, T, E, OE, S> ObservableTypes for WindowWithTime<'a, OE, S>
where
    JoinedSchedulerModeOf<OE, S>: ObserverMode,
    OE: ObservableTypes<Item = T, Error = E>,
    S: SchedulerTypes,
{
    type Item = BoxedUnicastObservable<'a, T, E, JoinedSchedulerModeOf<OE, S>>;
    type Error = E;
    /// The timer's thread rotates the windows, the source's feeds them and ends the last one, so
    /// the mode is both joined.
    type Mode = JoinedSchedulerModeOf<OE, S>;
    type Disposal = Disposal<JoinedSchedulerModeOf<OE, S>, T, E, S::Disposal, OE::Disposal>;
}

impl<'a, T, E, OE, S, OR> Observable<OR> for WindowWithTime<'a, OE, S>
where
    JoinedSchedulerModeOf<OE, S>: ObserverMode,
    OR: Observer<BoxedUnicastObservable<'a, T, E, JoinedSchedulerModeOf<OE, S>>, E>,
    E: Clone,
    OE: Observable<
            WindowWithTimeObserver<
                'a,
                JoinedSchedulerModeOf<OE, S>,
                T,
                E,
                OR,
                ChainSchedulerDisposalOf<S, OE>,
            >,
            Item = T,
            Error = E,
        >,
    S: Scheduler<WindowWithTimeTask<'a, T, E, OR, OE, S>>,
{
    fn subscribe(self, observer: OR) -> DisposeOnDrop<Self::Disposal> {
        assert!(!self.time_span.is_zero(), "time_span must be non-zero");
        let deadline = self.scheduler.clock().now().checked_add(self.time_span);
        // The windows own their buffered items, so the context needs no model of its own: it only
        // serializes the source against the timer.
        subscribe_with_context_two_steps(
            DelegateObserver::new(observer),
            (),
            |context| {
                // Open the first window before subscribing, so synchronous values have somewhere
                // to go. An observer that stops on it keeps the source and timer from starting.
                if context.send_next(Action::EmitWindow).is_stop() {
                    return None;
                }
                Some(
                    self.source
                        .subscribe(WindowWithTimeObserver(context.clone())),
                )
            },
            |context| {
                Some(setup_rotation_timer(
                    context,
                    self.scheduler,
                    self.time_span,
                    deadline,
                ))
            },
        )
        .map_inner_into()
    }
}

#[doc(hidden)]
pub struct WindowWithTimeObserver<'a, M: ObserverMode, T, E, OR, D: Disposable>(
    WindowContext<'a, M, T, E, OR, (), D>,
);

impl<'a, M, T, E, OR, D> Observer<T, E> for WindowWithTimeObserver<'a, M, T, E, OR, D>
where
    M: ObserverMode,
    E: Clone,
    OR: Observer<BoxedUnicastObservable<'a, T, E, M>, E>,
    D: Disposable,
{
    fn on_next(&mut self, value: T) -> Flow {
        // The value is queued as an action, so that it reaches the window outside the lock.
        self.0.send_next(Action::ForwardValue(value))
    }

    fn on_termination(self, termination: Termination<E>) {
        terminate(self.0, termination);
    }
}

fn setup_rotation_timer<'a, M, T, E, OR, D, S>(
    context: &WindowContext<'a, M, T, E, OR, (), D>,
    scheduler: S,
    time_span: Duration,
    deadline: Option<Instant>,
) -> DisposeOnDrop<S::Disposal>
where
    M: ObserverMode,
    OR: Observer<BoxedUnicastObservable<'a, T, E, M>, E>,
    D: Disposable,
    S: Scheduler<PeriodicContext<WindowContext<'a, M, T, E, OR, (), D>>>,
{
    // The context owns only the task's disposal, through its source subscription, never the task,
    // which its runtime owns: holding the context strongly forms no cycle. Stopping the context
    // disposes its source subscription, which cancels the task.
    let task = Task::periodic(
        context.clone(),
        // A stopped context, or an observer that stopped on the new window, ends the rotation.
        |context, _| !rotate(context).is_stop(),
        time_span,
        deadline,
    );
    // The task waits for its absolute anchor, even if subscribing or its first poll ran late.
    // An unrepresentable deadline keeps the task parked until disposal.
    let delay = deadline.is_none().then_some(time_span);
    scheduler.run_task(task, delay)
}
