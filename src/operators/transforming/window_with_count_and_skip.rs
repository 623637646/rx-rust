//! The [`WindowWithCountAndSkip`] operator, behind
//! [`ObservableExt::window_with_count_and_skip`](crate::observable::ObservableExt::window_with_count_and_skip).

use super::internal::{
    count::CountObserver,
    window::{DelegateObserver, Windows},
};
use crate::{
    delegate_disposal,
    disposable::{
        Disposable, DisposableExt, dispose_on_drop::DisposeOnDrop, option_disposal::OptionDisposal,
    },
    observable::{Observable, ObservableTypes},
    observer::{Observer, boxed_observer::ObserverMode},
    subject::unicast_subject::BoxedUnicastObservable,
    utils::MarkerType,
};
use educe::Educe;
use std::{marker::PhantomData, num::NonZeroUsize};

/// Subdivides the items of an observable into observable windows of `count` items each, starting a
/// new window every `skip` items.
/// See <https://reactivex.io/documentation/operators/window.html>
///
/// The first window opens as the operator is subscribed, and the next one after every `skip`
/// items. With `skip` smaller than `count` the windows overlap — each item is sent to every window
/// open when it arrives, cloned into all of them but the newest — and with `skip` larger than
/// `count` the items between two windows are dropped. With `skip` equal to `count` it is
/// [`WindowWithCount`](crate::operators::transforming::window_with_count::WindowWithCount), which
/// does not need the items to be [`Clone`].
///
/// A window is emitted before its first item is delivered, and each window can be subscribed to
/// once. Items emitted while a window has no subscriber are buffered and replayed to a later
/// subscriber; dropping a window without subscribing to it discards its items. The termination of
/// the source terminates every window still open.
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt,
///     operators::{
///         creating::from_iter::FromIter,
///         transforming::window_with_count_and_skip::WindowWithCountAndSkip,
///     },
/// };
/// use std::num::NonZeroUsize;
///
/// let mut windows = Vec::new();
/// WindowWithCountAndSkip::new(
///     FromIter::new([1, 2, 3, 4]),
///     NonZeroUsize::new(3).unwrap(),
///     NonZeroUsize::new(2).unwrap(),
/// )
/// .merge_all()
/// .subscribe_with_callback(|value| windows.push(value), |_| {});
///
/// // The windows start at the items 1 and 3; the one starting at 5 opens empty.
/// assert_eq!(windows, [1, 2, 3, 3, 4]);
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct WindowWithCountAndSkip<'a, OE> {
    source: OE,
    count: NonZeroUsize,
    skip: NonZeroUsize,
    /// The observer of a window may borrow for `'a`. Unlike the `_boxed` hooks this cannot be left
    /// to an unboxed default: the window is the `Item`, which [`ObservableTypes`] names without any
    /// observer, and its observer arrives only later, so the window boxes it.
    _marker: MarkerType<&'a ()>,
}

impl<OE> WindowWithCountAndSkip<'_, OE> {
    /// Creates a [`WindowWithCountAndSkip`] over `source`;
    /// [`ObservableExt::window_with_count_and_skip`](crate::observable::ObservableExt::window_with_count_and_skip)
    /// is the fluent form.
    pub fn new(source: OE, count: NonZeroUsize, skip: NonZeroUsize) -> Self {
        Self {
            source,
            count,
            skip,
            _marker: PhantomData,
        }
    }
}

delegate_disposal!(
    Disposal<D>,
    OptionDisposal<D>,
    where D: Disposable
);

impl<'a, T, E, OE> ObservableTypes for WindowWithCountAndSkip<'a, OE>
where
    <OE as ObservableTypes>::Mode: ObserverMode,
    OE: ObservableTypes<Item = T, Error = E>,
{
    type Item = BoxedUnicastObservable<'a, T, E, OE::Mode>;
    type Error = E;
    type Mode = OE::Mode;
    type Disposal = Disposal<OE::Disposal>;
}

/// The observer a [`WindowWithCountAndSkip`] gives its source.
#[doc(hidden)]
pub type WindowWithCountAndSkipObserver<'a, M, T, E, OR> =
    CountObserver<Windows<T>, DelegateObserver<'a, M, T, E, OR>>;

impl<'a, T, E, OE, OR> Observable<OR> for WindowWithCountAndSkip<'a, OE>
where
    <OE as ObservableTypes>::Mode: ObserverMode,
    OR: Observer<BoxedUnicastObservable<'a, T, E, <OE as ObservableTypes>::Mode>, E>,
    E: Clone,
    T: Clone,
    OE: Observable<
            WindowWithCountAndSkipObserver<'a, <OE as ObservableTypes>::Mode, T, E, OR>,
            Item = T,
            Error = E,
        >,
{
    fn subscribe(self, observer: OR) -> DisposeOnDrop<Self::Disposal> {
        subscribe(self.source, observer, self.count, self.skip, T::clone)
    }
}

/// Subscribes `source` with the observer of a [`WindowWithCountAndSkip`], or, with a `skip` equal
/// to `count` and [`no_overlap`](super::internal::overlap::no_overlap) as the duplicate, of a
/// [`WindowWithCount`](super::window_with_count::WindowWithCount).
pub(super) fn subscribe<'a, M, T, E, OE, OR>(
    source: OE,
    observer: OR,
    count: NonZeroUsize,
    skip: NonZeroUsize,
    duplicate: fn(&T) -> T,
) -> DisposeOnDrop<Disposal<OE::Disposal>>
where
    M: ObserverMode,
    OR: Observer<BoxedUnicastObservable<'a, T, E, M>, E>,
    E: Clone,
    OE: Observable<WindowWithCountAndSkipObserver<'a, M, T, E, OR>, Item = T, Error = E>,
{
    let delegate = DelegateObserver::new(observer, duplicate);
    let mut observer = CountObserver::new(Windows::new(), delegate, count, skip);
    if observer.open_first::<E>().is_break() {
        // The first window ended the stream, so the source is never subscribed to.
        return OptionDisposal::none().into_dispose_on_drop();
    }
    source
        .subscribe(observer)
        .map_inner(OptionDisposal::some)
        .map_inner_into()
}
