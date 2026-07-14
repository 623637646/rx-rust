use crate::disposable::Disposable;

/// A disposal that disposes `first`, then `second`.
///
/// The front-to-back dispose order is a guarantee, not an implementation detail:
/// operators encode their semantics in it. For example, `DoBeforeDisposal` places
/// its callback in `first` to run before the source's disposal, while
/// `DoAfterDisposal` places it in `second` to run after.
pub struct ChainDisposal<D1, D2> {
    first: D1,
    second: D2,
}

impl<D1, D2> ChainDisposal<D1, D2> {
    pub fn new(first: D1, second: D2) -> Self {
        Self { first, second }
    }
}

impl<D1, D2> Disposable for ChainDisposal<D1, D2>
where
    D1: Disposable,
    D2: Disposable,
{
    fn dispose(self) {
        self.first.dispose();
        self.second.dispose();
    }
}
