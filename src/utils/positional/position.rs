//! Typed access to one element of a collection by its position: a position known at compile time
//! in a tuple, whose elements differ in type, or at run time in a `Vec`.
//!
//! Nothing here knows what the elements are — sources, what an operator keeps for each source,
//! the inputs of `When` — so it depends on nothing else in the crate but the arity table of
//! [`tuple`](super::tuple).

use super::tuple::for_each_arity;
use educe::Educe;

/// A position in a collection, known either at compile time ([`TupleIndex`]) or at run time
/// (`usize`).
pub(crate) trait Position: Copy {
    fn position(self) -> usize;
}

/// The compile-time position of an element in a tuple: a source, or what an operator keeps for
/// one.
#[doc(hidden)]
#[derive(Educe)]
#[educe(Debug, Clone, Copy)]
pub struct TupleIndex<const INDEX: usize>;

impl<const INDEX: usize> Position for TupleIndex<INDEX> {
    fn position(self) -> usize {
        INDEX
    }
}

impl Position for usize {
    fn position(self) -> usize {
        self
    }
}

/// Reaches the element at `K` without erasing its type: a [`TupleIndex`] of a tuple, whose
/// elements differ in type, or a `usize` of a `Vec`.
///
/// Not named `ElementAt`: that is the public operator
/// [`ElementAt`](crate::operators::filtering::element_at::ElementAt).
pub(crate) trait AtPosition<K> {
    type Element;
    fn at(&self, index: K) -> &Self::Element;
    fn at_mut(&mut self, index: K) -> &mut Self::Element;
}

impl<T> AtPosition<usize> for Vec<T> {
    type Element = T;
    fn at(&self, index: usize) -> &T {
        &self[index]
    }
    fn at_mut(&mut self, index: usize) -> &mut T {
        &mut self[index]
    }
}

/// [`AtPosition`] for each element of a tuple, called by [`for_each_arity!`] once per arity.
macro_rules! impl_at_position {
    // `$all` goes through a `tt`, as it repeats at another depth than the element.
    (@elements $all:tt; $(($position:tt, $element:ident)),+) => {
        $(impl_at_position!(@element $all $position, $element);)+
    };
    // The element at `$position` of a tuple of `$all` elements.
    (@element [$($all:ident),+] $position:tt, $element:ident) => {
        impl<$($all),+> AtPosition<TupleIndex<$position>> for ($($all,)+) {
            type Element = $element;

            fn at(&self, _: TupleIndex<$position>) -> &$element {
                &self.$position
            }

            fn at_mut(&mut self, _: TupleIndex<$position>) -> &mut $element {
                &mut self.$position
            }
        }
    };
    ($(($position:tt, $element:ident, $($_names:tt)*)),+) => {
        impl_at_position!(@elements [$($element),+]; $(($position, $element)),+);
    };
}

for_each_arity!(impl_at_position);

#[cfg(test)]
mod tests {
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
}
