use super::IntoReversed;

#[test]
fn test_into_reversed() {
    assert_eq!((1,).into_reversed(), (1,));
    assert_eq!((1, "b", 'c').into_reversed(), ('c', "b", 1));
    assert_eq!(
        (0, 1, 2, 3, 4, 5, 6, 7).into_reversed(),
        (7, 6, 5, 4, 3, 2, 1, 0)
    );
}
