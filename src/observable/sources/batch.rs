//! Subscribing every source at once, as a batch: the shared [`SourceSubscriber`] of the
//! operators that collect their sources first.
//!
//! Such an operator says only which observer each position gets, with an [`ObserverFamily`]; the
//! batch keeps the subscriptions in slots of its own while it runs, and hands them back when it is
//! done. An operator that reaches a subscription while its sources are still being subscribed —
//! to release it as its source completes, or to dispose the others as one wins — brings a
//! subscriber of its own instead.

use super::subscribe::{SourceSubscriber, SubscribeEach};
use crate::{
    disposable::{Disposable, dispose_on_drop::DisposeOnDrop},
    observable::ObservableTypes,
    utils::positional::{position::Position, slots::ReservationSlots},
};
use educe::Educe;
use std::ops::ControlFlow;

/// The observers an operator subscribes its sources with, indexed by a [`Position`].
pub(crate) trait ObserverFamily {
    type At<K: Position>;

    fn at<K: Position>(&self, position: K) -> Self::At<K>;

    /// Whether the stream is still running: a batch subscribes no source once it is not.
    fn is_active(&self) -> bool;
}

/// A cursor over sources that are all collected: a tuple's, or the enumeration of a `Vec`.
///
/// Advancing it runs no user code, which is what a batch relies on. Only the backends of
/// [`sources`](super) make one, in `SourceOps::into_collected_cursor`: a cursor still pulling an
/// iterator ([`SourceSet::Cursor`](super::SourceSet::Cursor)) cannot be passed for it.
#[doc(hidden)]
#[derive(Educe)]
#[educe(Debug)]
pub struct CollectedCursor<C>(pub(super) C);

/// Subscribes every source a collected cursor has left at once, as a batch, into the slots `SL`.
///
/// The subscriptions come back guarded: the guard disposes the slots on drop, including during
/// unwinding, the last subscribed first. No source is subscribed once the family is not active,
/// and those left are dropped with the cursor.
///
/// It has one impl, for a [`CollectedCursor`] whose cursor takes the batch as its subscriber:
/// the bound an operator writes, on its
/// [`SourceSet::CollectedCursor`](super::SourceSet::CollectedCursor), instead of naming the
/// batch. A cursor still pulling an iterator does not implement it: its subscriber checks again
/// after `Iterator::next`, as Merge's and amb's do.
pub(crate) trait SubscribeBatch<F: ObserverFamily, SL: Disposable> {
    fn subscribe_batch(self, family: &F) -> DisposeOnDrop<SL>;
}

impl<C, F, SL> SubscribeBatch<F, SL> for CollectedCursor<C>
where
    C: for<'a> SubscribeEach<Batch<'a, F, SL>, ()>,
    F: ObserverFamily,
    SL: Default + Disposable,
{
    fn subscribe_batch(mut self, family: &F) -> DisposeOnDrop<SL> {
        // Dropped by a panic in a source, the slots dispose the sources subscribed so far.
        let mut batch = Batch {
            family,
            slots: SL::default(),
        };
        if family.is_active() {
            let _ = self.0.subscribe_each(&mut batch);
        }
        DisposeOnDrop::new(batch.slots)
    }
}

/// The subscriber of a batch: subscribes each source with the observer of its position and keeps
/// the subscription in the slot of that position, while the family is active. It breaks, with
/// nothing to say, once the family is not: the operator takes no more sources.
struct Batch<'a, F, SL> {
    family: &'a F,
    slots: SL,
}

impl<F, SL, K, OE> SourceSubscriber<K, OE, ()> for Batch<'_, F, SL>
where
    F: ObserverFamily,
    K: Position,
    OE: ObservableTypes,
    SL: ReservationSlots<K, Value = DisposeOnDrop<OE::Disposal>>,
{
    type Observer = F::At<K>;

    fn still_running(&self) -> ControlFlow<()> {
        // Advancing this collected cursor runs no user code. Check at the start of the batch
        // and after each subscription.
        ControlFlow::Continue(())
    }

    fn observer(&mut self, position: K) -> Option<F::At<K>> {
        Some(self.family.at(position))
    }

    fn store(&mut self, position: K, subscription: DisposeOnDrop<OE::Disposal>) -> ControlFlow<()> {
        let rejected = self.slots.fill(position, subscription);
        debug_assert!(rejected.is_none(), "a batch releases no slot");
        // The source may have ended the stream as it was subscribed: subscribe no other one.
        if self.family.is_active() {
            ControlFlow::Continue(())
        } else {
            ControlFlow::Break(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{ObserverFamily, SubscribeBatch};
    use crate::{
        disposable::dispose_on_drop::DisposeOnDrop,
        observable::sources::{IterSources, SourceOps, SourceSet},
        observer::{Flow, Observer, Termination},
        operators::creating::just::Just,
        utils::positional::position::Position,
    };
    use std::cell::Cell;

    /// Counts the observers it made, and is active until it has made two.
    struct CountFamily(Cell<usize>);
    struct IgnoreObserver;

    impl ObserverFamily for CountFamily {
        type At<K: Position> = IgnoreObserver;

        fn at<K: Position>(&self, _: K) -> Self::At<K> {
            self.0.set(self.0.get() + 1);
            IgnoreObserver
        }

        fn is_active(&self) -> bool {
            self.0.get() < 2
        }
    }

    impl<T, E> Observer<T, E> for IgnoreObserver {
        fn on_next(&mut self, _: T) -> Flow {
            Flow::Continue(())
        }

        fn on_termination(self, _: Termination<E>) {}
    }

    /// Collects the sources, then subscribes them; returns their position count.
    fn subscribe_collected<S>(sources: S, family: &CountFamily) -> usize
    where
        S: SourceOps,
        S::CollectedCursor: SubscribeBatch<CountFamily, S::SubscriptionSlots>,
    {
        let (cursor, position_count) = sources.into_collected_cursor();
        let _: DisposeOnDrop<<S as SourceSet>::SubscriptionSlots> = cursor.subscribe_batch(family);
        position_count
    }

    #[test]
    fn test_collected_tuple_subscribes_only_while_active() {
        let family = CountFamily(Cell::new(0));
        let sources = (Just::new(1), Just::new(2), Just::new(3));
        assert_eq!(subscribe_collected(sources, &family), 3);
        assert_eq!(family.0.get(), 2);
    }

    #[test]
    fn test_collected_list_collects_before_subscribing() {
        let family = CountFamily(Cell::new(0));
        let pulled = Cell::new(0);
        let sources = (0..3).map(|value| {
            assert_eq!(family.0.get(), 0);
            pulled.set(pulled.get() + 1);
            Just::new(value)
        });
        assert_eq!(subscribe_collected(IterSources(sources), &family), 3);
        assert_eq!(pulled.get(), 3);
        assert_eq!(family.0.get(), 2);
    }

    #[test]
    fn test_batch_subscribes_nothing_when_inactive() {
        let family = CountFamily(Cell::new(2));
        let sources = (0..3).map(Just::new);
        assert_eq!(subscribe_collected(IterSources(sources), &family), 3);
        assert_eq!(family.0.get(), 2);
        assert_eq!(
            subscribe_collected((Just::new(1), Just::new(2)), &family),
            2
        );
        assert_eq!(family.0.get(), 2);
    }
}
