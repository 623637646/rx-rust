//! The thread mode of an observable: whether its events can arrive from a thread other than the
//! one that subscribed.
//!
//! Every observable computes its [`Mode`](crate::observable::ObservableTypes::Mode) along the
//! chain, from the source down: a synchronous source is [`Local`], an operator that delivers
//! through a scheduler takes the scheduler's mode, and an operator with several sources joins
//! theirs ([`Joined`]). An operator that needs shared state picks its pointer from the mode — an
//! `Rc<RefCell<_>>` for `Local`, an `Arc<Mutex<_>>` for `Shared` — so a chain that never leaves its
//! thread pays for no lock and asks nothing of its observer, and both kinds of chain live in one
//! binary.
//!
//! The mode is a choice of pointers, not a promise the compiler takes on trust: a `Local` state is
//! an `Rc`, which is not `Send`, so declaring `Local` and emitting from another thread fails to
//! compile wherever the state would cross the thread. `Send` itself is only ever required where a
//! value really crosses one — when a task is handed to a multi-threaded scheduler, or an observer
//! is boxed into a `Send` box.
//!
//! This module is the bottom layer of the crate: the mode, and the locks, pointers and flags it
//! picks ([`mutable`]), depend on nothing else in it. What a mode means for an observer — which
//! box it is erased into — is an extension of the mode defined with the observers:
//! [`ObserverMode`](crate::observer::boxed_observer::ObserverMode).

pub mod mutable;

use mutable::{MutableBoolHelper, MutableHelper};
use std::{
    cell::{Cell, RefCell},
    marker::PhantomData,
    rc::Rc,
    sync::{Arc, Mutex, atomic::AtomicBool},
};

mod sealed {
    use super::{Local, Shared};

    pub trait Mode {}
    impl Mode for Local {}
    impl Mode for Shared {}
}

/// One of the two thread modes, [`Local`] and [`Shared`], and the types each one picks.
///
/// A mode is a type-level tag: it is never instantiated, and `PhantomData<M>` is `Send` exactly
/// when events of `M` may cross threads.
///
/// The trait is sealed: the two modes are the only ones. Even a mode that only forwards to
/// [`Local`] cannot be added:
///
// This example has no twin that compiles, so nothing tells that it still fails for the sealing
// alone: it must implement every item of `ThreadMode`, and follow the trait when an item is added
// or changed. Otherwise a missing item fails it too, and it would pass with the sealing gone.
/// ```compile_fail
/// use rx_rust::thread_mode::{Local, ThreadMode};
///
/// struct Custom;
///
/// impl ThreadMode for Custom {
///     type Ptr<T> = <Local as ThreadMode>::Ptr<T>;
///     type Weak<T> = <Local as ThreadMode>::Weak<T>;
///     type Flag = <Local as ThreadMode>::Flag;
///     type Or<M: ThreadMode> = M;
///
///     fn ptr<T>(value: T) -> Self::Ptr<T> {
///         Local::ptr(value)
///     }
///
///     fn downgrade<T>(ptr: &Self::Ptr<T>) -> Self::Weak<T> {
///         Local::downgrade(ptr)
///     }
///
///     fn upgrade<T>(weak: &Self::Weak<T>) -> Option<Self::Ptr<T>> {
///         Local::upgrade(weak)
///     }
/// }
/// ```
pub trait ThreadMode: sealed::Mode + 'static {
    /// The shared, lockable state: `Rc<RefCell<T>>` or `Arc<Mutex<T>>`. Clones share the state.
    /// Reach through it with [`MutableHelper`]; create it with [`ptr`](ThreadMode::ptr).
    type Ptr<T>: Clone + MutableHelper<Value = T>;
    /// The weak counterpart of [`Ptr`](ThreadMode::Ptr), which does not keep the state alive:
    /// `rc::Weak<RefCell<T>>` or `sync::Weak<Mutex<T>>`.
    type Weak<T>: Clone;
    /// A flag shared by several owners and read without a lock: `Rc<Cell<bool>>` or
    /// `Arc<AtomicBool>`. Clones share the flag. Reach through it with [`MutableBoolHelper`];
    /// create it with `Default`.
    type Flag: Clone + Default + MutableBoolHelper;
    /// The mode joined with another one: [`Shared`] if either is. An operator with several sources
    /// computes its own mode with it; see [`Joined`].
    type Or<M: ThreadMode>: ThreadMode;

    /// Creates a new shared, lockable state.
    fn ptr<T>(value: T) -> Self::Ptr<T>;

    /// Creates a weak pointer to the same state.
    fn downgrade<T>(ptr: &Self::Ptr<T>) -> Self::Weak<T>;

    /// Gets the state back from a weak pointer, unless its last strong pointer is gone.
    fn upgrade<T>(weak: &Self::Weak<T>) -> Option<Self::Ptr<T>>;
}

/// Events arrive only on the thread that subscribed.
///
/// For an observable implemented by hand this is a contract. Getting it wrong is not a runtime
/// error: `Local` makes the shared state downstream an `Rc<RefCell<_>>`, which is not `Send`, so a
/// `Local` source that emits from another thread either fails to compile (there is shared state
/// downstream) or is correct anyway (there is none). When unsure, declare [`Shared`]: it only costs
/// a lock. To have the declaration enforced, put a `PhantomData<M>` into the observer wrapper the
/// emitting code receives: `Local` is `!Send + !Sync`, so the wrapper is too.
pub struct Local(PhantomData<*const ()>);

/// Events can arrive from another thread than the one that subscribed.
pub struct Shared(());

impl ThreadMode for Local {
    type Ptr<T> = Rc<RefCell<T>>;
    type Weak<T> = std::rc::Weak<RefCell<T>>;
    type Flag = Rc<Cell<bool>>;
    type Or<M: ThreadMode> = M;

    fn ptr<T>(value: T) -> Self::Ptr<T> {
        Rc::new(RefCell::new(value))
    }
    fn downgrade<T>(ptr: &Self::Ptr<T>) -> Self::Weak<T> {
        Rc::downgrade(ptr)
    }
    fn upgrade<T>(weak: &Self::Weak<T>) -> Option<Self::Ptr<T>> {
        weak.upgrade()
    }
}

impl ThreadMode for Shared {
    type Ptr<T> = Arc<Mutex<T>>;
    type Weak<T> = std::sync::Weak<Mutex<T>>;
    type Flag = Arc<AtomicBool>;
    type Or<M: ThreadMode> = Shared;

    fn ptr<T>(value: T) -> Self::Ptr<T> {
        Arc::new(Mutex::new(value))
    }
    fn downgrade<T>(ptr: &Self::Ptr<T>) -> Self::Weak<T> {
        Arc::downgrade(ptr)
    }
    fn upgrade<T>(weak: &Self::Weak<T>) -> Option<Self::Ptr<T>> {
        weak.upgrade()
    }
}

/// The mode of `A` joined with `B`.
pub type Joined<A, B> = <A as ThreadMode>::Or<B>;
