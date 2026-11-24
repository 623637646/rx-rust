use super::{CloneLatest, LatestOps, PrependOps, QueueOps};
use crate::{
    thread_mode::mutable::MutableHelper,
    utils::positional::position::{AtPosition, TupleIndex},
};
use std::{
    cell::{Cell, RefCell},
    collections::VecDeque,
};

#[test]
fn test_latest_tuple() {
    type Items = (i32, &'static str);
    let mut latest = <Items as LatestOps>::empty(2);
    assert_eq!(latest, (None, None));
    *latest.at_mut(TupleIndex::<0>) = Some(1);
    *latest.at_mut(TupleIndex::<1>) = Some("b");
    assert_eq!(Items::clone_all(&latest), Some((1, "b")));
    assert_eq!((1, "b").prepend('a'), ('a', 1, "b"));
    assert_eq!(Items::take_all(&mut latest), (1, "b"));
    assert_eq!(latest, (None, None));
}

#[test]
fn test_queue_tuple() {
    type Items = (i32, char);
    let mut queues = <Items as QueueOps>::empty(2);
    assert_eq!(queues, (VecDeque::new(), VecDeque::new()));
    queues.at_mut(TupleIndex::<0>).extend([1, 2]);
    queues.at_mut(TupleIndex::<1>).push_back('a');
    assert_eq!(Items::pop_all(&mut queues), ((1, 'a'), 1));
    queues.at_mut(TupleIndex::<1>).push_back('b');
    assert_eq!(Items::pop_all(&mut queues), ((2, 'b'), 2));
}

#[test]
fn test_latest_list() {
    type Items = Vec<i32>;
    let mut latest = <Items as LatestOps>::empty(3);
    assert_eq!(latest, [None, None, None]);
    for index in 0..3 {
        *latest.at_mut(index) = Some(index as i32);
    }
    assert_eq!(Items::clone_all(&latest), Some(vec![0, 1, 2]));
    assert_eq!(vec![0, 1, 2].prepend('a'), ('a', vec![0, 1, 2]));
    assert_eq!(Items::take_all(&mut latest), [0, 1, 2]);
    // The positions stay, without values.
    assert_eq!(latest, [None, None, None]);
}

#[test]
fn test_queue_list() {
    type Items = Vec<i32>;
    let mut queues = <Items as QueueOps>::empty(12);
    assert_eq!(queues.len(), 12);
    assert!(queues.iter().all(VecDeque::is_empty));
    for index in 0..12 {
        queues.at_mut(index).push_back(index as i32);
    }
    queues.at_mut(7).push_back(-1);
    assert_eq!(
        Items::pop_all(&mut queues),
        ((0..12).collect::<Vec<_>>(), 11)
    );
    assert_eq!(queues.at(7), &[-1]);
}

#[test]
fn test_clone_all_when_every_position_has_a_value() {
    type Items = (i32, &'static str);
    let mut latest = <Items as LatestOps>::empty(2);
    assert_eq!(Items::clone_all(&latest), None);
    *latest.at_mut(TupleIndex::<0>) = Some(1);
    assert_eq!(Items::clone_all(&latest), None);
    *latest.at_mut(TupleIndex::<1>) = Some("a");
    assert_eq!(Items::clone_all(&latest), Some((1, "a")));
    *latest.at_mut(TupleIndex::<0>) = Some(2);
    assert_eq!(Items::clone_all(&latest), Some((2, "a")));

    type List = Vec<i32>;
    let mut latest = <List as LatestOps>::empty(2);
    assert_eq!(List::clone_all(&latest), None);
    *latest.at_mut(1) = Some(1);
    assert_eq!(List::clone_all(&latest), None);
    *latest.at_mut(0) = Some(0);
    assert_eq!(List::clone_all(&latest), Some(vec![0, 1]));
}

#[test]
fn test_clone_all_clones_nothing_until_complete() {
    struct Probe<'a>(&'a Cell<usize>);

    impl Clone for Probe<'_> {
        fn clone(&self) -> Self {
            self.0.set(self.0.get() + 1);
            Self(self.0)
        }
    }

    // A clone made before a missing position is found would be dropped under the lock.
    let clones = Cell::new(0);
    let mut latest = <(Probe<'_>, i32) as LatestOps>::empty(2);
    *latest.at_mut(TupleIndex::<0>) = Some(Probe(&clones));
    assert!(<(Probe<'_>, i32)>::clone_all(&latest).is_none());
    let mut list = <Vec<Probe<'_>> as LatestOps>::empty(2);
    *list.at_mut(0) = Some(Probe(&clones));
    assert!(<Vec<Probe<'_>>>::clone_all(&list).is_none());
    assert_eq!(clones.get(), 0);

    *latest.at_mut(TupleIndex::<1>) = Some(1);
    assert!(<(Probe<'_>, i32)>::clone_all(&latest).is_some());
    assert_eq!(clones.get(), 1);
}

#[test]
fn test_empty_list() {
    // No position is ever missing.
    let latest = <Vec<i32> as LatestOps>::empty(0);
    assert_eq!(Vec::clone_all(&latest), Some(vec![]));
}

#[test]
fn test_values_are_returned_without_dropping() {
    struct Probe<'a>(&'a RefCell<Vec<i32>>, i32);

    impl Drop for Probe<'_> {
        fn drop(&mut self) {
            self.0.with_mut(|drops| drops.push(self.1));
        }
    }

    let drops = RefCell::new(Vec::new());
    let mut latest = <Vec<Probe<'_>> as LatestOps>::empty(1);
    assert!(latest.at_mut(0).replace(Probe(&drops, 1)).is_none());
    let replaced = latest.at_mut(0).replace(Probe(&drops, 2));
    drops.with_ref(|drops| assert!(drops.is_empty()));
    drop(replaced);
    drops.with_ref(|drops| assert_eq!(*drops, [1]));

    drop(latest);
    drops.with_ref(|drops| assert_eq!(*drops, [1, 2]));
}
