//! An observer made of closures.

use super::{Flow, Observer, Termination};
use educe::Educe;

/// What a plain callback may answer in place of a [`Flow`].
///
/// A callback bound as `FnMut(T) -> R` with `R: IntoFlow` may return `()`, which keeps the source
/// going — so the common `|value| { ... }` needs no trailing [`Flow::Continue`] — or a [`Flow`],
/// which lets it end its own stream. This is what
/// [`CallbackObserver`] relies on.
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
        Flow::Continue
    }
}

impl IntoFlow for Flow {
    #[inline]
    fn into_flow(self) -> Flow {
        self
    }
}

/// An [`Observer`] made of two closures, one per event.
///
/// [`ObservableExt::subscribe_with_callback`](crate::observable::ObservableExt::subscribe_with_callback)
/// builds one, so it is rarely named directly.
///
/// # Examples
/// ```rust
/// use rx_rust::observer::{callback_observer::CallbackObserver, Flow, Observer, Termination};
///
/// let mut seen = Vec::new();
/// let mut observer = CallbackObserver::new(
///     |value: i32| {
///         seen.push(value);
///         if value == 2 { Flow::Stop } else { Flow::Continue }
///     },
///     |_: Termination<()>| unreachable!("a stopped observer is not terminated"),
/// );
/// assert_eq!(observer.on_next(1), Flow::Continue);
/// assert_eq!(observer.on_next(2), Flow::Stop);
/// drop(observer);
/// assert_eq!(seen, [1, 2]);
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct CallbackObserver<FN, FT> {
    #[educe(Debug(ignore))]
    on_next: FN,
    #[educe(Debug(ignore))]
    on_termination: FT,
}

impl<FN, FT> CallbackObserver<FN, FT> {
    /// Creates an observer that runs `on_next` for each value and `on_termination` for the last
    /// event. `on_next` may return `()` or a [`Flow`], see [`IntoFlow`].
    pub fn new<T, E, R>(on_next: FN, on_termination: FT) -> Self
    where
        FN: FnMut(T) -> R,
        R: IntoFlow,
        FT: FnOnce(Termination<E>),
    {
        Self {
            on_next,
            on_termination,
        }
    }
}

impl<T, E, R, FN, FT> Observer<T, E> for CallbackObserver<FN, FT>
where
    FN: FnMut(T) -> R,
    R: IntoFlow,
    FT: FnOnce(Termination<E>),
{
    fn on_next(&mut self, value: T) -> Flow {
        // A callback that returns nothing keeps the source going; one that returns a `Flow` can
        // end its own stream, see `IntoFlow`.
        (self.on_next)(value).into_flow()
    }

    fn on_termination(self, termination: Termination<E>) {
        (self.on_termination)(termination);
    }
}
