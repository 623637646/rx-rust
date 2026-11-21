//! Sources shared by the fixed-collection operators.
//!
//! Tuples retain their heterogeneous items; [`IterSources`] adapts a homogeneous iterator without
//! boxing. The public traits only select types: a source set's metadata and the resources
//! operators keep for it, which public disposals name. What the operators do with the sources —
//! collect them, prepare a cursor, subscribe a batch — is in the crate-private traits.
//!
//! Subscribing them has two levels:
//!
//! - [`subscribe`]: every operator subscribes its sources through a cursor and a
//!   [`SourceSubscriber`], which makes the observer of each source and stores its subscription in
//!   the slot of its position. Operators that take their sources a few at a time, or reach a
//!   subscription while the others are still being subscribed (Merge, amb, `When`), write one.
//! - [`batch`]: the shared subscriber, for the operators that collect the sources and their
//!   position count first, then subscribe the collection at once. They only name the observer
//!   of each position, with an [`ObserverFamily`].
//!
//! The source sets are declared here; [`mod@tuple`] implements them for tuples of sources and
//! [`list`] for [`IterSources`].

use crate::{
    disposable::Disposable, thread_mode::ThreadMode, utils::positional::combination::Combination,
};

mod batch;
mod list;
mod subscribe;
mod tuple;

pub use batch::CollectedCursor;
pub(crate) use batch::{ObserverFamily, SubscribeBatch};
pub use list::IterSources;
pub(crate) use subscribe::{SourceSubscriber, SubscribeEach};

mod sealed {
    pub trait Sealed {}
}

/// A collection of observables with one error type and their items in source order.
///
/// Implemented by tuples of one to eight sources and [`IterSources`]. It is
/// sealed: operators rely on each backend's collection and subscription rules. Besides the
/// sources' metadata, it selects the resources the operators keep for them — hidden, since only
/// the operators' types name them — which every source set provides on the same terms as its
/// metadata. It carries types only: the operations on them are crate-private. A list has no
/// tuple arity limit.
pub trait SourceSet: sealed::Sealed {
    /// The sources' items, as a tuple or a `Vec`, in source order: one [`Combination`].
    type Item: Combination;
    /// The error type shared by the sources.
    type Error;
    /// The modes of all sources [`Joined`](crate::thread_mode::Joined): `Shared` as soon as one
    /// of them is.
    type Mode: ThreadMode;

    /// The sources not pulled yet and their positions: a tuple of `Option`s for static positions,
    /// or an enumerated iterator for dynamic ones, prepared by `SourceOps::into_cursor`.
    ///
    /// It is subscribed with `SubscribeEach`: `subscribe_each` hands each remaining source, at
    /// its position, to a `SourceSubscriber` until the subscriber breaks, or every one of them to
    /// a subscriber that cannot. A subscriber that breaks after one source takes them one at a
    /// time, and the next call resumes after it. There is no `next()`: the sources of a tuple
    /// differ in type, so no one return type holds them, while the subscriber has an impl per
    /// position and source type, which carries what subscribing that source requires.
    #[doc(hidden)]
    type Cursor;

    /// The cursor over every source, once they are all collected, prepared by
    /// `SourceOps::into_collected_cursor`: the [`CollectedCursor`] wrapper around the tuple's own
    /// cursor, or around one over the `Vec` an iterator is collected into. It is subscribed as a
    /// batch.
    #[doc(hidden)]
    type CollectedCursor;

    /// The subscriptions of the sources, each at the position of its source; disposed or
    /// dropped, the slots dispose those left the last subscribed first, and neither a cursor nor
    /// the sources not subscribed. A batch only fills them. An operator pulling its sources one
    /// at a time holds each from its subscription to its completion: the default reserves each
    /// position for its source without storing a subscription yet, so a source that completes
    /// before its subscription is returned can release its reservation before it is filled.
    #[doc(hidden)]
    type SubscriptionSlots: Default + Disposable;
}

/// The operations on a [`SourceSet`] that need no observer: collecting every source, and
/// preparing a cursor over them.
///
/// Every source set implements it, on the same terms as [`SourceSet`]; it is apart only so that
/// the operations stay out of the public API, as
/// [`LatestOps`](crate::utils::positional::combination::LatestOps) and
/// [`QueueOps`](crate::utils::positional::combination::QueueOps) do for a [`Combination`]. The
/// types these operations return are [`SourceSet`]'s, next to the others the operators keep.
pub(crate) trait SourceOps: SourceSet {
    /// Collects every source before any is subscribed, with their position count: a tuple's
    /// arity, or the length of the `Vec` a finite iterator is collected into.
    ///
    /// The collection comes as a cursor, subscribed as a [`SubscribeBatch`] batch. The count is
    /// known only here, since a lazy iterator has none, so the two are returned together.
    fn into_collected_cursor(self) -> (Self::CollectedCursor, usize);

    /// Prepares the sources for traversal in source order, neither collecting an iterator nor
    /// subscribing a source. An operator subscribes it with a [`SourceSubscriber`] of its own.
    fn into_cursor(self) -> Self::Cursor;
}

/// A [`SourceSet`] whose sources all emit the same item type, as needed by merge and amb.
pub trait SameItemSources: SourceSet {
    /// The item type shared by every source.
    type SameItem;
}
