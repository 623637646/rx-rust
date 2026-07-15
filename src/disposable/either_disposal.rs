use crate::disposable::Disposable;
use educe::Educe;

#[derive(Educe)]
#[educe(Debug, Clone)]
pub enum EitherDisposal<D1, D2> {
    Left(D1),
    Right(D2),
}

impl<D1, D2> Disposable for EitherDisposal<D1, D2>
where
    D1: Disposable,
    D2: Disposable,
{
    fn dispose(self) {
        match self {
            EitherDisposal::Left(d) => d.dispose(),
            EitherDisposal::Right(d) => d.dispose(),
        }
    }
}
