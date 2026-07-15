use super::{Observable, Subscription};
use crate::{
    disposable::either_disposal::EitherDisposal, observer::Observer, utils::types::MaybeSend,
};
use educe::Educe;

/// An observable that is one of two concrete observable types.
///
/// Unlike [`super::boxed_observable::BoxedObservable`], this type preserves static
/// dispatch and does not allocate. It is useful when the set of possible observable
/// types is known at compile time.
#[derive(Educe)]
#[educe(Debug, Clone)]
pub enum EitherObservable<A, B> {
    Left(A),
    Right(B),
}

impl<'or, T, E, A, B> Observable<'or, T, E> for EitherObservable<A, B>
where
    A: Observable<'or, T, E>,
    B: Observable<'or, T, E>,
{
    type D = EitherDisposal<Subscription<A::D>, Subscription<B::D>>;

    fn subscribe(self, observer: impl Observer<T, E> + MaybeSend + 'or) -> Subscription<Self::D> {
        match self {
            Self::Left(observable) => {
                Subscription::new(EitherDisposal::Left(observable.subscribe(observer)))
            }
            Self::Right(observable) => {
                Subscription::new(EitherDisposal::Right(observable.subscribe(observer)))
            }
        }
    }
}
