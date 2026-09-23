//! An observable that is one of two types, without boxing.

use super::{Observable, Subscription};
use crate::{
    disposable::either_disposal::EitherDisposal, observer::Observer, utils::types::MaybeSend,
};
use educe::Educe;

/// An observable that is one of two concrete types.
///
/// Unlike [`BoxedObservable`](super::boxed_observable::BoxedObservable) it neither allocates nor
/// erases anything: it is the way to return one of two observable types from a function.
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::{either_observable::EitherObservable, ObservableExt},
///     operators::creating::{from_iter::FromIter, just::Just},
/// };
///
/// fn maybe(value: Option<i32>) -> EitherObservable<Just<i32>, FromIter<Vec<i32>>> {
///     match value {
///         Some(value) => EitherObservable::Left(Just::new(value)),
///         None => EitherObservable::Right(FromIter::new(Vec::new())),
///     }
/// }
///
/// let mut seen = Vec::new();
/// maybe(Some(1)).subscribe_with_callback(|value| seen.push(value), |_| {});
/// maybe(None).subscribe_with_callback(|_| -> () { unreachable!() }, |_| {});
/// assert_eq!(seen, [1]);
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub enum EitherObservable<A, B> {
    /// The first of the two types.
    Left(A),
    /// The second of the two types.
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
