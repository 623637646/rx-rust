//! An observable that is one of two types, without boxing.

use super::{Observable, ObservableTypes, Subscription};
use crate::{
    delegate_disposal,
    disposable::{Disposable, either_disposal::EitherDisposal},
    observer::Observer,
    thread_mode::Joined,
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

delegate_disposal!(
    /// The disposal of an [`EitherObservable`] subscription: the subscription of the side that was
    /// subscribed to.
    Disposal<A, B>,
    EitherDisposal<Subscription<A>, Subscription<B>>,
    where A: Disposable, B: Disposable
);

impl<A, B> ObservableTypes for EitherObservable<A, B>
where
    A: ObservableTypes,
    B: ObservableTypes<Item = A::Item, Error = A::Error>,
{
    type Item = A::Item;
    type Error = A::Error;
    /// Either side can be the one subscribed to, so the mode is both sides' joined.
    type Mode = Joined<A::Mode, B::Mode>;
    type Disposal = Disposal<A::Disposal, B::Disposal>;
}

impl<A, B, OR> Observable<OR> for EitherObservable<A, B>
where
    OR: Observer<A::Item, A::Error>,
    A: Observable<OR>,
    B: Observable<OR> + ObservableTypes<Item = A::Item, Error = A::Error>,
{
    fn subscribe(self, observer: OR) -> Subscription<Self::Disposal> {
        match self {
            Self::Left(observable) => {
                Subscription::new(EitherDisposal::Left(observable.subscribe(observer)).into())
            }
            Self::Right(observable) => {
                Subscription::new(EitherDisposal::Right(observable.subscribe(observer)).into())
            }
        }
    }
}
