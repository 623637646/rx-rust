//! A cloneable slot for a disposal that is built, and maybe replaced, after the slot was handed
//! out.

use crate::{
    disposable::{Disposable, dispose_on_drop::DisposeOnDrop},
    thread_mode::{ThreadMode, mutable::MutableHelper},
    utils::id_generator::{Id, IdGenerator},
};
use educe::Educe;

#[derive(Educe)]
#[educe(Debug, Default)]
enum State<D: Disposable> {
    #[educe(Default)]
    Idle,
    /// A disposal is being built with the lock released. The id is what the finished build
    /// compares itself against: a later `replace_with` starts its own build and stores its own id
    /// here, which is what makes the earlier one stale.
    Building(Id),
    Active(DisposeOnDrop<D>),
    Disposed,
}

impl<D: Disposable> State<D> {
    /// Whether the build `id` is still the current one: the slot was neither disposed nor
    /// replaced again since it started.
    fn is_current_build(&self, id: Id) -> bool {
        matches!(self, State::Building(current) if *current == id)
    }
}

#[derive(Educe)]
#[educe(Debug, Default)]
struct Inner<D: Disposable> {
    state: State<D>,
    id_generator: IdGenerator,
}

/// A cloneable slot holding at most one disposal, releasing the old disposal when replaced.
///
/// The slot holds the [`DisposeOnDrop`] that `subscribe` or `run_task` returns, so dropping its
/// last clone disposes what it holds, as dropping that wrapper would.
///
/// An operator that subscribes to something *after* it has already returned its own
/// subscription — `concat` subscribing to its second source, `catch` to its fallback,
/// `subscribe_on` from a scheduler task — hands the caller a clone of this slot and fills it when
/// the time comes. Disposing the slot disposes whatever it holds then and prevents later builders
/// from running. If a build is already in progress, its result is disposed when the builder
/// returns.
///
/// # Examples
/// ```rust
/// use rx_rust::disposable::{
///     Disposable, DisposableExt, callback_disposal::CallbackDisposal,
///     replaceable_disposal::ReplaceableDisposal,
/// };
/// use rx_rust::thread_mode::Local;
/// use std::cell::Cell;
///
/// let disposed = Cell::new(false);
/// let slot = ReplaceableDisposal::<Local, CallbackDisposal<_>>::default();
/// let handle = slot.clone();
///
/// // Filled later, through a clone.
/// slot.replace_with(|| CallbackDisposal::new(|| disposed.set(true)).into_dispose_on_drop());
/// assert!(!disposed.get());
///
/// handle.dispose();
/// assert!(disposed.get());
/// ```
///
/// Clones share the same slot. Sharing ownership does not require sharing between threads:
/// the pointer is the one the thread mode `M` picks, including for `Local`.
#[derive(Educe)]
#[educe(Debug, Clone(bound()))]
pub struct ReplaceableDisposal<M: ThreadMode, D: Disposable>(
    #[educe(Debug(ignore))] M::Ptr<Inner<D>>,
);

// Written by hand rather than derived: it builds through `M::ptr`, since `M::Ptr` is not `Default`.
impl<M: ThreadMode, D: Disposable> Default for ReplaceableDisposal<M, D> {
    fn default() -> Self {
        Self(M::ptr(Inner::default()))
    }
}

impl<M: ThreadMode, D: Disposable> ReplaceableDisposal<M, D> {
    /// Disposes the held disposal, if any, then builds and stores a new one.
    ///
    /// The builder runs with the lock released, so it may subscribe to anything. If the slot is
    /// disposed, or replaced again, while the builder runs, what it built is disposed at once;
    /// once the slot has been disposed the builder is not run at all — nor when the held disposal,
    /// as it is disposed, disposes or replaces the slot.
    pub fn replace_with(&self, disposal_builder: impl FnOnce() -> DisposeOnDrop<D>) {
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
        let Some(id) = id else {
            return;
        };

        if let Some(disposal) = superseded {
            disposal.dispose();
            // Disposing it runs code of its own, which may dispose or replace this slot: the
            // builder must not run then.
            if !self.0.with_ref(|inner| inner.state.is_current_build(id)) {
                return;
            }
        }

        let disposable = disposal_builder();

        let stale = self.0.with_mut(|inner| {
            if inner.state.is_current_build(id) {
                inner.state = State::Active(disposable);
                None
            } else {
                // Disposed, or superseded by a later build: this disposal is already dead.
                Some(disposable)
            }
        });
        if let Some(disposable) = stale {
            disposable.dispose();
        }
    }
}

impl<M: ThreadMode, D: Disposable> Disposable for ReplaceableDisposal<M, D> {
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
