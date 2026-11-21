//! The groups of the window operators: [`Windows`], whose every change is an [`Action`] for the
//! [`DelegateObserver`], which holds the open windows and the observer of the windows, and acts on
//! the actions outside any lock (decision 0014).
//!
//! A window is a subject whose subscriber is user code, so it cannot be fed under the lock of the
//! machine that decides when the windows open and close: the machine's state holds nothing of
//! them, and its events are the actions, in the order it decided them.
//!
//! The delegate keeps its windows in an [`Overlap`]: an operator with a shift or a skip keeps
//! several open at once, the others one at a time, with
//! [`no_overlap`](super::overlap::no_overlap) as the duplicate.

use super::{group::Groups, overlap::Overlap};
use crate::{
    observer::{Flow, Observer, Termination, boxed_observer::ObserverMode},
    subject::unicast_subject::{self, BoxedUnicastObservable, BoxedUnicastSender},
    utils::MarkerType,
};
use std::marker::PhantomData;

/// What the [`Windows`] of a machine ask the [`DelegateObserver`] to do, in the order the machine
/// decided it.
#[doc(hidden)]
pub enum Action<T, E> {
    /// Sends a source value to every open window, if there is one.
    ForwardValue(T),
    /// Completes the oldest open window, if there is one.
    CompleteOldest,
    /// Terminates every open window.
    TerminateAll(Termination<E>),
    /// Opens a new window, the newest, and emits it downstream.
    EmitWindow,
}

/// The groups of a window operator, which a machine drives: every change becomes an [`Action`],
/// so the machine keeps no state for them.
#[doc(hidden)]
pub struct Windows<T>(MarkerType<T>);

impl<T> Windows<T> {
    /// The windows of a machine.
    pub(crate) fn new() -> Self {
        Self(PhantomData)
    }
}

impl<T, E: Clone> Groups<E> for Windows<T> {
    type Item = T;
    type Event = Action<T, E>;

    fn open(&mut self, observer: &mut impl Observer<Action<T, E>, E>) -> Flow {
        observer.on_next(Action::EmitWindow)
    }

    #[inline]
    fn put(
        &mut self,
        value: &mut Option<T>,
        observer: &mut impl Observer<Action<T, E>, E>,
    ) -> Flow {
        match value.take() {
            Some(value) => observer.on_next(Action::ForwardValue(value)),
            None => Flow::Continue,
        }
    }

    fn close_oldest(&mut self, observer: &mut impl Observer<Action<T, E>, E>) -> Flow {
        observer.on_next(Action::CompleteOldest)
    }

    fn end(
        &mut self,
        termination: &Termination<E>,
        observer: &mut impl Observer<Action<T, E>, E>,
    ) -> Flow {
        observer.on_next(Action::TerminateAll(termination.clone()))
    }
}

/// Owns the open windows and the observer of the windows, so that a window is fed and emitted
/// outside the lock of the machine that decides when the windows open and close.
#[doc(hidden)]
pub struct DelegateObserver<'a, M: ObserverMode, T, E, OR> {
    outer_observer: OR,
    /// The sending ends of the windows that are open, which are the only place where the windows
    /// are fed from. None is open before the first window opens and after the last one ended, nor,
    /// with a skip or a shift larger than a window, between two windows.
    windows: Overlap<BoxedUnicastSender<'a, T, E, M>, T>,
}

impl<M: ObserverMode, T, E, OR> DelegateObserver<'_, M, T, E, OR> {
    /// Creates the delegate, with no window open yet. `duplicate` clones a value for every open
    /// window but the newest.
    pub(crate) fn new(outer_observer: OR, duplicate: fn(&T) -> T) -> Self {
        Self {
            outer_observer,
            windows: Overlap::new(duplicate),
        }
    }
}

impl<'a, M, T, E, OR> Observer<Action<T, E>, E> for DelegateObserver<'a, M, T, E, OR>
where
    M: ObserverMode,
    E: Clone,
    OR: Observer<BoxedUnicastObservable<'a, T, E, M>, E>,
{
    fn on_next(&mut self, action: Action<T, E>) -> Flow {
        match action {
            Action::ForwardValue(value) => {
                // A value no window accepts has nowhere to go: it is dropped here, outside the
                // lock. Only the observer of the windows can stop the window operator.
                self.windows.send(&mut Some(value), |sender, value| {
                    let _ = sender.on_next(value);
                });
                Flow::Continue
            }
            Action::CompleteOldest => {
                if let Some(sender) = self.windows.close_oldest() {
                    sender.on_termination(Termination::Completed);
                }
                Flow::Continue
            }
            Action::TerminateAll(termination) => {
                // The last window takes the termination itself: one clone fewer.
                let mut senders = self.windows.close_all();
                if let Some(last) = senders.next_back() {
                    for sender in senders {
                        sender.on_termination(termination.clone());
                    }
                    last.on_termination(termination);
                }
                Flow::Continue
            }
            Action::EmitWindow => {
                let (sender, window) = unicast_subject::new_boxed();
                self.windows.open(sender);
                self.outer_observer.on_next(window)
            }
        }
    }

    fn on_termination(self, termination: Termination<E>) {
        // The open windows are terminated by the `TerminateAll` action that precedes this one.
        debug_assert!(self.windows.is_empty());
        self.outer_observer.on_termination(termination);
    }
}
