use super::{Overlap, no_overlap};

#[test]
fn test_send_to_every_open_sink() {
    let mut overlap = Overlap::<Vec<i32>, i32>::new(i32::clone);
    let mut value = Some(1);
    overlap.send(&mut value, Vec::push);
    assert_eq!(value, Some(1));
    assert!(overlap.is_empty());

    overlap.open(Vec::new());
    let mut value = Some(2);
    overlap.send(&mut value, Vec::push);
    assert_eq!(value, None);
    overlap.open(Vec::new());
    overlap.send(&mut Some(3), Vec::push);

    assert_eq!(overlap.close_oldest(), Some(vec![2, 3]));
    overlap.open(Vec::new());
    overlap.send(&mut Some(4), Vec::push);
    assert_eq!(
        overlap.close_all().collect::<Vec<_>>(),
        [vec![3, 4], vec![4]]
    );
    assert!(overlap.is_empty());
    assert_eq!(overlap.close_oldest(), None);
}

#[test]
fn test_newest_takes_the_item() {
    // Not `Clone`: the newest, here the only one, takes it without a duplicate.
    struct Unique(i32);
    let mut overlap = Overlap::<Vec<Unique>, Unique>::new(no_overlap);
    overlap.open(Vec::new());
    overlap.send(&mut Some(Unique(1)), Vec::push);
    let bundle = overlap.close_oldest().unwrap();
    assert_eq!(bundle.iter().map(|item| item.0).collect::<Vec<_>>(), [1]);
}

#[test]
#[should_panic(expected = "a single sink is open at a time")]
fn test_no_overlap_with_two_open() {
    let mut overlap = Overlap::<Vec<i32>, i32>::new(no_overlap);
    overlap.open(Vec::new());
    overlap.open(Vec::new());
    overlap.send(&mut Some(1), Vec::push);
}
