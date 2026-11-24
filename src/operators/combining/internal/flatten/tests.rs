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
