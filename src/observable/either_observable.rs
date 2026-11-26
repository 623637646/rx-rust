//! An observable that is one of two types, without boxing.

use super::{JoinedModeOf, Observable, ObservableTypes};
use crate::{
    disposable::{dispose_on_drop::DisposeOnDrop, either_disposal::EitherDisposal},
    observer::Observer,
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

impl<A, B> ObservableTypes for EitherObservable<A, B>
where
    A: ObservableTypes,
    B: ObservableTypes<Item = A::Item, Error = A::Error>,
{
    type Item = A::Item;
    type Error = A::Error;
    /// Either side can be the one subscribed to, so the mode is both sides' joined.
    type Mode = JoinedModeOf<A, B>;
    /// The subscription of the side that was subscribed to.
    type Disposal = EitherDisposal<A::Disposal, B::Disposal>;
}

impl<A, B, OR> Observable<OR> for EitherObservable<A, B>
where
    OR: Observer<A::Item, A::Error>,
    A: Observable<OR>,
    B: Observable<OR> + ObservableTypes<Item = A::Item, Error = A::Error>,
{
    fn subscribe(self, observer: OR) -> DisposeOnDrop<Self::Disposal> {
        match self {
            Self::Left(observable) => observable
                .subscribe(observer)
                .map_inner(EitherDisposal::Left),
            Self::Right(observable) => observable
                .subscribe(observer)
                .map_inner(EitherDisposal::Right),
        }
    }
}
