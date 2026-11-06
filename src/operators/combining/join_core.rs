//! The machinery shared by [`Join`](super::join::Join) and
//! [`GroupJoin`](super::group_join::GroupJoin): both open a window for every value of either
//! source, which lasts until the duration observable of that value emits or completes, and both
//! serialize everything through one context. They differ only in what they emit, which a delegate
//! observer decides outside the lock from the [`Action`]s this module hands it: a pair for every
//! two values whose windows overlap, or a window of right values for every left value. Neither
//! operator is built on the other: each is a projection of those actions, ended as its
//! [`EndingSource`] says.
//!
//! The module is private, but what a public signature of either operator names stays `pub` and is
//! re-exported by both: the [`Disposal`] and the aliases, and the observers the sources and the
//! durations are subscribed with, since a source built with a non-boxed `Create` hands its
//! observer to user code, which may not hold a value of a private type.

use crate::{
    delegate_disposal,
    disposable::{Disposable, dispose_on_drop::DisposeOnDrop},
    observable::{ChainDisposalOf, JoinedModeOf, Observable, ObservableTypes},
    observer::{EventBatch, Flow, Observer, Termination},
    thread_mode::{Joined, ThreadMode},
    utils::{
        id_generator::Id,
        reservation_map::{Released, ReservationMap},
        serialized_delivery::{StopView, UpdateOutcome, Updated},
        subscribe_with_context::{
            self, SubscriptionContext, subscribe_with_context_two_steps_with_stop_view,
        },
    },
};

/// The thread mode of a join of `OE` and `OE2` whose windows last as long as `DL` and `DR`:
/// `Shared` as soon as one of them is, since the durations are subscribed with the operator's
/// context too.
pub type ModeOf<OE, OE2, DL, DR> = Joined<JoinedModeOf<OE, OE2>, JoinedModeOf<DL, DR>>;

delegate_disposal!(
    /// The disposal of a [`Join`](super::join::Join) or
    /// [`GroupJoin`](super::group_join::GroupJoin) subscription: it stops the context, which tells
    /// the observer that emits the pairs or windows to stop, then disposes the sources and the
    /// durations.
    Disposal<M, L, R, E, SD, DLD, DRD>,
    subscribe_with_context::Disposal<M, Action<L, R>, E, Model<DLD, DRD>, SD>,
    where M: ThreadMode, SD: Disposable, DLD: Disposable, DRD: Disposable
);

/// The [`Disposal`] of a join of `OE` and `OE2` whose windows last as long as `DL` and `DR`.
pub type DisposalOf<OE, OE2, DL, DR> = Disposal<
    ModeOf<OE, OE2, DL, DR>,
    <OE as ObservableTypes>::Item,
    <OE2 as ObservableTypes>::Item,
    <OE as ObservableTypes>::Error,
    ChainDisposalOf<OE2, OE>,
    <DL as ObservableTypes>::Disposal,
    <DR as ObservableTypes>::Disposal,
>;

/// The [`LeftObserver`] of a join of `OE` and `OE2` whose windows last as long as `DL` and `DR`,
/// with the delegate `D`.
pub(super) type LeftObserverOf<OE, OE2, DL, DR, D, FL> = LeftObserver<
    ModeOf<OE, OE2, DL, DR>,
    <OE as ObservableTypes>::Item,
    <OE2 as ObservableTypes>::Item,
    <OE as ObservableTypes>::Error,
    D,
    <DL as ObservableTypes>::Disposal,
    <DR as ObservableTypes>::Disposal,
    ChainDisposalOf<OE2, OE>,
    FL,
>;

/// [`LeftObserverOf`] for the right source.
pub(super) type RightObserverOf<OE, OE2, DL, DR, D, FR> = RightObserver<
    ModeOf<OE, OE2, DL, DR>,
    <OE as ObservableTypes>::Item,
    <OE2 as ObservableTypes>::Item,
    <OE as ObservableTypes>::Error,
    D,
    <DL as ObservableTypes>::Disposal,
    <DR as ObservableTypes>::Disposal,
    ChainDisposalOf<OE2, OE>,
    FR,
>;

/// [`LeftObserverOf`] for the durations.
pub(super) type DurationObserverOf<OE, OE2, DL, DR, D> = DurationObserver<
    ModeOf<OE, OE2, DL, DR>,
    <OE as ObservableTypes>::Item,
    <OE2 as ObservableTypes>::Item,
    <OE as ObservableTypes>::Error,
    D,
    <DL as ObservableTypes>::Disposal,
    <DR as ObservableTypes>::Disposal,
    ChainDisposalOf<OE2, OE>,
>;

/// Subscribes both sources with observers sharing one context, ended as `ending_source` says,
/// whose observer is the delegate that `delegate_builder` builds around the [`StopView`] of the
/// context.
pub(super) fn subscribe<L, R, E, OR, OE, OE2, FL, FR, DL, DR>(
    (left, right): (OE, OE2),
    (left_duration_selector, right_duration_selector): (FL, FR),
    ending_source: EndingSource,
    delegate_builder: impl FnOnce(StopView<ModeOf<OE, OE2, DL, DR>>) -> OR,
) -> DisposeOnDrop<DisposalOf<OE, OE2, DL, DR>>
where
    OR: Observer<Action<L, R>, E>,
    OE: Observable<LeftObserverOf<OE, OE2, DL, DR, OR, FL>, Item = L, Error = E>,
    OE2: Observable<RightObserverOf<OE, OE2, DL, DR, OR, FR>, Item = R, Error = E>,
    FL: FnMut(&L) -> DL,
    FR: FnMut(&R) -> DR,
    DL: Observable<DurationObserverOf<OE, OE2, DL, DR, OR>, Error = E>,
    DR: Observable<DurationObserverOf<OE, OE2, DL, DR, OR>, Error = E>,
{
    subscribe_with_context_two_steps_with_stop_view(
        delegate_builder,
        Model::new(ending_source),
        |context| {
            Some(left.subscribe(LeftObserver {
                context: context.clone(),
                duration_selector: left_duration_selector,
            }))
        },
        |context| {
            Some(right.subscribe(RightObserver {
                context: context.clone(),
                duration_selector: right_duration_selector,
            }))
        },
    )
    .map_inner_into()
}

/// What the observers of a join ask the delegate to do, in the order they decided it under the
/// lock.
pub(super) enum Action<L, R> {
    /// A left value arrived: it meets the right values that are open. It is kept under its id
    /// while its window is open, and not at all when it opens none.
    Left(Option<Id>, L),
    /// A right value arrived, as [`Action::Left`].
    Right(Option<Id>, R),
    /// The window `id` of a value of that side closed.
    Close(Side, Id),
    /// The source of that side completed: the values of the other side are open for values that
    /// will not come.
    Completed(Side),
}

/// The duration subscriptions of the open windows, and what decides when the stream ends.
struct Model<DLD: Disposable, DRD: Disposable> {
    /// The windows of the left values, `None` once the right source has completed: a left value
    /// waits for right values only, so from then on no window of one stays open, nor opens.
    lefts: Windows<DLD>,
    /// The windows of the right values, `None` once the left source has completed, as `lefts`.
    rights: Windows<DRD>,
    ending_source: EndingSource,
}

// A delegate emits several events for one action, or runs user code before an emission, while the
// delivery checks for a stop between two actions only. So it checks the `StopView` of its context
// before each emission, through `StopView::send`, `send_each` and `terminate`: a disposal from
// inside one of those emissions or that user code, or from another thread meanwhile, reaches the
// rest of them only through it.

/// Which source ends the stream once it has completed and the windows of its values have closed:
/// what is left of the other source then has nothing to meet.
#[derive(Clone, Copy)]
pub(super) enum EndingSource {
    /// Either source, as a join, whose values all wait for the other source's.
    Either,
    /// The left source only, as a group join, which still emits a window for every left value
    /// once the right source has finished.
    Left,
}

/// The duration subscription of each open window of one source's values, under an id never
/// reused, so that a late duration observer cannot close another window; `None` once the other
/// source has completed.
type Windows<D> = Option<ReservationMap<DisposeOnDrop<D>>>;

/// Which source a window belongs to. The ids of the two sides are handed out independently, so a
/// window is known by its side and its id together.
#[derive(Clone, Copy)]
pub(super) enum Side {
    Left,
    Right,
}

impl<DLD: Disposable, DRD: Disposable> Model<DLD, DRD> {
    /// Creates a model ended as `ending_source` says.
    fn new(ending_source: EndingSource) -> Self {
        Self {
            lefts: Some(ReservationMap::default()),
            rights: Some(ReservationMap::default()),
            ending_source,
        }
    }

    fn is_open(&self, side: Side, id: Id) -> bool {
        match side {
            Side::Left => self.lefts.as_ref().is_some_and(|lefts| lefts.contains(id)),
            Side::Right => self
                .rights
                .as_ref()
                .is_some_and(|rights| rights.contains(id)),
        }
    }

    /// Whether nothing can come any more: a completed source whose windows have all closed leaves
    /// the other source's values nothing to meet. A completed source also closes the windows of
    /// the other, so two completed sources always end the stream.
    fn is_finished(&self) -> bool {
        match (&self.lefts, &self.rights) {
            (None, None) => true,
            (Some(lefts), None) => lefts.is_empty(),
            (None, Some(rights)) => {
                matches!(self.ending_source, EndingSource::Either) && rights.is_empty()
            }
            (Some(_), Some(_)) => false,
        }
    }

    /// The events that hand `action` to the delegate, followed by the completion if nothing can
    /// come any more.
    fn events_ending_if_finished<L, R, E>(
        &self,
        action: Action<L, R>,
    ) -> EventBatch<Action<L, R>, E> {
        if self.is_finished() {
            EventBatch::NextAndTermination(action, Termination::Completed)
        } else {
            EventBatch::Next(action)
        }
    }
}

type JoinContext<M, L, R, E, OR, DLD, DRD, SD> =
    SubscriptionContext<M, Action<L, R>, E, OR, Model<DLD, DRD>, SD>;

macro_rules! impl_source_observer {
    ($name:ident, $input:ident, $disposal:ident, $this:ident, $other:ident, $side:ident) => {
        #[doc(hidden)]
        pub struct $name<
            M: ThreadMode,
            L,
            R,
            E,
            OR,
            DLD: Disposable,
            DRD: Disposable,
            SD: Disposable,
            F,
        > {
            context: JoinContext<M, L, R, E, OR, DLD, DRD, SD>,
            duration_selector: F,
        }

        impl<M, L, R, E, OR, DLD, DRD, SD, F, DO> Observer<$input, E>
            for $name<M, L, R, E, OR, DLD, DRD, SD, F>
        where
            M: ThreadMode,
            OR: Observer<Action<L, R>, E>,
            DLD: Disposable,
            DRD: Disposable,
            SD: Disposable,
            F: FnMut(&$input) -> DO,
            DO: Observable<
                    DurationObserver<M, L, R, E, OR, DLD, DRD, SD>,
                    Error = E,
                    Disposal = $disposal,
                >,
        {
            fn on_next(&mut self, value: $input) -> Flow {
                let duration = (self.duration_selector)(&value);
                let updated = self.context.update(|model| {
                    let id = model.$this.as_mut().map(ReservationMap::reserve);
                    UpdateOutcome::new(id).with_next_event(Action::$side(id, value))
                });
                let Updated::Ran {
                    result: Some(id),
                    flow: Flow::Continue,
                } = updated
                else {
                    return updated.flow();
                };
                let subscription = duration.subscribe(DurationObserver {
                    context: self.context.clone(),
                    side: Side::$side,
                    id,
                });
                // A window closed during subscription gives the subscription back to drop
                // outside the lock. Return this update's flow: the duration may have ended the
                // stream, and a source still emitting from its own subscribe has no disposal yet.
                self.context
                    .update(|model| {
                        let closed = match &mut model.$this {
                            Some(windows) => windows.fill(id, subscription),
                            // Closed with the others, by the completion of the other source.
                            None => Some(subscription),
                        };
                        UpdateOutcome::empty()
                            .without_events()
                            .with_drop_outside(closed)
                    })
                    .flow()
            }

            fn on_termination(self, termination: Termination<E>) {
                match termination {
                    Termination::Completed => {
                        let _ = self.context.update(|model| {
                            // The other side's windows wait for values that will not come.
                            let closed = model.$other.take();
                            UpdateOutcome::empty()
                                .with_drop_outside(closed)
                                .with_events(
                                    model.events_ending_if_finished(Action::Completed(Side::$side)),
                                )
                        });
                    }
                    error @ Termination::Error(_) => self.context.send_termination(error),
                }
            }
        }
    };
}

impl_source_observer!(LeftObserver, L, DLD, lefts, rights, Left);
impl_source_observer!(RightObserver, R, DRD, rights, lefts, Right);

/// Closes the window of one value, when its duration emits or completes.
#[doc(hidden)]
pub struct DurationObserver<
    M: ThreadMode,
    L,
    R,
    E,
    OR,
    DLD: Disposable,
    DRD: Disposable,
    SD: Disposable,
> {
    context: JoinContext<M, L, R, E, OR, DLD, DRD, SD>,
    side: Side,
    id: Id,
}

/// Closes the window `id` of `side`, and completes the stream if nothing can come any more.
fn close<M, L, R, E, OR, DLD, DRD, SD>(
    context: &JoinContext<M, L, R, E, OR, DLD, DRD, SD>,
    side: Side,
    id: Id,
) where
    M: ThreadMode,
    OR: Observer<Action<L, R>, E>,
    DLD: Disposable,
    DRD: Disposable,
    SD: Disposable,
{
    match side {
        Side::Left => close_window(context, side, id, |model| &mut model.lefts),
        Side::Right => close_window(context, side, id, |model| &mut model.rights),
    }
}

/// Closes the window `id` among the windows `windows` picks, those of `side`, and completes the
/// stream if nothing can come any more.
fn close_window<M, L, R, E, OR, DLD, DRD, SD, D>(
    context: &JoinContext<M, L, R, E, OR, DLD, DRD, SD>,
    side: Side,
    id: Id,
    windows: impl FnOnce(&mut Model<DLD, DRD>) -> &mut Windows<D>,
) where
    M: ThreadMode,
    OR: Observer<Action<L, R>, E>,
    DLD: Disposable,
    DRD: Disposable,
    SD: Disposable,
    D: Disposable,
{
    let _ = context.update(|model| {
        let released = windows(model)
            .as_mut()
            .map_or(Released::Absent, |windows| windows.release(id));
        if matches!(released, Released::Absent) {
            // Closed already, by the completion of the other source.
            return UpdateOutcome::empty()
                .without_drop_outside()
                .without_events();
        }
        UpdateOutcome::empty()
            .with_drop_outside(released)
            .with_events(model.events_ending_if_finished(Action::Close(side, id)))
    });
}

impl<M, L, R, E, OR, DLD, DRD, SD, T> Observer<T, E>
    for DurationObserver<M, L, R, E, OR, DLD, DRD, SD>
where
    M: ThreadMode,
    OR: Observer<Action<L, R>, E>,
    DLD: Disposable,
    DRD: Disposable,
    SD: Disposable,
{
    fn on_next(&mut self, _: T) -> Flow {
        close(&self.context, self.side, self.id);
        // The window is closed: nothing more is needed from the duration.
        Flow::Stop
    }

    fn on_termination(self, termination: Termination<E>) {
        match termination {
            Termination::Completed => close(&self.context, self.side, self.id),
            Termination::Error(error) => {
                let _ = self.context.update(|model| {
                    if model.is_open(self.side, self.id) {
                        UpdateOutcome::empty()
                            .with_termination_event(Termination::Error(error))
                            .without_drop_outside()
                    } else {
                        // The window was closed, and its duration disposed, before the error
                        // arrived.
                        UpdateOutcome::empty()
                            .without_events()
                            .with_drop_outside(error)
                    }
                });
            }
        }
    }
}
