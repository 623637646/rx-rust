use super::IdGenerator;

#[test]
fn test_next_id() {
    let mut ids = IdGenerator::default();
    let first = ids.next_id();
    let second = ids.next_id();
    assert!(first < second);
    assert_ne!(first, second); // `first` is never handed out again.
}
