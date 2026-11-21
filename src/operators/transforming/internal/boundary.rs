//! One group at a time, rotated whenever a `boundary` observable emits: the state machine of
//! [`Buffer`](super::super::buffer::Buffer) and [`Window`](super::super::window::Window)
//! (decision 0014).
//!
//! One context serializes the source against the boundary: its model is the [`Groups`], and its
//! events are those of the groups. The two operators differ in what a completed boundary does,
//! which each fixes in the type of its [`BoundaryObserver`], as `ENDS_STREAM`.

use super::group::{Events, Groups, update_groups};
use crate::{
    disposable::{Disposable, dispose_on_drop::DisposeOnDrop},
    observable::{ChainDisposalOf, JoinedModeOf, Observable, ObservableTypes},
    observer::{Flow, Observer, Termination},
    thread_mode::ThreadMode,
    utils::subscribe_with_context::{
        SubscriptionContext, TwoStepsDisposal, subscribe_with_context_two_steps,
    },
};

/// The context of a boundary subscription, whose groups `G` emit `Ev`, delivered to `OR`.
type Context<M, G, Ev, E, OR, D> = SubscriptionContext<M, Ev, E, OR, G, D>;

/// The observer a boundary operator over `OE` and `OE1` gives its source.
pub(crate) type SourceObserverOf<OE, OE1, G, Ev, E, OR> =
    SourceObserver<JoinedModeOf<OE, OE1>, G, Ev, E, OR, ChainDisposalOf<OE, OE1>>;

/// The observer a boundary operator over `OE` and `OE1` gives its boundary.
pub(crate) type BoundaryObserverOf<OE, OE1, G, Ev, E, OR, const ENDS_STREAM: bool> =
    BoundaryObserver<JoinedModeOf<OE, OE1>, G, Ev, E, OR, ChainDisposalOf<OE, OE1>, ENDS_STREAM>;

/// The disposal of a boundary subscription over `OE` and `OE1`, before the operator names it.
pub(crate) type BoundaryDisposal<OE, OE1, G, Ev, E> = TwoStepsDisposal<
    JoinedModeOf<OE, OE1>,
    Ev,
    E,
    G,
    <OE1 as ObservableTypes>::Disposal,
    <OE as ObservableTypes>::Disposal,
>;

/// Subscribes `boundary`, then `source`, with the groups `groups`, whose events go to `observer`:
/// the first group opens now. A completed boundary ends the stream if `ENDS_STREAM`, which the
/// operator fixes in the type of the observer `OE1` takes.
pub(crate) fn subscribe<T, E, OE, OE1, G, OR, const ENDS_STREAM: bool>(
    source: OE,
    boundary: OE1,
    mut groups: G,
    mut observer: OR,
) -> DisposeOnDrop<BoundaryDisposal<OE, OE1, G, G::Event, E>>
where
    G: Groups<E, Item = T>,
    OR: Observer<G::Event, E>,
    OE: Observable<SourceObserverOf<OE, OE1, G, G::Event, E, OR>, Item = T, Error = E>,
    OE1: Observable<BoundaryObserverOf<OE, OE1, G, G::Event, E, OR, ENDS_STREAM>, Error = E>,
{
    // The first group opens before anything is subscribed, so that synchronous values have
    // somewhere to go, and before the context exists, which nothing else can reach yet: without a
    // lock. An observer that stops on the first window keeps both sources from being subscribed.
    let flow = groups.open(&mut observer);
    subscribe_with_context_two_steps(
        observer,
        groups,
        |context| {
            if flow.is_stop() {
                return None;
            }
            Some(boundary.subscribe(BoundaryObserver(context.clone())))
        },
        |context| Some(source.subscribe(SourceObserver(context.clone()))),
    )
}

#[doc(hidden)]
pub struct SourceObserver<M: ThreadMode, G, Ev, E, OR, D: Disposable>(Context<M, G, Ev, E, OR, D>);

impl<M, T, E, G, Ev, OR, D> Observer<T, E> for SourceObserver<M, G, Ev, E, OR, D>
where
    M: ThreadMode,
    G: Groups<E, Item = T, Event = Ev>,
    OR: Observer<Ev, E>,
    D: Disposable,
{
    fn on_next(&mut self, value: T) -> Flow {
        // Kept outside the update, so that an item no group takes is dropped outside the lock.
        let mut value = Some(value);
        update_groups(&self.0, |groups, events| {
            let _ = groups.put(&mut value, events);
        })
        .flow()
    }

    fn on_termination(self, termination: Termination<E>) {
        let _ = self.0.update(|groups| Events::ending(groups, termination));
    }
}

/// `ENDS_STREAM` says what a completed boundary does: if set, it ends the groups and the stream,
/// as the completion of the source would — a buffer, which has no consumer to release items to,
/// would otherwise gather them without bound; if not, it only stops the rotation, and the open
/// group and the stream go on, as a window does.
#[doc(hidden)]
pub struct BoundaryObserver<M: ThreadMode, G, Ev, E, OR, D: Disposable, const ENDS_STREAM: bool>(
    Context<M, G, Ev, E, OR, D>,
);

impl<M, T1, E, G, Ev, OR, D, const ENDS_STREAM: bool> Observer<T1, E>
    for BoundaryObserver<M, G, Ev, E, OR, D, ENDS_STREAM>
where
    M: ThreadMode,
    G: Groups<E, Event = Ev>,
    OR: Observer<Ev, E>,
    D: Disposable,
{
    fn on_next(&mut self, value: T1) -> Flow {
        let flow = update_groups(&self.0, |groups, events| {
            let _ = groups.rotate(events);
        })
        .flow();
        // Only the arrival of a boundary matters, never its value, which is dropped here, after
        // the rotation and outside its lock, since dropping it can run arbitrary code.
        drop(value);
        flow
    }

    fn on_termination(self, termination: Termination<E>) {
        if ENDS_STREAM || !matches!(termination, Termination::Completed) {
            let _ = self.0.update(|groups| Events::ending(groups, termination));
        }
    }
}
