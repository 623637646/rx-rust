//! Gathers items into `Vec`s delimited by another observable.
//!
//! The [`Buffer`] operator, behind
//! [`ObservableExt::buffer`](crate::observable::ObservableExt::buffer).

use super::internal::{boundary, overlap::no_overlap};
use crate::{
    delegate_disposal,
    disposable::{Disposable, chain_disposal::ChainDisposal, dispose_on_drop::DisposeOnDrop},
    observable::{ChainDisposalOf, JoinedModeOf, Observable, ObservableTypes},
    observer::Observer,
    thread_mode::ThreadMode,
    utils::subscribe_with_context,
};
use educe::Educe;

#[doc(hidden)]
pub use super::internal::{
    boundary::{BoundaryObserver as BoundaryObserver_, SourceObserver},
    bundle::Bundles,
};

/// Gathers items from an observable into bundles and emits these bundles as `Vec<T>`, whenever a
/// `boundary` observable emits an item.
/// See <https://reactivex.io/documentation/operators/buffer.html>
///
/// The values of the `boundary` are dropped — only their arrival matters — so it can be an
/// observable of any item type.
///
/// Terminating the `boundary` terminates the outer observable: completing it emits the pending
/// bundle when non-empty and then completes, while an error from it discards the pending bundle
/// and errors. Note that this differs from
/// [`Window`](crate::operators::transforming::window::Window), where completing the `boundary`
/// only stops rotation; a buffer has no consumer to release items to, so leaving it open after
/// the `boundary` completed would accumulate items without bound.
///
/// # Relation to `Window`
///
/// Conceptually a buffer is a window whose items are collected into a `Vec`, that is
/// `source.window(boundary).concat_map(|window| window.to_vec())`. It is implemented on its own
/// rather than as that composition, because the two are not equivalent:
///
/// - Completing the `boundary` terminates a buffer, as described above, but only stops the
///   rotation of a window.
/// - Completing the `source` with an empty pending bundle emits nothing for a buffer, whereas the
///   composition emits a trailing empty `Vec`, because collecting the empty open window yields
///   the initial value.
/// - [`Window`](crate::operators::transforming::window::Window) requires `E: Clone`, since an
///   error is delivered to both the current window and the outer observable. A buffer has no
///   window to deliver to and so places no such bound on `E`.
///
/// The composition also pays for machinery a buffer does not need: a subject per window, and one
/// queued action per item. Where the three points above do not matter, the equivalence holds; the
/// `test_equivalent_to_window_and_collect` test in `tests/buffer.rs` pins it down, alongside two
/// tests that pin down the divergences.
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt,
///     observer::{Observer, Termination},
///     operators::transforming::buffer::Buffer,
///     subject::publish_subject::PublishSubject,
/// };
/// use std::convert::Infallible;
///
/// let mut values = Vec::new();
/// let mut source = PublishSubject::local();
/// let mut boundary = PublishSubject::<_, Infallible, _>::local();
///
/// let subscription = Buffer::new(source.clone(), boundary.clone()).subscribe_with_callback(
///     |value| values.push(value),
///     |termination| assert_eq!(termination, Termination::Completed),
/// );
///
/// let _ = source.on_next(1);
/// let _ = source.on_next(2);
/// let _ = boundary.on_next(());
/// let _ = source.on_next(3);
/// source.on_termination(Termination::Completed); // Emits the pending buffer.
///
/// drop((subscription, boundary));
/// assert_eq!(values, [vec![1, 2], vec![3]]);
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct Buffer<OE, OE1> {
    source: OE,
    boundary: OE1,
}

impl<OE, OE1> Buffer<OE, OE1> {
    /// Creates a [`Buffer`] over `source`;
    /// [`ObservableExt::buffer`](crate::observable::ObservableExt::buffer) is the fluent form.
    pub fn new<E>(source: OE, boundary: OE1) -> Self
    where
        OE: ObservableTypes<Error = E>,
        OE1: ObservableTypes<Error = E>,
    {
        Self { source, boundary }
    }
}

/// The observer a [`Buffer`] gives its source.
#[doc(hidden)]
pub type BufferObserver<M, T, E, OR, D> = SourceObserver<M, Bundles<T>, Vec<T>, E, OR, D>;

/// The observer a [`Buffer`] gives its boundary, whose completion ends the stream (`true`).
#[doc(hidden)]
pub type BoundaryObserver<M, T, E, OR, D> =
    BoundaryObserver_<M, Bundles<T>, Vec<T>, E, OR, D, true>;

delegate_disposal!(
    Disposal<M, T, E, D, D1>,
    subscribe_with_context::Disposal<M, Vec<T>, E, Bundles<T>, ChainDisposal<D, D1>>,
    where M: ThreadMode, D: Disposable, D1: Disposable
);

impl<T, E, OE, OE1> ObservableTypes for Buffer<OE, OE1>
where
    OE: ObservableTypes<Item = T, Error = E>,
    OE1: ObservableTypes<Error = E>,
{
    type Item = Vec<T>;
    type Error = E;
    type Mode = JoinedModeOf<OE, OE1>;
    type Disposal = Disposal<JoinedModeOf<OE, OE1>, T, E, OE::Disposal, OE1::Disposal>;
}

impl<T, E, OE, OE1, OR> Observable<OR> for Buffer<OE, OE1>
where
    OR: Observer<Vec<T>, E>,
    OE: Observable<
            BufferObserver<JoinedModeOf<OE, OE1>, T, E, OR, ChainDisposalOf<OE, OE1>>,
            Item = T,
            Error = E,
        >,
    OE1: Observable<
            BoundaryObserver<JoinedModeOf<OE, OE1>, T, E, OR, ChainDisposalOf<OE, OE1>>,
            Error = E,
        >,
{
    fn subscribe(self, observer: OR) -> DisposeOnDrop<Self::Disposal> {
        // One bundle at a time, so no item is cloned; each opens with room for as many items as
        // the last one emitted.
        boundary::subscribe(
            self.source,
            self.boundary,
            Bundles::with_room(no_overlap, 0),
            observer,
        )
        .map_inner_into()
    }
}
