use super::{AtPosition, Position, TupleIndex};

#[test]
fn test_position() {
    assert_eq!(TupleIndex::<3>.position(), 3);
    assert_eq!(5usize.position(), 5);
}

#[test]
fn test_at_position_tuple() {
    let mut tuple = (1, "b", 'c');
    assert_eq!(*tuple.at(TupleIndex::<1>), "b");
    *tuple.at_mut(TupleIndex::<2>) = 'd';
    assert_eq!(tuple, (1, "b", 'd'));
}

#[test]
fn test_at_position_vec() {
    let mut list = vec![1, 2, 3];
    assert_eq!(*list.at(1), 2);
    *list.at_mut(2) = 4;
    assert_eq!(list, [1, 2, 4]);
}
