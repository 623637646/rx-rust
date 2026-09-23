//! A disposal that is one of two types, without boxing.

use crate::disposable::Disposable;
use educe::Educe;

/// A disposal that is one of two concrete types.
///
/// This is what lets a `subscribe` that takes one of two paths return a single, unboxed disposal
/// type; [`EitherObservable`](crate::observable::either_observable::EitherObservable) uses it.
///
/// # Examples
/// ```rust
/// use rx_rust::disposable::{callback_disposal::CallbackDisposal, either_disposal::EitherDisposal, Disposable};
///
/// let mut left_disposed = false;
/// let disposal: EitherDisposal<_, ()> = EitherDisposal::Left(CallbackDisposal::new(|| left_disposed = true));
/// disposal.dispose();
/// assert!(left_disposed);
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub enum EitherDisposal<D1, D2> {
    /// The first of the two types.
    Left(D1),
    /// The second of the two types.
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
