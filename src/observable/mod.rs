//! The sending end of a stream: the [`Observable`] trait and the fluent operator API on it.
//!
//! An [`Observable`] is anything that can be subscribed to with an
//! [`Observer`]; subscribing returns a [`DisposeOnDrop`], which unsubscribes when dropped. Every
//! operator is a struct in [`operators`](crate::operators) that implements `Observable` over its
//! source, and [`ObservableExt`] gives each of them a method, so a pipeline reads as a chain of
//! calls.
//!
//! [`BoxedObservable`](boxed_observable::BoxedObservable) and
//! [`CloneableBoxedObservable`](boxed_observable::CloneableBoxedObservable) erase an observable's
//! type; [`EitherObservable`](either_observable::EitherObservable) picks one of two types without
//! boxing. [`future`] and `stream` (with the `futures` feature) hold what an observable becomes as
//! a `Future` or a `Stream`, through [`into_future`](ObservableExt::into_future), `into_stream` and
//! their siblings.
//!
//! # Examples
//! ```rust
//! use rx_rust::{
//!     observable::ObservableExt, observer::Termination, operators::creating::range::Range,
//! };
//!
//! let mut seen = Vec::new();
//! Range::new(1..=10)
//!     .filter(|value| value % 2 == 0)
//!     .map(|value| value * value)
//!     .take(3)
//!     .subscribe_with_callback(
//!         |value| seen.push(value),
//!         |termination| assert_eq!(termination, Termination::Completed),
//!     );
//! assert_eq!(seen, [4, 16, 36]);
//! ```

pub mod boxed_observable;
pub mod either_observable;
pub mod future;
mod observable_ext;
pub(crate) mod sources;
#[cfg(feature = "futures")]
pub mod stream;

use crate::{
    disposable::{Disposable, chain_disposal::ChainDisposal, dispose_on_drop::DisposeOnDrop},
    observable::boxed_observable::BoxedObservableMode,
    observer::{Observer, boxed_observer::ObserverMode},
    scheduler::SchedulerTypes,
    thread_mode::{Joined, ThreadMode},
};

pub use crate::utils::positional::combination::Combination;
#[doc(hidden)]
pub use crate::utils::positional::{
    position::TupleIndex,
    slots::{ListSlots, ReservationSlot, TupleSlots},
    tuple::IntoReversed,
};
pub use observable_ext::ObservableExt;
pub use sources::{CollectedCursor, IterSources, SameItemSources, SourceSet};

/// The thread mode of an operator with the two sources `A` and `B`: their modes
/// [`Joined`], `Shared` as soon as one of them is.
pub type JoinedModeOf<A, B> = Joined<<A as ObservableTypes>::Mode, <B as ObservableTypes>::Mode>;

/// The thread mode of an operator with the source `OE` and the scheduler `S`: their modes
/// [`Joined`], `Shared` as soon as one of them is.
pub type JoinedSchedulerModeOf<OE, S> =
    Joined<<OE as ObservableTypes>::Mode, <S as SchedulerTypes>::Mode>;

/// The boxed observer of the source `OE`'s mode, for its items and errors: what an operator that
/// hands the downstream observer to a user callback boxes it into, so it subscribes any observer.
pub type BoxedObserverOf<'a, OE> = <<OE as ObservableTypes>::Mode as ObserverMode>::BoxedObserver<
    'a,
    <OE as ObservableTypes>::Item,
    <OE as ObservableTypes>::Error,
>;

/// The erased observable of the source `OE`'s mode, for its items and errors: what
/// [`into_boxed`](ObservableExt::into_boxed) returns, a
/// [`BoxedObservable`](boxed_observable::BoxedObservable) for `Local` and a
/// [`SendBoxedObservable`](boxed_observable::SendBoxedObservable) for `Shared`.
pub type BoxedObservableOf<'or, 'sub, 'oe, OE> =
    <<OE as ObservableTypes>::Mode as BoxedObservableMode>::Boxed<
        'or,
        'sub,
        'oe,
        <OE as ObservableTypes>::Item,
        <OE as ObservableTypes>::Error,
    >;

/// The cloneable erased observable of the source `OE`'s mode: what
/// [`into_cloneable_boxed`](ObservableExt::into_cloneable_boxed) returns, a
/// [`CloneableBoxedObservable`](boxed_observable::CloneableBoxedObservable) for `Local` and a
/// [`SendCloneableBoxedObservable`](boxed_observable::SendCloneableBoxedObservable) for `Shared`.
pub type CloneableBoxedObservableOf<'or, 'sub, 'oe, OE> =
    <<OE as ObservableTypes>::Mode as BoxedObservableMode>::CloneableBoxed<
        'or,
        'sub,
        'oe,
        <OE as ObservableTypes>::Item,
        <OE as ObservableTypes>::Error,
    >;

/// The disposals of the two sources `A` and `B` chained into one: disposing it disposes `A`'s
/// subscription first, then `B`'s.
pub type ChainDisposalOf<A, B> =
    ChainDisposal<<A as ObservableTypes>::Disposal, <B as ObservableTypes>::Disposal>;

/// The disposal of a task run on the scheduler `S` and that of the source `OE` chained into one:
/// disposing it cancels the task first, then disposes the source's subscription. What the context
/// of an operator with a long-lived scheduler task owns, as `delay` or `buffer_with_time` do.
pub type ChainSchedulerDisposalOf<S, OE> =
    ChainDisposal<<S as SchedulerTypes>::Disposal, <OE as ObservableTypes>::Disposal>;

/// The part of an observable that does not depend on who observes it: what it emits, the thread
/// its events can arrive on, and the disposal of a subscription to it.
///
/// It is split from [`Observable`] on purpose, so that none of these can mention the observer's
/// type. An operator that owns the subscriptions of two sources, such as `merge`, subscribes each
/// of them with an observer whose type contains the other one's disposal; if a disposal could
/// depend on its observer, the disposal of the second source would appear in its own definition.
pub trait ObservableTypes {
    /// The values.
    type Item;
    /// The error a failed stream ends with.
    type Error;
    /// Whether the events can arrive from another thread than the one that subscribed:
    /// [`Local`](crate::thread_mode::Local) when they cannot,
    /// [`Shared`](crate::thread_mode::Shared) when they can.
    ///
    /// It is computed along the chain from the source down: a synchronous source is `Local`, an
    /// operator delivering through a scheduler takes the scheduler's mode, and an operator with
    /// several sources joins theirs ([`Joined`]). An operator that needs shared state picks its
    /// pointer from it. See [`thread_mode`](crate::thread_mode).
    type Mode: ThreadMode;
    /// The inner disposal of the [`DisposeOnDrop`] returned by [`Observable::subscribe`].
    type Disposal: Disposable;
}

/// A source of [`Item`](ObservableTypes::Item)s that ends with a
/// [`Termination`](crate::observer::Termination)`<`[`Error`](ObservableTypes::Error)`>`,
/// subscribed to by an observer of type `OR`. See
/// <https://reactivex.io/documentation/observable.html>.
///
/// The observer is a parameter of the trait rather than of `subscribe`, so that each implementation
/// states what it needs from its observer: only an operator that hands the observer to another
/// thread — through a scheduler, or a `Send` box — asks for `Send`, and a chain that stays on its
/// thread accepts any observer. See [`ObservableTypes`] for why the associated types are not
/// declared here.
///
/// # Examples
/// A synchronous chain takes an observer that is not `Send`:
/// ```rust
/// use rx_rust::{observable::ObservableExt, operators::creating::just::Just};
/// use std::{cell::RefCell, rc::Rc};
///
/// let values = Rc::new(RefCell::new(Vec::new()));
/// let observer_values = values.clone();
/// let _subscription = Just::new(1)
///     .map(|value| value + 1)
///     .subscribe_with_callback(move |value| observer_values.borrow_mut().push(value), |_| {});
/// assert_eq!(*values.borrow(), [2]);
/// ```
pub trait Observable<OR>: ObservableTypes
where
    OR: Observer<Self::Item, Self::Error>,
{
    /// Subscribes `observer`, which receives the events from now on, consuming the observable.
    ///
    /// The returned [`DisposeOnDrop`] unsubscribes when dropped.
    fn subscribe(self, observer: OR) -> DisposeOnDrop<Self::Disposal>;
}
