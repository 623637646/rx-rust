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

#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct CallbackObserver<FN, FT> {
    #[educe(Debug(ignore))]
    on_next: FN,
    #[educe(Debug(ignore))]
    on_termination: FT,
}

impl<FN, FT> CallbackObserver<FN, FT> {
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
