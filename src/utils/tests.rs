use super::take_leaving_room;

#[test]
fn test_take_leaving_room() {
    let mut values = Vec::with_capacity(4);
    values.extend([1, 2, 3]);

    let taken = take_leaving_room(&mut values, 8);
    assert_eq!(taken, [1, 2, 3]);
    assert!(values.is_empty());
    assert!(values.capacity() >= 8);
}

#[test]
fn test_take_leaving_room_empty() {
    let mut values = Vec::<i32>::with_capacity(4);

    // Nothing to take: nothing is allocated, and the buffer keeps its room.
    let taken = take_leaving_room(&mut values, 8);
    assert!(taken.is_empty());
    assert_eq!(taken.capacity(), 0);
    assert!(values.capacity() >= 4);
}
