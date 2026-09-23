//! A disposal that disposes two others in order.

use crate::disposable::Disposable;

/// A disposal that disposes `first`, then `second`.
///
/// The order is a guarantee operators rely on: `do_before_disposal` puts its callback in `first`
/// and `do_after_disposal` in `second`. [`DisposableExt::then`](crate::disposable::DisposableExt::then)
/// builds one from any disposal.
///
/// # Examples
/// ```rust
/// use rx_rust::disposable::{callback_disposal::CallbackDisposal, chain_disposal::ChainDisposal, Disposable};
///
/// use std::cell::RefCell;
///
/// let order = RefCell::new(Vec::new());
/// ChainDisposal::new(
///     CallbackDisposal::new(|| order.borrow_mut().push("first")),
///     CallbackDisposal::new(|| order.borrow_mut().push("second")),
/// )
/// .dispose();
/// assert_eq!(*order.borrow(), ["first", "second"]);
/// ```
pub struct ChainDisposal<D1, D2> {
    first: D1,
    second: D2,
}

impl<D1, D2> ChainDisposal<D1, D2> {
    /// Creates a disposal that disposes `first`, then `second`.
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
