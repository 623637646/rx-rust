//! The [`WindowWithTimeOrCount`] operator, behind
//! [`ObservableExt::window_with_time_or_count`](crate::observable::ObservableExt::window_with_time_or_count).

use super::internal::{
    overlap::no_overlap,
    time_or_count::{self, TimeOrCountTask},
};
use crate::{
    delegate_disposal,
    disposable::{Disposable, chain_disposal::ChainDisposal, dispose_on_drop::DisposeOnDrop},
    observable::{ChainSchedulerDisposalOf, JoinedSchedulerModeOf, Observable, ObservableTypes},
    observer::{Observer, boxed_observer::ObserverMode},
    scheduler::{Scheduler, SchedulerTypes},
    subject::unicast_subject::BoxedUnicastObservable,
    thread_mode::ThreadMode,
    utils::{MarkerType, subscribe_with_context},
};
use educe::Educe;
use std::{marker::PhantomData, num::NonZeroUsize, time::Duration};

#[doc(hidden)]
pub use super::internal::{
    time_or_count::TimeOrCountObserver,
    window::{Action, DelegateObserver, Windows},
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

/// The observer a [`WindowWithTimeOrCount`] gives its source.
#[doc(hidden)]
pub type WindowWithTimeOrCountObserver<'a, M, T, E, OR, D, C> =
    TimeOrCountObserver<M, Windows<T>, Action<T, E>, E, DelegateObserver<'a, M, T, E, OR>, D, C>;

/// The task of a [`WindowWithTimeOrCount`] timer. It holds the context strongly, so that the
/// windows keep being rotated after the source has let go of its observer without a termination,
/// until the subscription is disposed.
type WindowWithTimeOrCountTask<'a, T, E, OR, OE, S> = TimeOrCountTask<
    OE,
    S,
    Windows<T>,
    Action<T, E>,
    E,
    DelegateObserver<'a, JoinedSchedulerModeOf<OE, S>, T, E, OR>,
>;

delegate_disposal!(
    Disposal<M, T, E, SD, D>,
    subscribe_with_context::Disposal<
        M,
        Action<T, E>,
        E,
        time_or_count::Model<Windows<T>>,
        ChainDisposal<SD, D>,
    >,
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
        // One window at a time, so no item is cloned.
        time_or_count::subscribe(
            self.source,
            self.count,
            self.time_span,
            self.scheduler,
            Windows::new(),
            DelegateObserver::new(observer, no_overlap),
        )
        .map_inner_into()
    }
}
