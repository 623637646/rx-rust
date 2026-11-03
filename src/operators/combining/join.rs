//! The [`Join`] operator, behind [`ObservableExt::join`](crate::observable::ObservableExt::join).

use super::join_core::{
    self, Action, DurationObserverOf, EndingSource, LeftObserverOf, RightObserverOf, Side,
};
pub use super::join_core::{
    Disposal, DisposalOf, DurationObserver, LeftObserver, ModeOf, RightObserver,
};
use crate::{
    disposable::dispose_on_drop::DisposeOnDrop,
    observable::{Observable, ObservableTypes},
    observer::{Flow, Observer, Termination},
    thread_mode::ThreadMode,
    utils::{MarkerType, id_generator::Id, serialized_delivery::StopView},
};
use educe::Educe;
use std::{collections::BTreeMap, marker::PhantomData};

/// Combines the values of two observables whose windows overlap: every value opens a window that
/// lasts until the observable its duration selector returns for it emits or completes, and a value
/// is paired with every value of the other source whose window is open when it arrives.
/// See <https://reactivex.io/documentation/operators/join.html>
///
/// The pairs of a value are emitted when it arrives, in the order the values of the other source
/// arrived in; a window that closes emits nothing. A duration that completes at once (`Empty`)
/// makes its value meet only the values that are open when it arrives, and one that never ends
/// (`Never`) keeps it for as long as the other source runs.
///
/// The stream completes as soon as no pair can come any more: once one source has completed and
/// the windows of its values have all closed. An error from either source or from a duration ends
/// it with that error. A value of one source arriving after the other has completed is still
/// paired with the open values of the other, but opens no window of its own, so its duration
/// selector is called and the duration it returns is dropped unsubscribed.
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt,
///     observer::{Observer, Termination},
///     operators::{combining::join::Join, creating::empty::Empty},
///     subject::publish_subject::PublishSubject,
/// };
/// use std::convert::Infallible;
///
/// let mut pairs = Vec::new();
/// let mut lefts = PublishSubject::local();
/// let mut rights = PublishSubject::<_, Infallible, _>::local();
/// let mut close_lefts = PublishSubject::local();
///
/// let close = close_lefts.clone();
/// let subscription = Join::new(
///     lefts.clone(),
///     rights.clone(),
///     // A left value stays open until `close_lefts` emits.
///     move |_| close.clone(),
///     // A right value meets only the left values that are open when it arrives.
///     |_| Empty,
///     |left, right| format!("{left}{right}"),
/// )
/// .subscribe_with_callback(
///     |pair| pairs.push(pair),
///     |termination| assert_eq!(termination, Termination::Completed),
/// );
///
/// let _ = lefts.on_next("a");
/// let _ = rights.on_next(1);
/// let _ = lefts.on_next("b");
/// let _ = rights.on_next(2);
/// let _ = close_lefts.on_next(()); // Closes the windows of "a" and "b".
/// let _ = rights.on_next(3);
/// rights.on_termination(Termination::Completed); // No right value is open: no pair can come.
///
/// drop((subscription, lefts, close_lefts));
/// assert_eq!(pairs, ["a1", "a2", "b2"]);
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct Join<OE, OE2, FL, FR, F, DL, DR, T> {
    left: OE,
    right: OE2,
    left_duration_selector: FL,
    right_duration_selector: FR,
    result_selector: F,
    /// The selectors take references, so their results are not tied to the source by an argument.
    _marker: MarkerType<(DL, DR, T)>,
}

impl<OE, OE2, FL, FR, F, DL, DR, T> Join<OE, OE2, FL, FR, F, DL, DR, T> {
    /// Creates a [`Join`] of `left` and `right`;
    /// [`ObservableExt::join`](crate::observable::ObservableExt::join) is the fluent form.
    pub fn new<L, R, E>(
        left: OE,
        right: OE2,
        left_duration_selector: FL,
        right_duration_selector: FR,
        result_selector: F,
    ) -> Self
    where
        OE: ObservableTypes<Item = L, Error = E>,
        OE2: ObservableTypes<Item = R, Error = E>,
        FL: FnMut(&L) -> DL,
        FR: FnMut(&R) -> DR,
        DL: ObservableTypes<Error = E>,
        DR: ObservableTypes<Error = E>,
        F: FnMut(&L, &R) -> T,
    {
        Self {
            left,
            right,
            left_duration_selector,
            right_duration_selector,
            result_selector,
            _marker: PhantomData,
        }
    }
}

impl<L, R, T, E, OE, OE2, FL, FR, F, DL, DR> ObservableTypes for Join<OE, OE2, FL, FR, F, DL, DR, T>
where
    OE: ObservableTypes<Item = L, Error = E>,
    OE2: ObservableTypes<Item = R, Error = E>,
    DL: ObservableTypes<Error = E>,
    DR: ObservableTypes<Error = E>,
{
    type Item = T;
    type Error = E;
    type Mode = ModeOf<OE, OE2, DL, DR>;
    type Disposal = DisposalOf<OE, OE2, DL, DR>;
}

/// The [`JoinDelegate`] of a join of `OE` and `OE2` whose windows last as long as `DL` and `DR`.
type Delegate<OE, OE2, DL, DR, OR, F> = JoinDelegate<
    ModeOf<OE, OE2, DL, DR>,
    OR,
    F,
    <OE as ObservableTypes>::Item,
    <OE2 as ObservableTypes>::Item,
>;

impl<L, R, T, E, OE, OE2, FL, FR, F, DL, DR, OR> Observable<OR>
    for Join<OE, OE2, FL, FR, F, DL, DR, T>
where
    OR: Observer<T, E>,
    OE: Observable<
            LeftObserverOf<OE, OE2, DL, DR, Delegate<OE, OE2, DL, DR, OR, F>, FL>,
            Item = L,
            Error = E,
        >,
    OE2: Observable<
            RightObserverOf<OE, OE2, DL, DR, Delegate<OE, OE2, DL, DR, OR, F>, FR>,
            Item = R,
            Error = E,
        >,
    FL: FnMut(&L) -> DL,
    FR: FnMut(&R) -> DR,
    DL: Observable<DurationObserverOf<OE, OE2, DL, DR, Delegate<OE, OE2, DL, DR, OR, F>>, Error = E>,
    DR: Observable<DurationObserverOf<OE, OE2, DL, DR, Delegate<OE, OE2, DL, DR, OR, F>>, Error = E>,
    F: FnMut(&L, &R) -> T,
{
    fn subscribe(self, observer: OR) -> DisposeOnDrop<Self::Disposal> {
        join_core::subscribe(
            (self.left, self.right),
            (self.left_duration_selector, self.right_duration_selector),
            EndingSource::Either,
            |stop_view| JoinDelegate {
                observer,
                result_selector: self.result_selector,
                stop_view,
                lefts: BTreeMap::new(),
                rights: BTreeMap::new(),
            },
        )
    }
}

/// Owns the open values and the result selector, so that the pairs are made and emitted outside
/// the lock of the context that serializes the sources against the durations.
#[doc(hidden)]
pub struct JoinDelegate<M: ThreadMode, OR, F, L, R> {
    observer: OR,
    result_selector: F,
    stop_view: StopView<M>,
    /// The left values whose windows are open, in the order they arrived: the order they are
    /// paired in.
    lefts: BTreeMap<Id, L>,
    /// The right values whose windows are open, as `lefts`.
    rights: BTreeMap<Id, R>,
}

impl<M, OR, F, L, R, T, E> Observer<Action<L, R>, E> for JoinDelegate<M, OR, F, L, R>
where
    M: ThreadMode,
    OR: Observer<T, E>,
    F: FnMut(&L, &R) -> T,
{
    fn on_next(&mut self, action: Action<L, R>) -> Flow {
        match action {
            Action::Left(id, value) => {
                let pair = |right: &R| (self.result_selector)(&value, right);
                let others = self.rights.values();
                if join_core::send_each(&self.stop_view, &mut self.observer, others, pair).is_stop()
                {
                    return Flow::Stop;
                }
                if let Some(id) = id {
                    self.lefts.insert(id, value);
                }
            }
            Action::Right(id, value) => {
                let pair = |left: &L| (self.result_selector)(left, &value);
                let others = self.lefts.values();
                if join_core::send_each(&self.stop_view, &mut self.observer, others, pair).is_stop()
                {
                    return Flow::Stop;
                }
                if let Some(id) = id {
                    self.rights.insert(id, value);
                }
            }
            Action::Close(Side::Left, id) => {
                self.lefts.remove(&id);
            }
            Action::Close(Side::Right, id) => {
                self.rights.remove(&id);
            }
            Action::Completed(Side::Left) => self.rights.clear(),
            Action::Completed(Side::Right) => self.lefts.clear(),
        }
        Flow::Continue
    }

    fn on_termination(self, termination: Termination<E>) {
        self.observer.on_termination(termination);
    }
}
