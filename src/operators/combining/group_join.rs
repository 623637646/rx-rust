//! The [`GroupJoin`] operator, behind
//! [`ObservableExt::group_join`](crate::observable::ObservableExt::group_join).

use super::join_core::{
    self, Action, DurationObserverOf, EndingSource, LeftObserverOf, RightObserverOf, Side,
};
pub use super::join_core::{
    Disposal, DisposalOf, DurationObserver, LeftObserver, ModeOf, RightObserver,
};
use crate::{
    disposable::dispose_on_drop::DisposeOnDrop,
    observable::{Observable, ObservableTypes},
    observer::{Flow, Observer, Termination, boxed_observer::ObserverMode},
    subject::unicast_subject::{self, BoxedUnicastObservable, BoxedUnicastSender},
    utils::{MarkerType, id_generator::Id, serialized_delivery::StopView},
};
use educe::Educe;
use std::{collections::BTreeMap, marker::PhantomData};

/// Pairs every value of the left observable with a window of the right values that overlap it:
/// every value opens a window that lasts until the observable its duration selector returns for
/// it emits or completes, and the window of a left value receives the right values whose windows
/// are open when it arrives, then every right value that arrives while its own window is open.
/// See <https://reactivex.io/documentation/operators/join.html>
///
/// The window of a left value completes when its duration emits or completes, or when the right
/// source completes, since no right value can come any more: a left value that arrives after that
/// gets a window that is already complete, holding the right values still open. The stream
/// completes once the left source has completed and every window has completed. An error from
/// either source or from a duration ends the open windows and the stream with that error.
///
/// [`Join`](super::join::Join) is this operator with every window flattened into pairs, except
/// that it completes as soon as no pair can come, while this one keeps emitting a window for every
/// left value until the left source completes.
///
/// Each window is a single-consumer pipe, as those of
/// [`Window`](crate::operators::transforming::window::Window) are: it can be subscribed to once,
/// it buffers the values that arrive while it has no subscriber, and dropping it without
/// subscribing discards them. Disposing the outer subscription ends the open windows without a
/// termination.
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt,
///     observer::{Observer, Termination},
///     operators::{combining::group_join::GroupJoin, creating::empty::Empty},
///     subject::publish_subject::PublishSubject,
/// };
/// use std::convert::Infallible;
///
/// let mut groups = Vec::new();
/// let mut lefts = PublishSubject::local();
/// let mut rights = PublishSubject::<_, Infallible, _>::local();
/// let mut close_lefts = PublishSubject::local();
///
/// let close = close_lefts.clone();
/// let subscription = GroupJoin::new(
///     lefts.clone(),
///     rights.clone(),
///     // A left value stays open until `close_lefts` emits.
///     move |_| close.clone(),
///     // A right value meets only the left values that are open when it arrives.
///     |_| Empty,
/// )
/// .flat_map(|(left, window)| window.to_vec().map(move |rights| (left, rights)))
/// .subscribe_with_callback(
///     |group| groups.push(group),
///     |termination| assert_eq!(termination, Termination::Completed),
/// );
///
/// let _ = lefts.on_next("a");
/// let _ = rights.on_next(1);
/// let _ = lefts.on_next("b");
/// let _ = rights.on_next(2);
/// let _ = close_lefts.on_next(()); // Completes the windows of "a" and "b".
/// lefts.on_termination(Termination::Completed);
///
/// drop((subscription, rights, close_lefts));
/// assert_eq!(groups, [("a", vec![1, 2]), ("b", vec![2])]);
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct GroupJoin<'a, OE, OE2, FL, FR, DL, DR> {
    left: OE,
    right: OE2,
    left_duration_selector: FL,
    right_duration_selector: FR,
    /// The observer of a window may borrow for `'a`: the window is part of the `Item`, which
    /// [`ObservableTypes`] names without any observer, so the window boxes it. The selectors take
    /// references, so their results are not tied to the source by an argument.
    _marker: MarkerType<(&'a (), DL, DR)>,
}

impl<OE, OE2, FL, FR, DL, DR> GroupJoin<'_, OE, OE2, FL, FR, DL, DR> {
    /// Creates a [`GroupJoin`] of `left` and `right`;
    /// [`ObservableExt::group_join`](crate::observable::ObservableExt::group_join) is the fluent
    /// form.
    pub fn new<L, R, E>(
        left: OE,
        right: OE2,
        left_duration_selector: FL,
        right_duration_selector: FR,
    ) -> Self
    where
        OE: ObservableTypes<Item = L, Error = E>,
        OE2: ObservableTypes<Item = R, Error = E>,
        FL: FnMut(&L) -> DL,
        FR: FnMut(&R) -> DR,
        DL: ObservableTypes<Error = E>,
        DR: ObservableTypes<Error = E>,
    {
        Self {
            left,
            right,
            left_duration_selector,
            right_duration_selector,
            _marker: PhantomData,
        }
    }
}

impl<'a, L, R, E, OE, OE2, FL, FR, DL, DR> ObservableTypes
    for GroupJoin<'a, OE, OE2, FL, FR, DL, DR>
where
    ModeOf<OE, OE2, DL, DR>: ObserverMode,
    OE: ObservableTypes<Item = L, Error = E>,
    OE2: ObservableTypes<Item = R, Error = E>,
    DL: ObservableTypes<Error = E>,
    DR: ObservableTypes<Error = E>,
{
    type Item = (L, BoxedUnicastObservable<'a, R, E, ModeOf<OE, OE2, DL, DR>>);
    type Error = E;
    type Mode = ModeOf<OE, OE2, DL, DR>;
    /// The disposal of the machinery shared with [`Join`](super::join::Join).
    type Disposal = DisposalOf<OE, OE2, DL, DR>;
}

/// The [`GroupJoinDelegate`] of a group join of `OE` and `OE2` whose windows last as long as `DL`
/// and `DR`.
type Delegate<'a, OE, OE2, DL, DR, OR> = GroupJoinDelegate<
    'a,
    ModeOf<OE, OE2, DL, DR>,
    OR,
    <OE2 as ObservableTypes>::Item,
    <OE as ObservableTypes>::Error,
>;

impl<'a, L, R, E, OE, OE2, FL, FR, DL, DR, OR> Observable<OR>
    for GroupJoin<'a, OE, OE2, FL, FR, DL, DR>
where
    ModeOf<OE, OE2, DL, DR>: ObserverMode,
    OR: Observer<(L, BoxedUnicastObservable<'a, R, E, ModeOf<OE, OE2, DL, DR>>), E>,
    R: Clone,
    E: Clone,
    OE: Observable<
            LeftObserverOf<OE, OE2, DL, DR, Delegate<'a, OE, OE2, DL, DR, OR>, FL>,
            Item = L,
            Error = E,
        >,
    OE2: Observable<
            RightObserverOf<OE, OE2, DL, DR, Delegate<'a, OE, OE2, DL, DR, OR>, FR>,
            Item = R,
            Error = E,
        >,
    FL: FnMut(&L) -> DL,
    FR: FnMut(&R) -> DR,
    DL: Observable<
            DurationObserverOf<OE, OE2, DL, DR, Delegate<'a, OE, OE2, DL, DR, OR>>,
            Error = E,
        >,
    DR: Observable<
            DurationObserverOf<OE, OE2, DL, DR, Delegate<'a, OE, OE2, DL, DR, OR>>,
            Error = E,
        >,
{
    fn subscribe(self, observer: OR) -> DisposeOnDrop<Self::Disposal> {
        join_core::subscribe(
            (self.left, self.right),
            (self.left_duration_selector, self.right_duration_selector),
            EndingSource::Left,
            |stop_view| GroupJoinDelegate {
                observer,
                stop_view,
                windows: BTreeMap::new(),
                rights: BTreeMap::new(),
            },
        )
    }
}

/// Owns the open windows and right values, so that the windows are fed and emitted outside the
/// lock of the context that serializes the sources against the durations.
#[doc(hidden)]
pub struct GroupJoinDelegate<'a, M: ObserverMode, OR, R, E> {
    observer: OR,
    stop_view: StopView<M>,
    /// The sending end of every open window, by the id of its left value.
    windows: BTreeMap<Id, BoxedUnicastSender<'a, R, E, M>>,
    /// The right values whose windows are open, in the order they arrived: the order a new window
    /// receives them in.
    rights: BTreeMap<Id, R>,
}

impl<'a, M, OR, L, R, E> Observer<Action<L, R>, E> for GroupJoinDelegate<'a, M, OR, R, E>
where
    M: ObserverMode,
    OR: Observer<(L, BoxedUnicastObservable<'a, R, E, M>), E>,
    R: Clone,
    E: Clone,
{
    fn on_next(&mut self, action: Action<L, R>) -> Flow {
        match action {
            Action::Left(id, value) => {
                let (mut sender, window) = unicast_subject::new_boxed();
                // A `Flow::Stop` ends the window's share of the right values only. When a clone
                // disposed the subscription, the stop view stops the emission of the window below.
                let _ = join_core::send_each(
                    &self.stop_view,
                    &mut sender,
                    self.rights.values(),
                    Clone::clone,
                );
                match id {
                    Some(id) => {
                        self.windows.insert(id, sender);
                    }
                    // No right value can come any more.
                    None => sender.on_termination(Termination::Completed),
                }
                return join_core::send(&self.stop_view, &mut self.observer, || (value, window));
            }
            Action::Right(id, value) => {
                // A window whose consumer stopped is dropped, without a termination, and gets no
                // more clones; its duration still decides when its left value stops meeting right
                // values, and so when the stream ends. The consumer of a window, or a clone, may
                // also dispose the subscription, which leaves the rest of them unfed: they are all
                // dropped without a termination then, whether here or with the delegate, which the
                // disposal stops the delivery of: no flow has to tell it.
                let stop_view = &self.stop_view;
                self.windows.retain(|_, sender| {
                    !sender.is_closed()
                        && join_core::send(stop_view, sender, || value.clone()).is_continue()
                });
                if let Some(id) = id {
                    self.rights.insert(id, value);
                }
            }
            Action::Close(Side::Left, id) => {
                if let Some(sender) = self.windows.remove(&id) {
                    join_core::terminate(&self.stop_view, sender, || Termination::Completed);
                }
            }
            Action::Close(Side::Right, id) => drop(self.rights.remove(&id)),
            Action::Completed(Side::Left) => self.rights.clear(),
            Action::Completed(Side::Right) => {
                // The consumer of a window may dispose the subscription on its completion, which
                // drops the rest of them without a termination.
                for sender in std::mem::take(&mut self.windows).into_values() {
                    join_core::terminate(&self.stop_view, sender, || Termination::Completed);
                }
            }
        }
        Flow::Continue
    }

    fn on_termination(self, termination: Termination<E>) {
        // A completion comes once every window has completed, so only an error finds some open;
        // one whose consumer stopped since the last right value is dropped without a clone of it,
        // as a right value drops it. The consumer of a window, or a clone, may dispose the
        // subscription, which ends the rest of the windows, and the stream, without a termination.
        let Self {
            observer,
            stop_view,
            windows,
            ..
        } = self;
        for sender in windows.into_values().filter(|sender| !sender.is_closed()) {
            join_core::terminate(&stop_view, sender, || termination.clone());
        }
        join_core::terminate(&stop_view, observer, || termination);
    }
}
