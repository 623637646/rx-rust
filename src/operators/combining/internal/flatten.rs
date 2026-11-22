//! The shared state machine for flattening the inner observables produced by a source.
//!
//! The concurrency limit counts reserved places, including subscriptions still being built.
//! The [`Store`] decides what happens when all places are taken.
//!
//! | Operator | `max_concurrency` | Full policy |
//! |----------|-------------------|-------------|
//! | [`MergeAll`](super::super::merge_all::MergeAll) | configurable | [`Enqueue`] in FIFO order |
//! | [`ConcatAll`](super::super::concat_all::ConcatAll) | one | [`Enqueue`] in FIFO order |
//! | [`ExhaustAll`](super::super::exhaust_all::ExhaustAll) | one | [`Ignore`] the arrival |
//! | [`Switch`](super::super::switch::Switch) | one | [`Replace`] the current inner |
//!
//! An inner observable is subscribed with the lock released. Its place is freed when it has
//! completed and its subscription has returned, whichever happens second. That update reserves
//! the place for the oldest queued inner, if any. A loop handles synchronous completions without
//! growing the stack. The stream completes once the source has completed and no place is taken.
//!
//! The mapping forms run their selector on the thread delivering the source item, so the queue
//! holds the observables the selector already made: making them as their place comes would call
//! the `FnMut` from the threads of the inner observables that complete. [`Ignore`] reserves a
//! place first and only calls the selector if the reservation succeeds.
//!
//! [`Enqueue`] names the inner observable type in its disposal and observers, whose function
//! pointer subscribes the queued inners. [`Ignore`] and [`Replace`] queue [`Infallible`], so their
//! inner observers and disposals name only the inner disposal type, and an inner observable may
//! keep the observer it is given in a local it borrows (decision 0013).
//!
//! [`ConcatEagerAll`](super::super::concat_eager_all::ConcatEagerAll) and
//! [`ConcatEagerMap`](crate::operators::transforming::concat_eager_map::ConcatEagerMap) use
//! [`Enqueue`] and restore emission order by inner position through [`reorder`](super::reorder).
//! [`merge`](super::merge) handles tuples and lists, pulling each source as room becomes available.
//!
//! The observer types and their generic arguments are public so a non-boxed `Create` can expose
//! its observer to a user callback.

use crate::{
    disposable::{Disposable, dispose_on_drop::DisposeOnDrop},
    observable::{JoinedModeOf, Observable, ObservableTypes},
    observer::{Flow, Observer, Termination},
    thread_mode::ThreadMode,
    utils::{
        id_generator::Id,
        on_panic::on_panic,
        reservation_map::ReservationMap,
        serialized_delivery::{DropDecided, StopView, UpdateOutcome, Updated},
        subscribe_with_context::{
            self, SubscriptionContext, subscribe_with_context_with_stop_view,
        },
    },
};
use std::{collections::VecDeque, convert::Infallible, num::NonZeroUsize};

/// What becomes of an inner observable that arrives while every place is taken.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WhenFull {
    /// It waits in the store, for the place a completion frees.
    Wait,
    /// It is dropped, before the selector has even made it.
    Drop,
    /// It is subscribed in the place of the inner observable subscribed first, which is disposed.
    Replace,
}

/// The overflow policy and its storage for inner observables waiting for a place.
pub(crate) trait Store {
    /// The type actually stored, or [`Infallible`] for a policy that queues nothing.
    type Queued;

    /// What becomes of an arrival that finds no place.
    const WHEN_FULL: WhenFull;

    /// Whether nothing waits. Used to check that a free place never coexists with a queue.
    fn is_empty(&self) -> bool;

    /// Takes the oldest queued observable.
    fn pop(&mut self) -> Option<Self::Queued>;
}

/// The admission side of the store, which knows the type `O` of the arriving observables.
pub(crate) trait QueueArrival<O>: Store {
    /// Keeps `observable` for later. Only ever called under [`WhenFull::Wait`].
    fn push(&mut self, observable: O);

    /// Turns a queued observable back into the arrival it was. Never called for a store that
    /// queues nothing, whose [`Queued`](Store::Queued) is [`Infallible`].
    fn into_arrival(queued: Self::Queued) -> O;
}

/// The [`Store`] of [`MergeAll`](super::super::merge_all::MergeAll) and
/// [`ConcatAll`](super::super::concat_all::ConcatAll): an inner observable that finds no place
/// waits in line, without a bound, and is subscribed in the order it arrived.
#[doc(hidden)]
pub struct Enqueue<O>(VecDeque<O>);

impl<O> Enqueue<O> {
    /// An empty line.
    pub(crate) fn new() -> Self {
        Self(VecDeque::new())
    }
}

impl<O> Store for Enqueue<O> {
    type Queued = O;
    const WHEN_FULL: WhenFull = WhenFull::Wait;

    fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    fn pop(&mut self) -> Option<O> {
        self.0.pop_front()
    }
}

impl<O> QueueArrival<O> for Enqueue<O> {
    fn push(&mut self, observable: O) {
        self.0.push_back(observable);
    }

    fn into_arrival(queued: O) -> O {
        queued
    }
}

/// A store that keeps nothing: nothing ever waits in it, and it is never pushed to.
macro_rules! store_keeping_nothing {
    ($(#[$attribute:meta])* $name:ident => $when_full:expr) => {
        $(#[$attribute])*
        #[doc(hidden)]
        #[derive(Debug, Default)]
        pub struct $name;

        impl Store for $name {
            type Queued = Infallible;
            const WHEN_FULL: WhenFull = $when_full;

            fn is_empty(&self) -> bool {
                true
            }

            fn pop(&mut self) -> Option<Infallible> {
                None
            }
        }

        impl<O> QueueArrival<O> for $name {
            fn push(&mut self, _observable: O) {
                unreachable!("a store that keeps nothing is never pushed to")
            }

            fn into_arrival(queued: Infallible) -> O {
                match queued {}
            }
        }
    };
}

store_keeping_nothing!(
    /// The [`Store`] of [`ExhaustAll`](super::super::exhaust_all::ExhaustAll): an inner
    /// observable that finds no place is dropped, and nothing ever waits.
    Ignore => WhenFull::Drop
);

store_keeping_nothing!(
    /// The [`Store`] of [`Switch`](super::super::switch::Switch): an inner observable that
    /// finds no place takes that of the running one, which is disposed, and nothing ever waits.
    Replace => WhenFull::Replace
);

/// The [`Selector`] of a flattening operator over an observable of observables: the inner
/// observable is the source's item itself.
#[doc(hidden)]
#[derive(Debug, Clone, Copy, Default)]
pub struct Identity;

impl<OE1> Selector<OE1> for Identity {
    type Observable = OE1;

    fn select(&mut self, observable: OE1) -> OE1 {
        observable
    }
}

/// The disposal of a flattening subscription over the store `Q`, whose source's is `SD` and
/// whose inner observables' is `D`. Each operator wraps it in a `Disposal` of its own.
pub(crate) type ContextDisposal<M, T, E, SD, Q, D> =
    subscribe_with_context::Disposal<M, T, E, Model<Q, D>, SD>;

/// The [`ContextDisposal`] of a flattening operator over `OE`, whose inner observables are `OE1`s.
pub(crate) type ContextDisposalOf<OE, OE1, T, E, Q> = ContextDisposal<
    JoinedModeOf<OE, OE1>,
    T,
    E,
    <OE as ObservableTypes>::Disposal,
    Q,
    <OE1 as ObservableTypes>::Disposal,
>;

/// The context of a flattening subscription over the store `Q`.
type Context<M, T, E, OR, Q, D, SD> = SubscriptionContext<M, T, E, OR, Model<Q, D>, SD>;

/// The observer a flattening operator over `OE`, whose inner observables are `OE1`s, gives its
/// source.
pub(crate) type SourceObserverOf<OE, OE1, T, E, OR, Q, F> = SourceObserver<
    JoinedModeOf<OE, OE1>,
    T,
    E,
    OR,
    Q,
    F,
    <OE1 as ObservableTypes>::Disposal,
    <OE as ObservableTypes>::Disposal,
    <Q as Store>::Queued,
>;

/// The observer a flattening operator over `OE`, whose inner observables are `OE1`s, gives each
/// of them.
pub(crate) type InnerObserverOf<OE, OE1, T, E, OR, Q> = InnerObserver<
    JoinedModeOf<OE, OE1>,
    T,
    E,
    OR,
    Q,
    <OE1 as ObservableTypes>::Disposal,
    <OE as ObservableTypes>::Disposal,
    <Q as Store>::Queued,
>;

/// What the observers carry to subscribe a queued observable, an `O`, with a new inner observer.
///
/// An inner observer subscribes the next queued observable with an observer of its own type, so
/// it cannot ask for the bound that allows it on its own `Observer` implementation, as
/// [`SubscribeFn`](crate::utils::subscribe_fn::SubscribeFn) explains. The subscription is made where the
/// operator is subscribed, and carried as a function pointer, whose type may name the observer it
/// is held by. For a store that queues nothing, `O` is [`Infallible`], and it is never called:
/// the observers then do not name the type of the inner observables (decision 0013).
type SubscribeQueued<M, T, E, OR, Q, D, SD, O> =
    fn(O, InnerObserver<M, T, E, OR, Q, D, SD, O>) -> DisposeOnDrop<D>;

/// What turns each item of the source into an inner observable, on the thread of the source.
///
/// A closure is one. A trait rather than an `FnMut` bound, so that a selector that keeps state of
/// its own has a type its operator can name: the
/// [`IndexingSelector`](super::reorder::IndexingSelector) of
/// [`ConcatEagerAll`](super::super::concat_eager_all::ConcatEagerAll), which gives each inner
/// observable its position.
pub(crate) trait Selector<V> {
    /// The inner observable an item is turned into.
    type Observable;

    /// Turns `value` into its inner observable.
    fn select(&mut self, value: V) -> Self::Observable;
}

impl<V, OE1, F: FnMut(V) -> OE1> Selector<V> for F {
    type Observable = OE1;

    fn select(&mut self, value: V) -> OE1 {
        self(value)
    }
}

/// Subscribes `source` with an observer that turns each of its items into an inner observable
/// with `selector`, and flattens them, at most `max_concurrency` at a time, as the store
/// `waiting` says, into the observer that `observer_builder` builds around the [`StopView`] of the
/// context: `|_| observer` for most, and one that emits several values for one it is sent, as that
/// of [`ConcatEagerAll`](super::super::concat_eager_all::ConcatEagerAll) does with the values it
/// kept, and checks for a stop between them.
pub(crate) fn subscribe<V, T, E, OE, OE1, OR, Q, F>(
    source: OE,
    observer_builder: impl FnOnce(StopView<JoinedModeOf<OE, OE1>>) -> OR,
    max_concurrency: NonZeroUsize,
    waiting: Q,
    selector: F,
) -> DisposeOnDrop<ContextDisposalOf<OE, OE1, T, E, Q>>
where
    OR: Observer<T, E>,
    OE: Observable<SourceObserverOf<OE, OE1, T, E, OR, Q, F>, Item = V, Error = E>,
    OE1: Observable<InnerObserverOf<OE, OE1, T, E, OR, Q>, Item = T, Error = E>,
    Q: QueueArrival<OE1>,
    F: Selector<V, Observable = OE1>,
{
    let model = Model {
        running: ReservationMap::default(),
        waiting,
        max_concurrency,
        is_source_completed: false,
    };
    subscribe_with_context_with_stop_view(observer_builder, model, |context| {
        source.subscribe(SourceObserver {
            context,
            selector,
            subscribe_queued: |queued, observer| {
                <Q as QueueArrival<OE1>>::into_arrival(queued).subscribe(observer)
            },
        })
    })
}

/// The state of a flattening operator over the store `Q`, whose inner observables' disposal is
/// `D`.
pub(crate) struct Model<Q, D: Disposable> {
    /// The places taken: the inner observables subscribed to, or being subscribed to, never more
    /// than `max_concurrency`.
    ///
    /// One that completes while it is being subscribed is finished, and keeps its place until its
    /// builder fills it: a place is freed in one update only, the second of the completion and
    /// the fill, and goes in that same update to the observable that waited the longest. One that
    /// is replaced leaves the map at once, its place taken by its successor, and its builder gets
    /// the subscription back from `fill`.
    running: ReservationMap<DisposeOnDrop<D>>,
    /// The inner observables waiting for a place. Never holds one while a place is free, since a
    /// place freed goes to it at once: an observable that arrives while every place is taken goes
    /// behind those that wait, never ahead.
    waiting: Q,
    max_concurrency: NonZeroUsize,
    is_source_completed: bool,
}

/// What an update that may free a place answers: the next observable to subscribe and its key,
/// and the finished subscription, dropped outside the lock.
type NextStep<T, E, O, D> =
    UpdateOutcome<T, E, Option<(O, Id)>, DropDecided<Option<DisposeOnDrop<D>>>, true>;

impl<Q: Store, D: Disposable> Model<Q, D> {
    /// Whether an arrival can be subscribed at once: a place is free. Then nothing waits, since a
    /// place freed goes to the observable that waits at once.
    fn has_place(&self) -> bool {
        let has_place = self.running.len() < self.max_concurrency.get();
        debug_assert!(!has_place || self.waiting.is_empty());
        has_place
    }

    /// Whether the reservation is still present. A synchronous completion keeps its reservation
    /// until its subscription returns.
    fn has_reservation(&self, key: Id) -> bool {
        self.running.contains(key)
    }

    /// Whether the stream can complete. An empty reservation map also implies an empty queue.
    fn can_complete(&self) -> bool {
        debug_assert!(!self.running.is_empty() || self.waiting.is_empty());
        self.is_source_completed && self.running.is_empty()
    }

    /// Queues completion if the source has completed and all places have been released.
    fn complete_if_ready<T, E, R, DO>(
        &self,
        outcome: UpdateOutcome<T, E, R, DO>,
    ) -> UpdateOutcome<T, E, R, DO, true> {
        if self.can_complete() {
            outcome.with_termination_event(Termination::Completed)
        } else {
            outcome.without_events()
        }
    }

    /// Reserves a place for an inner observable about to be subscribed: the only way a place is
    /// taken. The caller has made room for it — a place found free, one released by a
    /// replacement, or one freed by a completion — so the limit holds.
    fn reserve_place(&mut self) -> Id {
        let key = self.running.reserve();
        debug_assert!(self.running.len() <= self.max_concurrency.get());
        key
    }

    /// Reserves a place if one is free.
    fn try_reserve_place(&mut self) -> Option<Id> {
        self.has_place().then(|| self.reserve_place())
    }

    /// Finds the place of `observable`: one that is free, that of the inner observable subscribed
    /// first, or the store, as the [`Store`] policy says. Answers the observable and its key when it
    /// is to be subscribed, and the subscription it replaced, to drop outside the lock.
    ///
    /// Not for [`WhenFull::Drop`], whose arrival asks
    /// [`try_reserve_place`](Self::try_reserve_place) instead, before the selector runs.
    fn admit<O>(&mut self, observable: O) -> (Option<(O, Id)>, Option<DisposeOnDrop<D>>)
    where
        Q: QueueArrival<O>,
    {
        if let Some(key) = self.try_reserve_place() {
            return (Some((observable, key)), None);
        }
        match Q::WHEN_FULL {
            WhenFull::Wait => {
                self.waiting.push(observable);
                (None, None)
            }
            WhenFull::Drop => unreachable!("an arrival under `WhenFull::Drop` is never admitted"),
            WhenFull::Replace => {
                // One whose subscription is still being built leaves the map as well: its builder
                // gets the subscription back from `fill`, and drops it.
                let replaced = self.running.release_first();
                (Some((observable, self.reserve_place())), replaced)
            }
        }
    }

    /// Takes the next waiting observable and reserves its place, if `freed` says that a
    /// reservation has just been both filled and finished.
    ///
    /// Only the second of a reservation's finish and fill frees its place, keeping synchronous
    /// completions iterative; until then the place stays counted, so nobody else can take it, and
    /// it goes to the observable that waited the longest in the same update.
    fn advance(&mut self, freed: bool) -> Option<(Q::Queued, Id)> {
        if !freed {
            return None;
        }
        let observable = self.waiting.pop()?;
        Some((observable, self.reserve_place()))
    }

    /// [`advance`](Self::advance) as the outcome of the update that finished or filled a
    /// reservation, which hands `finished` over when it was the second of the two, and completes
    /// the stream if that left it over.
    fn next_step<T, E>(
        &mut self,
        finished: Option<DisposeOnDrop<D>>,
    ) -> NextStep<T, E, Q::Queued, D> {
        let next = self.advance(finished.is_some());
        self.complete_if_ready(UpdateOutcome::new(next))
            .with_drop_outside(finished)
    }
}

/// The observer a flattening operator gives its source. `selector` turns an item into the inner
/// observable, outside the lock. `D` is the disposal of the inner observables, and `O` the type of
/// those that wait: the inner observables themselves, or [`Infallible`] for a store that queues
/// nothing, so that only a store that queues them names their type, in `Q` and `O`.
#[doc(hidden)]
pub struct SourceObserver<M: ThreadMode, T, E, OR, Q, F, D: Disposable, SD: Disposable, O> {
    context: Context<M, T, E, OR, Q, D, SD>,
    selector: F,
    subscribe_queued: SubscribeQueued<M, T, E, OR, Q, D, SD, O>,
}

impl<M, V, T, E, OR, Q, F, OE1, SD, O> Observer<V, E>
    for SourceObserver<M, T, E, OR, Q, F, OE1::Disposal, SD, O>
where
    M: ThreadMode,
    OR: Observer<T, E>,
    Q: QueueArrival<OE1, Queued = O>,
    F: Selector<V, Observable = OE1>,
    OE1: Observable<
            InnerObserver<M, T, E, OR, Q, <OE1 as ObservableTypes>::Disposal, SD, O>,
            Item = T,
            Error = E,
        >,
    SD: Disposable,
{
    fn on_next(&mut self, value: V) -> Flow {
        let (observable, key, replacing) = if Q::WHEN_FULL == WhenFull::Drop {
            // The place comes first: the selector does not run for a value that finds none, so no
            // work is started for it. The value never enters the lock, and is dropped here.
            let updated = self
                .context
                .update(|model| UpdateOutcome::new(model.try_reserve_place()));
            let Updated::Ran {
                result: Some(key), ..
            } = updated
            else {
                return updated.flow();
            };
            // A selector that panics gives the place back, and leaves the stream as it was, as
            // it does under the other stores.
            let selecting = on_panic(|| {
                let _ = self.context.update(|model| {
                    UpdateOutcome::empty().with_drop_outside(model.running.release(key))
                });
            });
            let observable = self.selector.select(value);
            drop(selecting);
            (observable, key, None)
        } else {
            // Every value is kept, so its observable is made before the lock is taken: what waits
            // is the observable. Nothing is reserved yet, so a selector that panics leaves the
            // stream as it was.
            let observable = self.selector.select(value);
            // The disposal of a replaced subscription runs as the admitting update returns, with
            // the place of its successor already reserved.
            let replacing =
                (Q::WHEN_FULL == WhenFull::Replace).then(|| stop_on_panic(&self.context));
            let updated = self.context.update(|model| {
                let (admitted, replaced) = model.admit(observable);
                UpdateOutcome::new(admitted).with_drop_outside(replaced)
            });
            let Updated::Ran {
                result: Some((observable, key)),
                ..
            } = updated
            else {
                return updated.flow();
            };
            (observable, key, replacing)
        };
        // Subscribes the admitted observable in the place `key` reserves for it, then the ones
        // that wait for as long as there is room, under one guard from the reservation on.
        let _guard = replacing.unwrap_or_else(|| stop_on_panic(&self.context));
        let subscription = observable.subscribe(InnerObserver {
            context: self.context.clone(),
            key,
            subscribe_queued: self.subscribe_queued,
        });
        let filled = fill(&self.context, key, subscription);
        subscribe_waiting(&self.context, filled, self.subscribe_queued)
    }

    fn on_termination(self, termination: Termination<E>) {
        match termination {
            Termination::Completed => {
                let _ = self.context.update(|model| {
                    model.is_source_completed = true;
                    model.complete_if_ready(UpdateOutcome::empty())
                });
            }
            error @ Termination::Error(_) => {
                self.context.send_termination(error);
            }
        }
    }
}

/// The observer a flattening operator gives each inner observable, whose disposal is `D`. `O` is
/// the type of the inner observables that wait, as in [`SourceObserver`].
#[doc(hidden)]
pub struct InnerObserver<M: ThreadMode, T, E, OR, Q, D: Disposable, SD: Disposable, O> {
    context: Context<M, T, E, OR, Q, D, SD>,
    key: Id,
    /// What it subscribes the next queued observable with, once it has completed.
    subscribe_queued: SubscribeQueued<M, T, E, OR, Q, D, SD, O>,
}

impl<M, T, E, OR, Q, D, SD, O> Observer<T, E> for InnerObserver<M, T, E, OR, Q, D, SD, O>
where
    M: ThreadMode,
    OR: Observer<T, E>,
    Q: Store<Queued = O>,
    D: Disposable,
    SD: Disposable,
{
    fn on_next(&mut self, value: T) -> Flow {
        if Q::WHEN_FULL != WhenFull::Replace {
            // Its place is its own until it completes, so whatever it sends is wanted.
            return self.context.send_next(value);
        }
        let key = self.key;
        self.context
            .update(|model| {
                if model.has_reservation(key) {
                    UpdateOutcome::empty()
                        .without_drop_outside()
                        .with_next_event(value)
                } else {
                    // Replaced, so nothing it sends is wanted anymore, as if downstream had
                    // stopped: the disposal on its way does the same, but a source that is not
                    // done with this value yet stops here already. The value is dropped outside
                    // the lock: dropping it can run arbitrary code.
                    UpdateOutcome::empty()
                        .with_drop_outside(value)
                        .without_events()
                        .answering_stop()
                }
            })
            .flow()
    }

    fn on_termination(self, termination: Termination<E>) {
        match termination {
            Termination::Completed => {
                // The finished subscription's disposal runs as this update returns, before the
                // next observable is subscribed: a panic there would leave its place reserved.
                // A store that keeps nothing reserves no place here, and its stream goes on.
                let _guard = (Q::WHEN_FULL == WhenFull::Wait).then(|| stop_on_panic(&self.context));
                let key = self.key;
                let finished = self.context.update(|model| {
                    // One that was replaced finds its entry gone, and frees no place. One still
                    // being subscribed keeps its place until its builder fills it.
                    let finished = model.running.finish(key);
                    model.next_step(finished)
                });
                let _ = subscribe_waiting(&self.context, finished, self.subscribe_queued);
            }
            error @ Termination::Error(_) => {
                if Q::WHEN_FULL != WhenFull::Replace {
                    self.context.send_termination(error);
                    return;
                }
                let key = self.key;
                let _ = self.context.update(|model| {
                    if model.has_reservation(key) {
                        UpdateOutcome::empty()
                            .with_termination_event(error)
                            .without_drop_outside()
                    } else {
                        // Replaced: its error is not the stream's. An ignored error can re-enter
                        // this context when dropped.
                        UpdateOutcome::empty()
                            .without_events()
                            .with_drop_outside(error)
                    }
                });
            }
        }
    }
}

/// The guard an observer holds from the reservation of a place until its fill: a panic in between
/// — in the inner observable's `subscribe`, in the downstream observer it emits to as it is
/// subscribed, or in the disposal of the subscription it takes the place of — stops the context, as
/// [`subscribe_with_context`](crate::utils::subscribe_with_context::subscribe_with_context())
/// does when its builder panics, and as
/// [`Merge`](super::super::merge::Merge) with a limit does. The place would stay taken
/// otherwise, so that the stream never completes, and nothing that waits would ever be
/// subscribed.
///
/// The selector is not in that span: it runs before the place is reserved, or, under [`Ignore`],
/// gives the place back when it panics, so the stream goes on either way.
fn stop_on_panic<M: ThreadMode, T, E, OR, Q, D: Disposable, SD: Disposable>(
    context: &Context<M, T, E, OR, Q, D, SD>,
) -> impl Drop {
    on_panic(|| context.stop())
}

/// Fills the place `key` reserves with the subscription just made, and answers the update: the
/// waiting observable the place goes to, if the inner observable completed while it was being
/// subscribed.
fn fill<M, T, E, OR, Q, D, SD>(
    context: &Context<M, T, E, OR, Q, D, SD>,
    key: Id,
    subscription: DisposeOnDrop<D>,
) -> Updated<Option<(Q::Queued, Id)>>
where
    M: ThreadMode,
    OR: Observer<T, E>,
    Q: Store,
    D: Disposable,
    SD: Disposable,
{
    context.update(|model| {
        let finished = model.running.fill(key, subscription);
        model.next_step(finished)
    })
}

/// Goes on from `updated`, the update that freed a place — a [`fill`] or a completion — with the
/// waiting observables, for as long as each completes before its subscription is filled. One
/// that stays subscribed hands the store over to its completion instead, so a chain of
/// observables that complete at once never recurses.
///
/// Answers the flow of the last update: the inner observable may have ended the stream while it
/// was being subscribed. Only a continuing update starts the next waiting observable: an error
/// queued behind a delivery running elsewhere stops the work that waits, even before it is
/// delivered. A stream that the disposal of a finished subscription ends after that update still
/// gets the next one subscribed, like one racing a termination: its fill finds the context
/// stopped, which disposes the subscription and ends the loop.
///
/// The caller holds the guard of [`stop_on_panic`] whenever an observable may wait.
fn subscribe_waiting<M, T, E, OR, Q, D, SD, O>(
    context: &Context<M, T, E, OR, Q, D, SD>,
    mut updated: Updated<Option<(O, Id)>>,
    subscribe_queued: SubscribeQueued<M, T, E, OR, Q, D, SD, O>,
) -> Flow
where
    M: ThreadMode,
    OR: Observer<T, E>,
    Q: Store<Queued = O>,
    D: Disposable,
    SD: Disposable,
{
    while let Updated::Ran {
        result: Some((observable, key)),
        flow: Flow::Continue(()),
    } = updated
    {
        let subscription = subscribe_queued(
            observable,
            InnerObserver {
                context: context.clone(),
                key,
                subscribe_queued,
            },
        );
        updated = fill(context, key, subscription);
    }
    updated.flow()
}

#[cfg(test)]
mod tests {
    use super::{Enqueue, Ignore, Model, QueueArrival, Replace, Store, WhenFull};
    use crate::{
        disposable::dispose_on_drop::DisposeOnDrop,
        utils::{id_generator::Id, reservation_map::ReservationMap},
    };
    use std::num::NonZeroUsize;

    /// A model of at most `max_concurrency` inner observables, which are plain numbers here: the
    /// model only moves them.
    fn model<Q: Store>(max_concurrency: usize, waiting: Q) -> Model<Q, ()> {
        Model {
            running: ReservationMap::default(),
            waiting,
            max_concurrency: NonZeroUsize::new(max_concurrency).unwrap(),
            is_source_completed: false,
        }
    }

    /// The observable an admission says to subscribe.
    fn admit<Q: QueueArrival<i32>>(model: &mut Model<Q, ()>, observable: i32) -> Option<i32> {
        let (admitted, _) = model.admit(observable);
        admitted.map(|(observable, _)| observable)
    }

    /// Completes the inner observable subscribed first, and answers the observable its place goes
    /// to and whether the stream is over.
    fn complete_first<Q: Store>(model: &mut Model<Q, ()>) -> (Option<Q::Queued>, bool) {
        drop(model.running.release_first());
        let next = model.advance(true);
        (next.map(|(observable, _)| observable), model.can_complete())
    }

    /// Completes the inner observable under `key` while it is still being subscribed, then fills
    /// its place as its builder returns, and answers what the fill hands on.
    fn complete_then_fill<Q: Store<Queued = i32>>(
        model: &mut Model<Q, ()>,
        key: Id,
    ) -> (Option<(i32, Id)>, bool) {
        assert!(model.running.finish(key).is_none());
        assert_eq!(model.advance(false), None);
        assert!(!model.can_complete());
        let finished = model.running.fill(key, DisposeOnDrop::new(()));
        (model.advance(finished.is_some()), model.can_complete())
    }

    #[test]
    fn test_enqueue_keeps_every_observable_in_order() {
        let mut store = Enqueue::new();
        assert_eq!(Enqueue::<i32>::WHEN_FULL, WhenFull::Wait);
        assert!(store.is_empty());
        store.push(1);
        store.push(2);
        assert!(!store.is_empty());
        assert_eq!(store.pop(), Some(1));
        assert_eq!(store.pop(), Some(2));
        assert_eq!(store.pop(), None);
    }

    #[test]
    fn test_ignore_and_replace_keep_nothing() {
        assert_eq!(Ignore::WHEN_FULL, WhenFull::Drop);
        assert!(Ignore.is_empty());
        assert_eq!(Store::pop(&mut Ignore), None);

        assert_eq!(Replace::WHEN_FULL, WhenFull::Replace);
        assert!(Replace.is_empty());
        assert_eq!(Store::pop(&mut Replace), None);
    }

    #[test]
    fn test_admit_takes_the_free_places_then_enqueues() {
        let mut model = model(2, Enqueue::new());
        assert_eq!(admit(&mut model, 1), Some(1));
        assert_eq!(admit(&mut model, 2), Some(2));
        assert_eq!(admit(&mut model, 3), None);
        assert_eq!(admit(&mut model, 4), None);
        assert_eq!(model.running.len(), 2);

        // A place freed goes to the one that waited the longest.
        assert_eq!(complete_first(&mut model), (Some(3), false));
        assert_eq!(model.running.len(), 2);
        assert_eq!(complete_first(&mut model), (Some(4), false));
        assert_eq!(complete_first(&mut model), (None, false));
    }

    #[test]
    fn test_a_completion_while_subscribing_keeps_the_place_until_the_fill() {
        let mut model = model(1, Enqueue::new());
        let (first, _) = model.admit(1);
        let (_, first) = first.unwrap();
        assert_eq!(admit(&mut model, 2), None);
        // 1 completes while it is being subscribed: its place is not free before its builder
        // returns, so an arrival in between waits behind 2, and the fill hands the place to 2.
        assert!(model.running.finish(first).is_none());
        assert_eq!(model.running.len(), 1);
        assert_eq!(admit(&mut model, 3), None);
        let finished = model.running.fill(first, DisposeOnDrop::new(()));
        let next = model.advance(finished.is_some());
        assert_eq!(next.map(|(next, _)| next), Some(2));
        assert!(!model.can_complete());
        assert_eq!(complete_first(&mut model), (Some(3), false));
    }

    #[test]
    fn test_a_completion_while_subscribing_frees_no_place_for_an_arrival() {
        let mut model = model(1, Ignore);
        let key = model.try_reserve_place().unwrap();
        assert!(model.running.finish(key).is_none());
        // Its builder has not returned yet: an arrival in between finds no place.
        assert!(model.try_reserve_place().is_none());
        let finished = model.running.fill(key, DisposeOnDrop::new(()));
        assert!(finished.is_some());
        assert!(model.try_reserve_place().is_some());
    }

    #[test]
    fn test_admit_replaces_the_running_observable() {
        let mut model = model(1, Replace);
        let (first, _) = model.admit(1);
        let (_, first) = first.unwrap();
        assert!(model.running.fill(first, DisposeOnDrop::new(())).is_none());

        let (second, replaced) = model.admit(2);
        let (observable, second) = second.unwrap();
        assert_eq!(observable, 2);
        assert!(replaced.is_some()); // The subscription of 1, to dispose outside the lock.
        assert_eq!(model.running.len(), 1);
        assert!(!model.running.contains(first)); // Whatever 1 still sends is dropped.
        assert!(model.running.contains(second));

        // One still being subscribed is replaced too: its late fill hands the subscription back.
        let (third, replaced) = model.admit(3);
        assert!(third.is_some());
        assert!(replaced.is_none());
        assert!(model.running.fill(second, DisposeOnDrop::new(())).is_some());
    }

    #[test]
    fn test_try_reserve_place_refuses_an_arrival_that_finds_none() {
        let mut model = model(1, Ignore);
        assert!(model.try_reserve_place().is_some());
        assert!(model.try_reserve_place().is_none());
        assert_eq!(complete_first(&mut model), (None, false));
        assert!(model.try_reserve_place().is_some());
    }

    #[test]
    fn test_advance_ends_the_stream_once_the_source_and_every_inner_are_done() {
        let mut model = model(1, Enqueue::new());
        assert_eq!(admit(&mut model, 1), Some(1));
        assert_eq!(admit(&mut model, 2), None);
        model.is_source_completed = true;

        drop(model.running.release_first());
        let (next, key) = model.advance(true).unwrap(); // Reserve a place for the queued inner.
        assert_eq!(next, 2);
        assert!(!model.can_complete());
        // The last one completes while it is being subscribed: the stream ends as its builder
        // fills its place, not before.
        assert_eq!(complete_then_fill(&mut model, key), (None, true));
    }
}
