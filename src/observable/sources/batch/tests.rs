use super::{ObserverFamily, SubscribeBatch};
use crate::{
    disposable::dispose_on_drop::DisposeOnDrop,
    observable::sources::{InternalSourceSet, IterSources, SourceSet, testing::IgnoreObserver},
    operators::creating::just::Just,
    utils::positional::position::Position,
};
use std::cell::Cell;

/// Counts the observers it made, and is active until it has made two.
struct CountFamily(Cell<usize>);

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

/// Collects the sources, then subscribes them; returns their position count.
fn subscribe_collected<S>(sources: S, family: &CountFamily) -> usize
where
    S: InternalSourceSet,
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
