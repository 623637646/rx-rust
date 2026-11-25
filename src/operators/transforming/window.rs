//! Divides the items into observables delimited by another observable.
//!
//! The [`Window`] operator, behind
//! [`ObservableExt::window`](crate::observable::ObservableExt::window).

use super::internal::{boundary, overlap::no_overlap};
use crate::{
    delegate_disposal,
    disposable::{Disposable, chain_disposal::ChainDisposal, dispose_on_drop::DisposeOnDrop},
    observable::{ChainDisposalOf, JoinedModeOf, Observable, ObservableTypes},
    observer::{Observer, boxed_observer::ObserverMode},
    subject::unicast_subject::BoxedUnicastObservable,
    thread_mode::ThreadMode,
    utils::{MarkerType, subscribe_with_context},
};
use educe::Educe;
use std::marker::PhantomData;

#[doc(hidden)]
pub use super::internal::{
    boundary::{BoundaryObserver as BoundaryObserver_, SourceObserver as SourceObserver_},
    window::{Action, DelegateObserver, Windows},
};

/// Subdivides the items of an observable into observable windows, opening a new one whenever the
/// `boundary` observable emits an item.
/// See <https://reactivex.io/documentation/operators/window.html>
///
/// The values of the `boundary` are dropped — only their arrival matters — so it can be an
/// observable of any item type.
///
/// Completing the `boundary` stops future window rotation without terminating the current window
/// or the outer observable. An error from the `boundary` terminates the current window and the
/// outer observable.
///
/// Each window is a single-consumer pipe: it can be subscribed to once, it buffers the items that
/// arrive while it has no subscriber, and dropping it without subscribing discards its items. A
/// window is serialized on its own rather than together with the outer observable, so the events
/// of a window keep their order among themselves, but they are not ordered against the emission of
/// a later window. Disposing the outer subscription ends the open window without a termination:
/// what it had buffered is still delivered to its subscriber, even a later one, which is then
/// dropped without being notified.
///
/// Disposing the subscription of a single window does not necessarily release its observer where
/// it happens: the window releases it on its next item, when it ends, or when the outer
/// subscription is disposed, whichever comes first. See
/// [the unicast subject](crate::subject::unicast_subject) each window is built on.
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt,
///     observer::{Observer, Termination},
///     operators::transforming::window::Window,
///     subject::publish_subject::PublishSubject,
/// };
/// use std::convert::Infallible;
///
/// let mut windows = Vec::new();
/// let mut source = PublishSubject::local();
/// let mut boundary = PublishSubject::<_, Infallible, _>::local();
///
/// let subscription = Window::new(source.clone(), boundary.clone())
///     .concat_map(|window| window.to_vec())
///     .subscribe_with_callback(
///         |values| windows.push(values),
///         |termination| assert_eq!(termination, Termination::Completed),
///     );
///
/// let _ = source.on_next(1);
/// let _ = source.on_next(2);
/// let _ = boundary.on_next(()); // Completes the first window and opens the second.
/// let _ = source.on_next(3);
/// source.on_termination(Termination::Completed);
///
/// drop((subscription, boundary));
/// assert_eq!(windows, [vec![1, 2], vec![3]]);
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct Window<'a, OE, OE1> {
    source: OE,
    boundary: OE1,
    /// The observer of a window may borrow for `'a`. Unlike the `_boxed` hooks this cannot be left
    /// to an unboxed default: the window is the `Item`, which [`ObservableTypes`] names without any
    /// observer, and its observer arrives only later, so the window boxes it.
    _marker: MarkerType<&'a ()>,
}

impl<OE, OE1> Window<'_, OE, OE1> {
    /// Creates a [`Window`] over `source`;
    /// [`ObservableExt::window`](crate::observable::ObservableExt::window) is the fluent form.
    pub fn new<E>(source: OE, boundary: OE1) -> Self
    where
        OE: ObservableTypes<Error = E>,
        OE1: ObservableTypes<Error = E>,
    {
        Self {
            source,
            boundary,
            _marker: PhantomData,
        }
    }
}

/// The observer a [`Window`] gives its source.
#[doc(hidden)]
pub type SourceObserver<'a, M, T, E, OR, D> =
    SourceObserver_<M, Windows<T>, Action<T, E>, E, DelegateObserver<'a, M, T, E, OR>, D>;

/// The observer a [`Window`] gives its boundary, whose completion only stops the rotation
/// (`false`).
#[doc(hidden)]
pub type BoundaryObserver<'a, M, T, E, OR, D> =
    BoundaryObserver_<M, Windows<T>, Action<T, E>, E, DelegateObserver<'a, M, T, E, OR>, D, false>;

delegate_disposal!(
    Disposal<M, T, E, D, D1>,
    subscribe_with_context::Disposal<M, Action<T, E>, E, Windows<T>, ChainDisposal<D, D1>>,
    where M: ThreadMode, D: Disposable, D1: Disposable
);

impl<'a, T, E, OE, OE1> ObservableTypes for Window<'a, OE, OE1>
where
    JoinedModeOf<OE, OE1>: ObserverMode,
    OE: ObservableTypes<Item = T, Error = E>,
    OE1: ObservableTypes<Error = E>,
{
    type Item = BoxedUnicastObservable<'a, T, E, JoinedModeOf<OE, OE1>>;
    type Error = E;
    type Mode = JoinedModeOf<OE, OE1>;
    type Disposal = Disposal<JoinedModeOf<OE, OE1>, T, E, OE::Disposal, OE1::Disposal>;
}

impl<'a, T, E, OE, OE1, OR> Observable<OR> for Window<'a, OE, OE1>
where
    JoinedModeOf<OE, OE1>: ObserverMode,
    OR: Observer<BoxedUnicastObservable<'a, T, E, JoinedModeOf<OE, OE1>>, E>,
    E: Clone,
    OE: Observable<
            SourceObserver<'a, JoinedModeOf<OE, OE1>, T, E, OR, ChainDisposalOf<OE, OE1>>,
            Item = T,
            Error = E,
        >,
    OE1: Observable<
            BoundaryObserver<'a, JoinedModeOf<OE, OE1>, T, E, OR, ChainDisposalOf<OE, OE1>>,
            Error = E,
        >,
{
    fn subscribe(self, observer: OR) -> DisposeOnDrop<Self::Disposal> {
        // One window at a time, so no item is cloned.
        boundary::subscribe(
            self.source,
            self.boundary,
            Windows::new(),
            DelegateObserver::new(observer, no_overlap),
        )
        .map_inner_into()
    }
}
