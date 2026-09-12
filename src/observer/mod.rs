pub mod boxed_observer;
pub mod callback_observer;

use crate::{observer::boxed_observer::BoxedObserver, utils::types::MaybeSend};
use educe::Educe;

/// Represents the termination state of an operation, which can either be completed successfully or with an error.
#[derive(Educe)]
#[educe(Debug, Clone, PartialEq, Eq)]
pub enum Termination<E> {
    /// Indicates that the operation has completed successfully.
    Completed,
    /// Indicates that the operation has completed with an error.
    Error(E),
}

/// Whether an observer still accepts events, as reported by [`Observer::on_next`].
///
/// This is the only way an observer can tell the source that is pushing into it to stop, while
/// that push is running. It matters most for a synchronous source, which delivers its whole
/// sequence before `subscribe` returns: until it does, nobody holds the subscription yet, so
/// disposing it is not yet possible, and an infinite synchronous source would never end.
///
/// It does not replace disposal, and no source may rely on it alone: an observer can also go away
/// between two events, which no return value can report. See the variants for what each one
/// promises.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[must_use = "an upstream that ignores the flow keeps pushing into an observer that stopped"]
pub enum Flow {
    /// The observer is not known to have stopped, so the source may keep pushing.
    ///
    /// This is a hint, not a guarantee: it means "no stop has been observed here", and the
    /// observer may still be disposed before the next event, or already have been disposed
    /// somewhere this call could not see — a value queued behind a delivery running elsewhere is
    /// reported as `Continue` even when that delivery is about to stop. A source must therefore
    /// still honor its disposal; treating `Continue` as proof that downstream is alive is wrong.
    Continue,
    /// The observer will never accept another event.
    ///
    /// The caller must not call [`Observer::on_next`] again, must not call
    /// [`Observer::on_termination`], and should drop the observer instead: it has either already
    /// delivered its own termination downstream or been disposed, so a termination sent now would
    /// be a second one. Unlike [`Flow::Continue`], this is a guarantee, never a hint.
    Stop,
}

impl Flow {
    /// Returns whether this is [`Flow::Continue`].
    #[inline]
    pub fn is_continue(self) -> bool {
        matches!(self, Flow::Continue)
    }

    /// Returns whether this is [`Flow::Stop`].
    #[inline]
    pub fn is_stop(self) -> bool {
        matches!(self, Flow::Stop)
    }
}

/// A trait for observing the progress and termination state of an operation.
pub trait Observer<T, E> {
    /// Called when the next value in the operation is available.
    ///
    /// Returns whether the observer accepts further events. [`Flow::Stop`] means it accepts none
    /// and must not be terminated either, so the caller stops pushing and drops it; see [`Flow`]
    /// for the exact promise each variant makes. An operator that forwards values must return what
    /// its own downstream returned, so that the answer reaches the source at the end of the chain.
    fn on_next(&mut self, value: T) -> Flow;

    /// Called when the operation has reached its termination state.
    fn on_termination(self, termination: Termination<E>);
}

#[derive(Educe)]
#[educe(Debug, Clone, PartialEq, Eq)]
pub enum Event<T, E> {
    Next(T),
    Termination(Termination<E>),
}

pub trait BoxedObserverExt<T, E>: Observer<T, E> + Sized {
    fn into_boxed<'or>(self) -> BoxedObserver<'or, T, E>
    where
        Self: MaybeSend + 'or,
    {
        BoxedObserver::new(self)
    }
}

impl<T, E, OR> BoxedObserverExt<T, E> for OR where OR: Observer<T, E> {}
