//! Divides the items into observables of a given length.
//!
//! The [`WindowWithCount`] operator, behind
//! [`ObservableExt::window_with_count`](crate::observable::ObservableExt::window_with_count).

use super::{
    internal::{
        count::CountObserver,
        overlap::NoOverlap,
        window::{DelegateObserver, Windows},
    },
    window_with_count_and_skip,
};
use crate::{
    disposable::dispose_on_drop::DisposeOnDrop,
    observable::{Observable, ObservableTypes},
    observer::{Observer, boxed_observer::ObserverMode},
    subject::unicast_subject::BoxedUnicastObservable,
    utils::MarkerType,
};
use educe::Educe;
use std::{marker::PhantomData, num::NonZeroUsize};

/// Subdivides the items of an observable into observable windows of a specified number of items.
/// See <https://reactivex.io/documentation/operators/window.html>
///
/// A window is emitted before its first item is delivered, and each window can be subscribed to
/// once. Items emitted while a window has no subscriber are buffered and replayed to a later
/// subscriber; dropping a window without subscribing to it discards its items.
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
///     operators::{
///         creating::from_iter::FromIter, transforming::window_with_count::WindowWithCount,
///     },
/// };
/// use std::num::NonZeroUsize;
///
/// let mut windows = Vec::new();
/// WindowWithCount::new(FromIter::new([1, 2, 3, 4]), NonZeroUsize::new(2).unwrap())
///     .concat_map(|window| window.to_vec())
///     .subscribe_with_callback(|values| windows.push(values), |_| {});
///
/// // A full window opens the next one, which the completion leaves empty.
/// assert_eq!(windows, [vec![1, 2], vec![3, 4], vec![]]);
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct WindowWithCount<'a, OE> {
    source: OE,
    count: NonZeroUsize,
    /// The observer of a window may borrow for `'a`. Unlike the `_boxed` hooks this cannot be left
    /// to an unboxed default: the window is the `Item`, which [`ObservableTypes`] names without any
    /// observer, and its observer arrives only later, so the window boxes it.
    _marker: MarkerType<&'a ()>,
}

impl<OE> WindowWithCount<'_, OE> {
    /// Creates a [`WindowWithCount`] over `source`;
    /// [`ObservableExt::window_with_count`](crate::observable::ObservableExt::window_with_count) is
    /// the fluent form.
    pub fn new(source: OE, count: NonZeroUsize) -> Self {
        Self {
            source,
            count,
            _marker: PhantomData,
        }
    }
}

/// The observer a [`WindowWithCount`] gives its source: that of a
/// [`WindowWithCountAndSkip`](super::window_with_count_and_skip::WindowWithCountAndSkip) whose
/// windows never overlap, so that it does not ask its items to be [`Clone`].
#[doc(hidden)]
pub type WindowWithCountObserver<'a, M, T, E, OR> =
    CountObserver<Windows<T>, DelegateObserver<'a, M, T, E, OR, NoOverlap>>;

impl<'a, T, E, OE> ObservableTypes for WindowWithCount<'a, OE>
where
    <OE as ObservableTypes>::Mode: ObserverMode,
    OE: ObservableTypes<Item = T, Error = E>,
{
    type Item = BoxedUnicastObservable<'a, T, E, OE::Mode>;
    type Error = E;
    type Mode = OE::Mode;
    type Disposal = window_with_count_and_skip::Disposal<OE::Disposal>;
}

impl<'a, T, E, OE, OR> Observable<OR> for WindowWithCount<'a, OE>
where
    <OE as ObservableTypes>::Mode: ObserverMode,
    OR: Observer<BoxedUnicastObservable<'a, T, E, <OE as ObservableTypes>::Mode>, E>,
    E: Clone,
    OE: Observable<
            WindowWithCountObserver<'a, <OE as ObservableTypes>::Mode, T, E, OR>,
            Item = T,
            Error = E,
        >,
{
    fn subscribe(self, observer: OR) -> DisposeOnDrop<Self::Disposal> {
        // A `WindowWithCountAndSkip` whose windows open as the one before is full: a single one is
        // ever open, so no item is cloned (decision 0014).
        window_with_count_and_skip::subscribe(
            self.source,
            observer,
            self.count,
            self.count,
            NoOverlap,
        )
    }
}
