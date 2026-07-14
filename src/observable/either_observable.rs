use super::{Observable, Subscription};
use crate::{disposable::Disposable, observer::Observer, utils::types::MaybeSend};
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

/// The subscription produced by an [`EitherObservable`].
pub enum EitherDisposal<A, B>
where
    A: Disposable,
    B: Disposable,
{
    Left(Subscription<A>),
    Right(Subscription<B>),
}

impl<A, B> Disposable for EitherDisposal<A, B>
where
    A: Disposable,
    B: Disposable,
{
    fn dispose(self) {
        // Dropping the contained subscription disposes the selected branch.
    }
}

impl<A, B> std::fmt::Debug for EitherDisposal<A, B>
where
    A: Disposable,
    B: Disposable,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(std::any::type_name::<Self>())
    }
}

impl<'or, T, E, A, B> Observable<'or> for EitherObservable<A, B>
where
    A: Observable<'or, T = T, E = E>,
    B: Observable<'or, T = T, E = E>,
{
    type T = T;
    type E = E;
    type D = EitherDisposal<A::D, B::D>;

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
