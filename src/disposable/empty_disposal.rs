//! `()` as the disposal of a subscription that holds nothing to release.
//!
//! Synchronous sources such as [`Just`](crate::operators::creating::just::Just) have delivered
//! everything before `subscribe` returns, so their subscription has nothing to dispose of.

use crate::disposable::Disposable;

impl Disposable for () {
    fn dispose(self) {}
}
