use super::Bundles;
use crate::{
    observer::{Flow, Observer, Termination},
    operators::transforming::internal::group::{Events, Groups},
};
use std::convert::Infallible;

#[test]
fn test_bundles_with_room() {
    let mut bundles = Bundles::with_room(i32::clone, 3);
    let mut events = Events::new();
    let _ = Groups::<Infallible>::open(&mut bundles, &mut events);
    let _ = Groups::<Infallible>::put(&mut bundles, &mut Some(1), &mut events);
    let _ = Groups::<Infallible>::open(&mut bundles, &mut events);
    let _ = Groups::<Infallible>::put(&mut bundles, &mut Some(2), &mut events);
    assert!(matches!(events, Events::Zero));

    let _ = Groups::<Infallible>::close_oldest(&mut bundles, &mut events);
    let Events::One(first) = events else {
        panic!("one bundle emitted");
    };
    assert_eq!(first, [1, 2]);
    // Opened before any bundle was emitted: the room given.
    assert_eq!(first.capacity(), 3);

    // The completion emits the open bundles that are not empty.
    let mut events = Events::new();
    let _ = Groups::<Infallible>::open(&mut bundles, &mut events);
    let _ = bundles.end(&Termination::<Infallible>::Completed, &mut events);
    assert_eq!(events.into_vec(), [vec![2]]);
}

#[test]
fn test_bundles_room_of_last() {
    let mut bundles = Bundles::with_room(i32::clone, 0);
    let mut events = Events::new();
    let _ = Groups::<Infallible>::open(&mut bundles, &mut events);
    for value in 1..=5 {
        let _ = Groups::<Infallible>::put(&mut bundles, &mut Some(value), &mut events);
    }
    let _ = Groups::<Infallible>::close_oldest(&mut bundles, &mut events);
    let _ = Groups::<Infallible>::open(&mut bundles, &mut events);
    let _ = Groups::<Infallible>::put(&mut bundles, &mut Some(6), &mut events);
    let _ = Groups::<Infallible>::close_oldest(&mut bundles, &mut events);
    let emitted = events.into_vec();
    assert_eq!(emitted, [vec![1, 2, 3, 4, 5], vec![6]]);
    assert_eq!(emitted[1].capacity(), 5);
}

#[test]
fn test_bundles_value_between_bundles_left() {
    let mut bundles = Bundles::with_room(i32::clone, 2);
    let mut value = Some(1);
    let mut events = Events::new();
    let _ = Groups::<Infallible>::put(&mut bundles, &mut value, &mut events);
    assert_eq!(value, Some(1));
    assert!(matches!(events, Events::Zero));
}

#[test]
fn test_bundles_end_emits_nothing_after_a_stop() {
    /// Takes one bundle, then stops.
    struct TakeOne(Vec<Vec<i32>>);

    impl Observer<Vec<i32>, Infallible> for TakeOne {
        fn on_next(&mut self, value: Vec<i32>) -> Flow {
            self.0.push(value);
            Flow::Break(())
        }

        fn on_termination(self, _: Termination<Infallible>) {}
    }

    let mut bundles = Bundles::with_room(i32::clone, 2);
    let mut observer = TakeOne(Vec::new());
    let _ = Groups::<Infallible>::open(&mut bundles, &mut observer);
    let _ = Groups::<Infallible>::put(&mut bundles, &mut Some(1), &mut observer);
    let _ = Groups::<Infallible>::open(&mut bundles, &mut observer);
    let _ = Groups::<Infallible>::put(&mut bundles, &mut Some(2), &mut observer);
    assert!(
        bundles
            .end(&Termination::Completed, &mut observer)
            .is_break()
    );
    assert_eq!(observer.0, [vec![1, 2]]);
}
