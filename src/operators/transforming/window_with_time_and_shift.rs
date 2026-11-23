//! The [`WindowWithTimeAndShift`] operator, behind
//! [`ObservableExt::window_with_time_and_shift`](crate::observable::ObservableExt::window_with_time_and_shift).

use super::internal::time_shift::{self, TimeShiftTask};
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
use std::{marker::PhantomData, time::Duration};

#[doc(hidden)]
pub use super::internal::{
    time_shift::TimeShiftObserver,
    window::{Action, DelegateObserver, Windows},
};

/// Subdivides the items of an observable into observable windows that each last `time_span`,
/// opening a new window every `time_shift`.
/// See <https://reactivex.io/documentation/operators/window.html>
///
/// The first window opens at subscription. Later openings are spaced `time_shift` apart on the
/// scheduler's clock. A window completes `time_span` after opening, even when empty. Each item
/// goes to every open window. When windows overlap, the newest takes the original item and
/// earlier windows receive clones. When `time_shift` exceeds `time_span`, items that arrive in
/// the gaps are dropped. If a close and an open coincide, the oldest window completes first. Use
/// [`WindowWithTime`](super::window_with_time::WindowWithTime) when the span and shift are equal
/// and the items lack `Clone`.
///
/// A termination of the source terminates every window still open, then the outer observable.
/// A late timer processes overdue openings and closings in their scheduled order.
///
/// Deadlines outside the range of `Instant` never fire. A window with an out-of-range closing
/// deadline stays open while the source and outer subscription remain active.
///
/// Each window is a single-consumer pipe: it can be subscribed to once, it buffers the items that
/// arrive while it has no subscriber, and dropping it without subscribing discards its items. A
/// window is serialized on its own rather than together with the outer observable, so the events
/// of a window keep their order among themselves, but they are not ordered against the emission of
/// a later window. Disposing the outer subscription ends the open windows without a termination:
/// what they had buffered is still delivered to their subscribers, even later ones, which are then
/// dropped without being notified.
///
/// Disposing the subscription of a single window does not necessarily release its observer where
/// it happens: the window releases it on its next item, when it ends, or when the outer
/// subscription is disposed, whichever comes first. See
/// [the unicast subject](crate::subject::unicast_subject) each window is built on.
///
/// # Panics
///
/// Subscribing panics if `time_span` or `time_shift` is zero.
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt,
///     observer::Observer,
///     operators::transforming::window_with_time_and_shift::WindowWithTimeAndShift,
///     scheduler::virtual_time::VirtualTime,
///     subject::publish_subject::PublishSubject,
///     thread_mode::mutable::{MutableExt, MutableHelper},
/// };
/// use std::{convert::Infallible, sync::{Arc, Mutex}, time::Duration};
///
/// let time = VirtualTime::new();
/// let shift = Duration::from_millis(5);
/// let windows = Arc::new(Mutex::new(Vec::new()));
/// let windows_observer = windows.clone();
/// let mut source = PublishSubject::<_, Infallible, _>::shared();
/// let _subscription =
///     WindowWithTimeAndShift::new(source.clone(), shift * 2, shift, time.scheduler())
///         .flat_map(|window| window.to_vec())
///         .subscribe_with_callback(
///             move |values| windows_observer.with_mut(|windows| windows.push(values)),
///             |_| {},
///         );
///
/// let _ = source.on_next(1);
/// time.advance_by(shift); // Opens the second window.
/// let _ = source.on_next(2);
/// time.advance_by(shift); // Completes the first window and opens the third.
/// assert_eq!(windows.clone_value(), [vec![1, 2]]);
/// let _ = source.on_next(3);
/// time.advance_by(shift);
/// assert_eq!(windows.clone_value(), [vec![1, 2], vec![2, 3]]);
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct WindowWithTimeAndShift<'a, OE, S> {
    source: OE,
    time_span: Duration,
    time_shift: Duration,
    scheduler: S,
    /// The observer of a window may borrow for `'a`. Unlike the `_boxed` hooks this cannot be left
    /// to an unboxed default: the window is the `Item`, which [`ObservableTypes`] names without any
    /// observer, and its observer arrives only later, so the window boxes it.
    _marker: MarkerType<&'a ()>,
}

impl<OE, S> WindowWithTimeAndShift<'_, OE, S> {
    /// Creates a [`WindowWithTimeAndShift`] over `source`;
    /// [`ObservableExt::window_with_time_and_shift`](crate::observable::ObservableExt::window_with_time_and_shift)
    /// is the fluent form.
    pub fn new(source: OE, time_span: Duration, time_shift: Duration, scheduler: S) -> Self {
        Self {
            source,
            time_span,
            time_shift,
            scheduler,
            _marker: PhantomData,
        }
    }
}

/// The observer a [`WindowWithTimeAndShift`] gives its source.
#[doc(hidden)]
pub type WindowWithTimeAndShiftObserver<'a, M, T, E, OR, D> =
    TimeShiftObserver<M, Windows<T>, Action<T, E>, E, DelegateObserver<'a, M, T, E, OR>, D>;

/// The task of a [`WindowWithTimeAndShift`] timer. It holds the context strongly, so that the
/// windows keep being opened and completed after the source has let go of its observer without a
/// termination, until the subscription is disposed.
pub(super) type WindowWithTimeAndShiftTask<'a, T, E, OR, OE, S> = TimeShiftTask<
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
        time_shift::Model<Windows<T>>,
        ChainDisposal<SD, D>,
    >,
    where M: ThreadMode, SD: Disposable, D: Disposable
);

impl<'a, T, E, OE, S> ObservableTypes for WindowWithTimeAndShift<'a, OE, S>
where
    JoinedSchedulerModeOf<OE, S>: ObserverMode,
    OE: ObservableTypes<Item = T, Error = E>,
    S: SchedulerTypes,
{
    type Item = BoxedUnicastObservable<'a, T, E, JoinedSchedulerModeOf<OE, S>>;
    type Error = E;
    /// The timer's thread opens and completes the windows, the source's feeds them and ends the
    /// last ones, so the mode is both joined.
    type Mode = JoinedSchedulerModeOf<OE, S>;
    type Disposal = Disposal<JoinedSchedulerModeOf<OE, S>, T, E, S::Disposal, OE::Disposal>;
}

impl<'a, T, E, OE, S, OR> Observable<OR> for WindowWithTimeAndShift<'a, OE, S>
where
    JoinedSchedulerModeOf<OE, S>: ObserverMode,
    OR: Observer<BoxedUnicastObservable<'a, T, E, JoinedSchedulerModeOf<OE, S>>, E>,
    E: Clone,
    OE: Observable<
            WindowWithTimeAndShiftObserver<
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
    S: Scheduler<WindowWithTimeAndShiftTask<'a, T, E, OR, OE, S>>,
    T: Clone,
{
    fn subscribe(self, observer: OR) -> DisposeOnDrop<Self::Disposal> {
        time_shift::subscribe(
            self.source,
            self.time_span,
            self.time_shift,
            self.scheduler,
            Windows::new(),
            DelegateObserver::new(observer, T::clone),
        )
        .map_inner_into()
    }
}
