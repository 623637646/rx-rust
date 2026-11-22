//! The state machine shared by the operators that merge a source set — a tuple or a list of
//! observables, pulled as there is room for them: [`Merge`](super::super::merge::Merge), with or
//! without a `max_concurrency`, and [`ConcatEager`](super::super::concat_eager::ConcatEager),
//! which [`Concat`](super::super::concat::Concat) over a list is too, as a `Merge` with a
//! `max_concurrency` of one.
//!
//! One [`Driver`] pulls the sources from the source set's cursor, at most `max_concurrency` of
//! them at a time, and nothing waits in a queue: once there is no room, the cursor itself waits in
//! the model for the completion that makes some. Each source is subscribed with a
//! [`MergeObserver`]. What sets the operators apart is their sources and the observer the merge
//! is subscribed with:
//!
//! | Operator      | `max_concurrency` | Sources                                 | Observer          |
//! |---------------|-------------------|-----------------------------------------|-------------------|
//! | `Merge`       | none, or any      | its own                                 | the downstream    |
//! | `Concat` list | one               | its own, through `Merge`                | the downstream    |
//! | `ConcatEager` | none, or any      | `IndexedSource` tags events by position | `ReorderObserver` |
//!
//! Everything else is written once: the place reserved before a source is pulled, under the lock
//! that stores the subscription before it; the subscription held in the slot of its position from
//! its subscription to its completion, and handed back to drop outside the lock when the source
//! completed first; the sources pulled in a loop rather than by recursion, however many of them
//! complete as they are subscribed; the stream given up when a panic leaves a place taken; and
//! its completion once every source has been pulled and has completed. With no limit there is
//! always room, so the cursor never waits and its type is `Infallible` in the model's.
//!
//! Other source-set operators have their own state machines. A `Concat` tuple hands the
//! downstream observer from each pair to the next and forwards values without locking. `Amb`
//! disposes the losers when a source wins and forwards the winner's events without locking. `Zip`,
//! `CombineLatest`, `ForkJoin` and `WithLatestFrom` hold items of different types at typed
//! positions, need their position count before subscribing, and end on rules of their own; the
//! operators whose two sources have roles (`take_until`, `sample`, `sequence_equal`, …) end
//! with one of them. An observable of observables pushes its sources rather than being pulled,
//! and goes to [`flatten`](super::flatten) instead.
//!
//! The module is private, but what a public signature of those operators names stays `pub` and
//! is re-exported by them: the observer each source is subscribed with, since a source built with
//! a non-boxed `Create` hands it to user code, and the tags in its generic arguments.

use crate::{
    disposable::dispose_on_drop::DisposeOnDrop,
    observable::{
        ObservableTypes, SourceSet,
        sources::{SourceOps, SourceSubscriber, SubscribeEach},
    },
    observer::{Flow, Observer, Termination},
    thread_mode::ThreadMode,
    utils::{
        on_panic::on_panic,
        positional::{position::Position, slots::ReservationSlots},
        serialized_delivery::StopView,
        subscribe_with_context::{
            self, DropUndecided, SubscriptionContext, UpdateOutcome, Updated,
            subscribe_with_context_with_stop_view,
        },
    },
};
use educe::Educe;
use std::{convert::Infallible, num::NonZeroUsize, ops::ControlFlow};

/// The limit of a [`Merge`](super::super::merge::Merge) or a
/// [`ConcatEager`](super::super::concat_eager::ConcatEager) that has none, their default: every
/// source is subscribed at once. A limit is a `NonZeroUsize`, given by their `max_concurrency`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Unlimited;

/// The [`Driver`] of a merge over `S` sending `T`s to `OR`, whose sources wait for room as a `C`.
pub(crate) type DriverOf<S, T, OR, C> = Driver<
    <S as SourceSet>::Mode,
    T,
    <S as SourceSet>::Error,
    OR,
    C,
    <S as SourceSet>::SubscriptionSlots,
>;

/// The disposal of the context of a merge sending `T`s, whose sources wait for room as a `C` and
/// are kept in the slots `SL`: each operator names it through a `delegate_disposal!` of its own.
pub(crate) type ContextDisposal<M, T, E, C, SL> =
    subscribe_with_context::Disposal<M, T, E, Model<C, SL>, ()>;

/// The [`ContextDisposal`] of a merge over `S` sending `T`s, whose sources wait for room as a `C`.
pub(crate) type ContextDisposalOf<S, T, C> = ContextDisposal<
    <S as SourceSet>::Mode,
    T,
    <S as SourceSet>::Error,
    C,
    <S as SourceSet>::SubscriptionSlots,
>;

/// Subscribes a merge with no limit, of the `sources`, with the observer that `observer_builder`
/// builds around the [`StopView`] of its context.
pub(crate) fn subscribe_unlimited<S, T, OR, B>(
    sources: S,
    observer_builder: B,
) -> DisposeOnDrop<ContextDisposalOf<S, T, Infallible>>
where
    S: SourceOps,
    <S as SourceSet>::Cursor: for<'a> SubscribeEach<&'a DriverOf<S, T, OR, Infallible>, Pause>,
    OR: Observer<T, <S as SourceSet>::Error>,
    B: FnOnce(StopView<<S as SourceSet>::Mode>) -> OR,
{
    // There is always room, so the sources never wait: they stay with this call, and neither
    // the observers nor the subscription name them.
    subscribe_driver(
        observer_builder,
        NonZeroUsize::MAX,
        sources.into_cursor(),
        |_| unreachable!("a merge with no limit always has room"),
        |parked, _| match parked {},
    )
}

/// Subscribes a merge of at most `max_concurrency` of the `sources` at once, with the observer
/// that `observer_builder` builds around the [`StopView`] of its context.
pub(crate) fn subscribe_limited<S, T, OR, B>(
    sources: S,
    max_concurrency: NonZeroUsize,
    observer_builder: B,
) -> DisposeOnDrop<ContextDisposalOf<S, T, <S as SourceSet>::Cursor>>
where
    S: SourceOps,
    <S as SourceSet>::Cursor:
        for<'a> SubscribeEach<&'a DriverOf<S, T, OR, <S as SourceSet>::Cursor>, Pause>,
    OR: Observer<T, <S as SourceSet>::Error>,
    B: FnOnce(StopView<<S as SourceSet>::Mode>) -> OR,
{
    subscribe_driver(
        observer_builder,
        max_concurrency,
        sources.into_cursor(),
        std::convert::identity,
        |parked, driver| driver.run(parked, std::convert::identity),
    )
}

/// Subscribes a merge of at most `max_concurrency` sources at once: runs the `cursor` until there
/// is no room, then `park`s it, and `resume` goes on with the sources that waited for room.
fn subscribe_driver<M, T, E, OR, X, C, SL>(
    observer_builder: impl FnOnce(StopView<M>) -> OR,
    max_concurrency: NonZeroUsize,
    cursor: X,
    park: fn(X) -> C,
    resume: DriverResumeFn<M, T, E, OR, C, SL>,
) -> DisposeOnDrop<ContextDisposal<M, T, E, C, SL>>
where
    M: ThreadMode,
    OR: Observer<T, E>,
    X: for<'a> SubscribeEach<&'a Driver<M, T, E, OR, C, SL>, Pause>,
    SL: Default,
{
    let model = Model {
        cursor: CursorState::Held,
        // The place of the first source, which is pulled at once.
        occupied: 1,
        slots: SL::default(),
        max_concurrency,
    };
    subscribe_with_context_with_stop_view(observer_builder, model, |context| {
        Driver { context, resume }.run(cursor, park);
        // The subscriptions are in the model, which the context drops as it stops.
        DisposeOnDrop::default()
    })
}

/// The state of a merge: the sources not pulled yet, the places taken, and the subscriptions.
pub(crate) struct Model<C, SL> {
    /// Where the sources not pulled yet are.
    cursor: CursorState<C>,
    /// How many places are taken, never more than `max_concurrency`: one by each source
    /// subscribed and not completed, and one by the next source while the driver has room to
    /// pull it. That place is taken before the source is pulled — by the `store` of the one
    /// before, under the same lock — since a source pulled must be subscribed: nothing waits in
    /// a queue. Only [`reserve`](Self::reserve) takes a place and only
    /// [`release_place`](Self::release_place) gives one up; a completion that finds the sources
    /// parked hands its place to the next one instead.
    occupied: usize,
    /// The subscription of each source, from its subscription to its completion. A source that
    /// completes before its subscription is stored releases its slot, and the subscription is then
    /// handed back, to drop outside the lock.
    slots: SL,
    /// `NonZeroUsize::MAX` with no limit: more sources than can be counted.
    max_concurrency: NonZeroUsize,
}

/// Where the sources of a merge that are not pulled yet are.
enum CursorState<C> {
    /// With the one thread pulling them, outside the lock.
    Held,
    /// Waiting in the model for room, with its remaining elements and their positions. With no
    /// limit `C` is [`Infallible`]: there is always room.
    Parked(C),
    /// Nowhere: every source has been pulled.
    Exhausted,
}

impl<C> CursorState<C> {
    /// Takes the sources out if they are parked, leaving them held.
    fn take_parked(&mut self) -> Option<C> {
        match std::mem::replace(self, Self::Held) {
            Self::Parked(cursor) => Some(cursor),
            other => {
                *self = other;
                None
            }
        }
    }
}

impl<C, SL> Model<C, SL> {
    /// Takes the place of the next source if there is room for it.
    fn reserve(&mut self) -> bool {
        let has_room = self.occupied < self.max_concurrency.get();
        if has_room {
            self.occupied += 1;
        }
        has_room
    }

    /// Gives a place up, and completes the stream along with `outcome` if it was the last one
    /// and every source has been pulled. The cursor's state is checked too: between a `store`
    /// that found no room and the parking of the sources, the driver holds them without a place.
    fn release_place<T, E, R>(
        &mut self,
        outcome: UpdateOutcome<T, E, R>,
    ) -> UpdateOutcome<T, E, R, DropUndecided, true> {
        self.occupied -= 1;
        if matches!(self.cursor, CursorState::Exhausted) && self.occupied == 0 {
            outcome.with_termination_event(Termination::Completed)
        } else {
            outcome.without_events()
        }
    }
}

/// Subscribes the sources as there is room for them: the context, and what resumes the sources
/// that waited for room, a function captured where the bounds that allow it hold, as
/// [`SubscribeFn`](crate::utils::subscribe_fn::SubscribeFn) does. An observer that completes may
/// resume them; stating its bounds on the `Observer` impl would make that impl depend on itself.
#[derive(Educe)]
#[educe(Clone(bound()))]
pub(crate) struct Driver<M: ThreadMode, T, E, OR, C, SL> {
    context: SubscriptionContext<M, T, E, OR, Model<C, SL>, ()>,
    resume: ResumeFn<C, Self>,
}

/// Goes on with the sources of `C` that waited for room: [`Driver::run`] over them.
type ResumeFn<C, D> = fn(C, &D);

/// The [`ResumeFn`] of a [`Driver`].
type DriverResumeFn<M, T, E, OR, C, SL> = ResumeFn<C, Driver<M, T, E, OR, C, SL>>;

/// Why subscribing stops before the last source.
pub(crate) enum Pause {
    /// No room: park the remaining sources unless a completion has made room since.
    Full,
    /// The stream is over.
    Stopped,
}

impl<M, T, E, OR, C, SL> Driver<M, T, E, OR, C, SL>
where
    M: ThreadMode,
    OR: Observer<T, E>,
{
    /// Pulls and subscribes the next source, whose place is taken, then the next ones
    /// for as long as there is room. Once there is none, the cursor waits in the model, as `park`
    /// makes it, for the completion that makes some, which pulls the next source itself. A source
    /// that completes as it is subscribed only frees its place for this loop, so a chain of them
    /// never recurses.
    ///
    /// The cursor is an `X` rather than the `C` that waits: with no limit nothing ever waits, and
    /// the sources stay out of the model's type.
    fn run<X>(&self, mut cursor: X, park: fn(X) -> C)
    where
        X: for<'a> SubscribeEach<&'a Self, Pause>,
    {
        // A panic while pulling — in `Iterator::next`, in a source's `subscribe`, or in the
        // downstream observer a source emits to as it is subscribed — would leave a place taken
        // and the cursor gone, so that nothing pulls or completes again: give the stream up, as
        // `subscribe_with_context` does when its builder panics.
        let _guard = on_panic(|| self.context.stop());
        // The driver is a shared handle, so the subscriber is `&Driver`.
        let mut subscriber = self;
        loop {
            match cursor.subscribe_each(&mut subscriber) {
                ControlFlow::Break(Pause::Full) => {
                    let updated = self.context.update(move |model| {
                        // A completion may have made room since the subscription was stored.
                        if model.reserve() {
                            UpdateOutcome::new(Some(cursor))
                        } else {
                            model.cursor = CursorState::Parked(park(cursor));
                            UpdateOutcome::new(None)
                        }
                    });
                    // A stopped context dropped the cursor outside the lock, and so does a stop
                    // here.
                    let Updated::Ran {
                        result: Some(kept),
                        flow: Flow::Continue(()),
                    } = updated
                    else {
                        return;
                    };
                    cursor = kept;
                }
                ControlFlow::Continue(()) => {
                    let _ = self.context.update(|model| {
                        model.cursor = CursorState::Exhausted;
                        // The place taken for a source that there was not.
                        model.release_place(UpdateOutcome::empty())
                    });
                    return;
                }
                ControlFlow::Break(Pause::Stopped) => return,
            }
        }
    }

    /// Breaks once the stream is over.
    fn ensure_running(&self) -> ControlFlow<Pause> {
        if self.context.is_active() {
            ControlFlow::Continue(())
        } else {
            ControlFlow::Break(Pause::Stopped)
        }
    }
}

/// Subscribes each source with an observer holding the driver, and stores its subscription in the
/// model, where its completion releases it.
impl<M, T, E, OR, C, SL, K, OE> SourceSubscriber<K, OE, Pause> for &Driver<M, T, E, OR, C, SL>
where
    M: ThreadMode,
    OR: Observer<T, E>,
    K: Position,
    OE: ObservableTypes<Error = E>,
    SL: ReservationSlots<K, Value = DisposeOnDrop<<OE as ObservableTypes>::Disposal>>,
{
    type Observer = MergeObserver<M, T, E, OR, C, SL, K>;

    fn still_running(&self) -> ControlFlow<Pause> {
        self.ensure_running()
    }

    fn observer(&mut self, position: K) -> Option<Self::Observer> {
        Some(MergeObserver {
            driver: Driver::clone(self),
            position,
        })
    }

    fn store(
        &mut self,
        position: K,
        subscription: DisposeOnDrop<<OE as ObservableTypes>::Disposal>,
    ) -> ControlFlow<Pause> {
        // One update stores the subscription and, with the lock still held, takes the place of
        // the next source if there is room.
        let updated = self.context.update(|model| {
            let late = model.slots.fill(position, subscription);
            let next = if model.reserve() {
                ControlFlow::Continue(())
            } else {
                ControlFlow::Break(Pause::Full)
            };
            UpdateOutcome::new((next, late.is_some())).with_drop_outside(late)
        });
        // The source may have ended the stream as it was subscribed: only a continuing flow goes
        // on.
        match updated {
            Updated::Ran {
                result: (next, released),
                flow: Flow::Continue(()),
            } => {
                // Disposing a synchronously completed source can re-enter and stop the stream
                // after the update decided its flow. Check only when that disposal ran.
                if released {
                    self.ensure_running()?;
                }
                next
            }
            _ => ControlFlow::Break(Pause::Stopped),
        }
    }
}

/// The observer of the source at `position`: forwards its values, and makes room as it completes.
#[doc(hidden)]
pub struct MergeObserver<M: ThreadMode, T, E, OR, C, SL, K> {
    driver: Driver<M, T, E, OR, C, SL>,
    position: K,
}

impl<M, T, E, OR, C, SL, K> Observer<T, E> for MergeObserver<M, T, E, OR, C, SL, K>
where
    M: ThreadMode,
    OR: Observer<T, E>,
    SL: ReservationSlots<K>,
{
    fn on_next(&mut self, value: T) -> Flow {
        self.driver.context.send_next(value)
    }

    fn on_termination(self, termination: Termination<E>) {
        match termination {
            Termination::Completed => {
                // Releasing the finished subscription can panic after taking the cursor, before
                // `run` has armed its own guard.
                let _guard = on_panic(|| self.driver.context.stop());
                let next = self.driver.context.update(|model| {
                    let finished = model.slots.release(self.position);
                    let outcome = match model.cursor.take_parked() {
                        // The sources are parked only while no room is left: hand the place of
                        // this source to the next one, which is pulled here.
                        Some(cursor) => UpdateOutcome::new(Some(cursor)).without_events(),
                        // While a driver holds them it finds this room itself; once none remain
                        // the stream may be over.
                        None => model.release_place(UpdateOutcome::new(None)),
                    };
                    outcome.with_drop_outside(finished)
                });
                // An error may be queued behind a delivery running elsewhere already: its stop
                // keeps the next source from being pulled, even before it is delivered.
                if let Updated::Ran {
                    result: Some(cursor),
                    flow: Flow::Continue(()),
                } = next
                {
                    // Disposing the finished subscription can re-enter and stop the stream after
                    // the update decided its flow. That disposal always ran: the sources are
                    // parked only while no source is being subscribed, so this one was stored.
                    if self.driver.context.is_active() {
                        (self.driver.resume)(cursor, &self.driver);
                    }
                }
            }
            error @ Termination::Error(_) => {
                self.driver.context.send_termination(error);
            }
        }
    }
}
