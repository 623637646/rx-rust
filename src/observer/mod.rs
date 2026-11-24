//! The receiving end of a stream: the [`Observer`] trait and the events it receives.
//!
//! An observer gets each value through [`Observer::on_next`], answering with a [`Flow`] that says
//! whether it accepts more, and the last event through [`Observer::on_termination`], which
//! consumes it. [`Termination`] is that last event, [`Event`] is either kind of event as a
//! value, for `materialize` and friends, and [`EventBatch`] is several events that must arrive
//! together.
//!
//! A pair of closures is an observer too, through
//! [`CallbackObserver`](callback_observer::CallbackObserver), and
//! [`BoxedObserver`](boxed_observer::BoxedObserver) erases an observer's type.
//!
//! # Examples
//! ```rust
//! use rx_rust::{
//!     observable::Observable,
//!     observer::{Flow, Observer, Termination},
//!     operators::creating::from_iter::FromIter,
//! };
//! use std::convert::Infallible;
//!
//! struct Summing(i32);
//!
//! impl Observer<i32, Infallible> for Summing {
//!     fn on_next(&mut self, value: i32) -> Flow {
//!         self.0 += value;
//!         Flow::Continue(())
//!     }
//!
//!     fn on_termination(self, termination: Termination<Infallible>) {
//!         assert_eq!(termination, Termination::Completed);
//!         assert_eq!(self.0, 6);
//!     }
//! }
//!
//! FromIter::new([1, 2, 3]).subscribe(Summing(0));
//! ```

pub mod boxed_observer;
pub mod callback_observer;
pub mod emitter;

use educe::Educe;
use std::ops::ControlFlow;

/// The last event of a stream: it either completed or failed with an error.
///
/// There is one termination type rather than separate `on_completed` / `on_error` callbacks, so
/// that an operator that treats both the same way handles them in one place.
#[derive(Educe)]
#[educe(Debug, Clone, PartialEq, Eq)]
pub enum Termination<E> {
    /// The stream ended after delivering every value.
    Completed,
    /// The stream ended because of `E`; no more values follow.
    Error(E),
}

/// Whether an observer still accepts events, as reported by [`Observer::on_next`]: the standard
/// [`ControlFlow`], so that a source can forward it with `?` or drive an iterator with
/// [`Iterator::try_for_each`].
///
/// This is the only way an observer can tell the source that is pushing into it to stop, while
/// that push is running. It matters most for a synchronous source, which delivers its whole
/// sequence before `subscribe` returns: until it does, nobody holds the subscription yet, so
/// disposing it is not yet possible, and an infinite synchronous source would never end.
///
/// It does not replace disposal, and no source may rely on it alone: an observer can also go away
/// between two events, which no return value can report. The two variants promise different
/// things:
///
/// - `Flow::Continue(())`: the observer is not known to have stopped, so the source may keep
///   pushing. This is a hint, not a guarantee: it means "no stop has been observed here", and the
///   observer may still be disposed before the next event, or already have been disposed
///   somewhere this call could not see — a value queued behind a delivery running elsewhere is
///   reported as `Continue` even when that delivery is about to stop. A source must therefore
///   still honor its disposal; treating `Continue` as proof that downstream is alive is wrong.
/// - `Flow::Break(())`: the observer will never accept another event. The caller must stop pushing
///   and drop the observer without calling [`Observer::on_termination`]. This applies even when
///   the observer ends the stream itself, for example because a callback has received enough
///   values.
///
/// [`ControlFlow`] is `#[must_use]`. Use `let _ =` only when the flow can be ignored deliberately.
pub type Flow = ControlFlow<()>;

/// What may answer in place of a [`Flow`].
///
/// A callback bound as `FnMut(T) -> R` with `R: IntoFlow` may return `()`, which keeps the source
/// going — so the common `|value| { ... }` needs no trailing `Flow::Continue(())` — or a [`Flow`],
/// which lets it end its own stream. [`CallbackObserver`](callback_observer::CallbackObserver)
/// relies on it.
///
/// A callback that only diverges, such as `|_| unreachable!()`, is inferred to return `!`, which
/// no stable impl can cover: spell its return type out, as `|_| -> () { unreachable!() }`.
pub trait IntoFlow {
    /// The flow this answer stands for.
    fn into_flow(self) -> Flow;
}

impl IntoFlow for () {
    #[inline]
    fn into_flow(self) -> Flow {
        Flow::Continue(())
    }
}

impl IntoFlow for Flow {
    #[inline]
    fn into_flow(self) -> Flow {
        self
    }
}

/// Receives the values of a stream, then its termination. See the [module documentation](self).
pub trait Observer<T, E> {
    /// Receives the next value.
    ///
    /// Returns whether the observer accepts further events. [`Flow::Break`] means it accepts none
    /// and must not be terminated either, so the caller stops pushing and drops it; see [`Flow`]
    /// for the exact promise each variant makes. An operator that forwards values must return what
    /// its own downstream returned, so that the answer reaches the source at the end of the chain.
    fn on_next(&mut self, value: T) -> Flow;

    /// Receives the last event, consuming the observer.
    fn on_termination(self, termination: Termination<E>);
}

/// Either event of a stream as a value: what
/// [`materialize`](crate::observable::ObservableExt::materialize) emits and
/// [`dematerialize`](crate::observable::ObservableExt::dematerialize) consumes.
#[derive(Educe)]
#[educe(Debug, Clone, PartialEq, Eq)]
pub enum Event<T, E> {
    /// A value.
    Next(T),
    /// The last event.
    Termination(Termination<E>),
}

/// Events delivered as one unit, so that nothing can slip in between them: what the serialized
/// deliveries of [`utils`](crate::utils) take, and what an update of a
/// [`SubscriptionContext`](crate::utils::subscribe_with_context::SubscriptionContext) queues.
#[derive(Educe)]
#[educe(Debug, Clone, PartialEq, Eq)]
pub enum EventBatch<T, E> {
    /// One value.
    Next(T),
    /// The termination alone.
    Termination(Termination<E>),
    /// A last value, then the termination.
    NextAndTermination(T, Termination<E>),
    /// Several values, in order.
    NextBatch(Vec<T>),
    /// Several values, then the termination.
    NextBatchAndTermination(Vec<T>, Termination<E>),
}

impl<T, E> EventBatch<T, E> {
    /// Returns whether the batch carries a termination, after which nothing can be queued.
    pub fn ends_stream(&self) -> bool {
        matches!(
            self,
            Self::Termination(_) | Self::NextAndTermination(..) | Self::NextBatchAndTermination(..)
        )
    }
}
