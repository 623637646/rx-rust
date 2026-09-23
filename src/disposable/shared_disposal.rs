//! A cloneable slot for a disposal that is built, and maybe replaced, after the slot was handed out.

use crate::{
    disposable::Disposable,
    utils::{
        id_generator::{Id, IdGenerator},
        mutable::{Mutable, MutableHelper},
        types::Shared,
    },
};
use educe::Educe;

#[derive(Educe)]
#[educe(Debug, Default)]
enum State<D> {
    #[educe(Default)]
    Idle,
    /// A disposal is being built with the lock released. The id is what the finished build
    /// compares itself against: a later `replace` starts its own build and stores its own id
    /// here, which is what makes the earlier one stale.
    Building(Id),
    Active(D),
    Disposed,
}

#[derive(Educe)]
#[educe(Debug, Default)]
struct Inner<D> {
    state: State<D>,
    id_generator: IdGenerator,
}

/// A cloneable slot holding at most one disposal, which can be filled or replaced later.
///
/// An operator that subscribes to something *after* it has already returned its own
/// subscription — `concat` subscribing to its second source, `catch` to its fallback,
/// `subscribe_on` from a scheduler task — hands the caller a clone of this slot and fills it when
/// the time comes. Disposing the slot disposes whatever it holds then, and whatever is put in
/// afterwards is disposed at once.
///
/// # Examples
/// ```rust
/// use rx_rust::disposable::{callback_disposal::CallbackDisposal, shared_disposal::SharedDisposal, Disposable};
/// use std::cell::Cell;
///
/// let disposed = Cell::new(false);
/// let slot = SharedDisposal::default();
/// let handle = slot.clone();
///
/// slot.replace(|| CallbackDisposal::new(|| disposed.set(true))); // Filled later, through a clone.
/// assert!(!disposed.get());
///
/// handle.dispose();
/// assert!(disposed.get());
/// ```
#[derive(Educe)]
#[educe(Debug, Clone, Default)]
pub struct SharedDisposal<D>(Shared<Mutable<Inner<D>>>);

impl<D> SharedDisposal<D> {
    /// Disposes the held disposal, if any, then builds and stores a new one.
    ///
    /// The builder runs with the lock released, so it may subscribe to anything. If the slot is
    /// disposed, or replaced again, while the builder runs, what it built is disposed at once;
    /// once the slot has been disposed the builder is not run at all.
    pub fn replace(&self, disposal_builder: impl FnOnce() -> D)
    where
        D: Disposable,
    {
        // The superseded disposal is handed back rather than disposed under the lock.
        let (id, superseded) = self.0.with_mut(|inner| {
            if matches!(inner.state, State::Disposed) {
                return (None, None);
            }
            let id = inner.id_generator.next_id();
            match std::mem::replace(&mut inner.state, State::Building(id)) {
                State::Idle | State::Building(_) => (Some(id), None),
                State::Active(disposal) => (Some(id), Some(disposal)),
                State::Disposed => unreachable!("the disposed state returned above"),
            }
        });
        if let Some(disposal) = superseded {
            disposal.dispose();
        }

        let Some(id) = id else {
            return;
        };

        let disposable = disposal_builder();

        let stale = self.0.with_mut(|inner| {
            let is_current = match &inner.state {
                State::Building(current) => *current == id,
                State::Idle | State::Active(_) | State::Disposed => false,
            };
            if is_current {
                inner.state = State::Active(disposable);
                None
            } else {
                // Reset, disposed, or superseded by a later build: this disposal is already dead.
                Some(disposable)
            }
        });
        if let Some(disposable) = stale {
            disposable.dispose();
        }
    }
}

impl<D> Disposable for SharedDisposal<D>
where
    D: Disposable,
{
    fn dispose(self) {
        // The state is replaced under the lock and matched after it is released, so the inner
        // disposal is disposed outside the lock.
        match self
            .0
            .with_mut(|inner| std::mem::replace(&mut inner.state, State::Disposed))
        {
            State::Idle | State::Building(_) | State::Disposed => {}
            State::Active(disposable) => {
                Disposable::dispose(disposable);
            }
        }
    }
}
