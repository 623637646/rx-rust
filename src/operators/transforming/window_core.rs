//! The machinery shared by [`Window`](super::window::Window),
//! [`WindowWithTime`](super::window_with_time::WindowWithTime) and
//! [`WindowWithTimeOrCount`](super::window_with_time_or_count::WindowWithTimeOrCount): each
//! serializes its source against what rotates its windows — a boundary, a timer, or a timer and
//! the count — through one context, whose events are the [`Action`]s of this module. The
//! [`DelegateObserver`] the context delivers them to acts on them outside the lock: it holds the
//! open window and the observer of the windows.
//!
//! [`WindowWithCount`](super::window_with_count::WindowWithCount) has a single source and nothing
//! else rotating its windows, so it needs no context and feeds its windows itself.

use crate::{
    disposable::Disposable,
    observer::{EventBatch, Flow, Observer, Termination, boxed_observer::ObserverMode},
    subject::unicast_subject::{self, BoxedUnicastObservable, BoxedUnicastSender},
    utils::subscribe_with_context::SubscriptionContext,
};

/// The context of a window operator whose model is `MD` and whose sources are disposed by `D`.
pub(super) type WindowContext<'a, M, T, E, OR, MD, D> =
    SubscriptionContext<M, Action<T, E>, E, DelegateObserver<'a, M, T, E, OR>, MD, D>;

/// What the observers of a window operator ask the [`DelegateObserver`] to do, in the order they
/// decided it under the lock.
pub(super) enum Action<T, E> {
    /// Sends a source value to the current window, if there is one.
    ForwardValue(T),
    /// Terminates the current window, if there is one.
    TerminateWindow(Termination<E>),
    /// Opens a new window, makes it the current one and emits it downstream.
    EmitWindow,
}

impl<T, E> Action<T, E> {
    /// Completes the current window and opens the next one: two events rather than one, so that
    /// disposing the outer subscription while the current window is completing suppresses the new
    /// window.
    pub(super) fn rotation() -> [Self; 2] {
        [
            Self::TerminateWindow(Termination::Completed),
            Self::EmitWindow,
        ]
    }
}

/// Rotates the windows with a send of its own, for a rotation that changes no model: a boundary of
/// `window`, a tick of `window_with_time`. A rotation that does, such as one of
/// `window_with_time_or_count`, queues [`Action::rotation`] from its update instead.
pub(super) fn rotate<'a, M, T, E, OR, MD, D>(
    context: &WindowContext<'a, M, T, E, OR, MD, D>,
) -> Flow
where
    M: ObserverMode,
    OR: Observer<BoxedUnicastObservable<'a, T, E, M>, E>,
    D: Disposable,
{
    context.send(EventBatch::NextBatch(Action::rotation().into()))
}

/// Terminates the current window, then the outer observable.
pub(super) fn terminate<'a, M, T, E, OR, MD, D>(
    context: WindowContext<'a, M, T, E, OR, MD, D>,
    termination: Termination<E>,
) where
    M: ObserverMode,
    E: Clone,
    OR: Observer<BoxedUnicastObservable<'a, T, E, M>, E>,
    D: Disposable,
{
    let _ = context.send(EventBatch::NextAndTermination(
        Action::TerminateWindow(termination.clone()),
        termination,
    ));
}

/// Owns the state that the actions act on, so that a window is fed and emitted outside the lock of
/// the context that serializes the source against what rotates the windows.
pub(super) struct DelegateObserver<'a, M: ObserverMode, T, E, OR> {
    outer_observer: OR,
    /// The sending end of the window that is currently open, which is the only place where the
    /// windows are fed from. It is `None` until the first window opens and after the last window
    /// ended.
    sender: Option<BoxedUnicastSender<'a, T, E, M>>,
}

impl<M: ObserverMode, T, E, OR> DelegateObserver<'_, M, T, E, OR> {
    /// Creates the delegate, with no window open yet.
    pub(super) fn new(outer_observer: OR) -> Self {
        Self {
            outer_observer,
            sender: None,
        }
    }
}

impl<'a, M, T, E, OR> Observer<Action<T, E>, E> for DelegateObserver<'a, M, T, E, OR>
where
    M: ObserverMode,
    OR: Observer<BoxedUnicastObservable<'a, T, E, M>, E>,
{
    fn on_next(&mut self, action: Action<T, E>) -> Flow {
        match action {
            Action::ForwardValue(value) => {
                match &mut self.sender {
                    // The consumer of one window stops that window, not the operator: the next
                    // window has its own consumer.
                    Some(sender) => {
                        let _ = sender.on_next(value);
                    }
                    // No window accepts values, so this one has nowhere to go.
                    None => drop(value),
                }
                Flow::Continue
            }
            Action::TerminateWindow(termination) => {
                if let Some(sender) = self.sender.take() {
                    sender.on_termination(termination);
                }
                Flow::Continue
            }
            Action::EmitWindow => {
                debug_assert!(self.sender.is_none());
                let (sender, window) = unicast_subject::new_boxed();
                self.sender = Some(sender);
                self.outer_observer.on_next(window)
            }
        }
    }

    fn on_termination(self, termination: Termination<E>) {
        // The current window is terminated by the `TerminateWindow` action that precedes this one.
        debug_assert!(self.sender.is_none());
        self.outer_observer.on_termination(termination);
    }
}
