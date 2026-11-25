//! Divides the items into observables, one per time span.
//!
//! The [`WindowWithTime`] operator, behind
//! [`ObservableExt::window_with_time`](crate::observable::ObservableExt::window_with_time).

use super::{
    internal::{
        overlap::no_overlap,
        time_shift,
        window::{DelegateObserver, Windows},
    },
    window_with_time_and_shift::{
        self, WindowWithTimeAndShiftObserver, WindowWithTimeAndShiftTask,
    },
};
use crate::{
    disposable::dispose_on_drop::DisposeOnDrop,
    observable::{ChainSchedulerDisposalOf, JoinedSchedulerModeOf, Observable, ObservableTypes},
    observer::{Observer, boxed_observer::ObserverMode},
    scheduler::{Scheduler, SchedulerTypes},
    subject::unicast_subject::BoxedUnicastObservable,
    utils::MarkerType,
};
use educe::Educe;
use std::{marker::PhantomData, time::Duration};

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
    type Disposal = window_with_time_and_shift::Disposal<
        JoinedSchedulerModeOf<OE, S>,
        T,
        E,
        S::Disposal,
        OE::Disposal,
    >;
}

impl<'a, T, E, OE, S, OR> Observable<OR> for WindowWithTime<'a, OE, S>
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
{
    fn subscribe(self, observer: OR) -> DisposeOnDrop<Self::Disposal> {
        // A `WindowWithTimeAndShift` whose windows open as the one before is completed: a single
        // one is ever open, so no item is cloned (decision 0014).
        time_shift::subscribe(
            self.source,
            self.time_span,
            self.time_span,
            self.scheduler,
            Windows::new(),
            DelegateObserver::new(observer, no_overlap),
        )
        .map_inner_into()
    }
}
