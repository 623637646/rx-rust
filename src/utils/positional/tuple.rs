//! The table of arities the crate's tuple implementations cover, and tuple reversal.
//!
//! Nothing here knows what the elements are — sources, their disposals, what an operator keeps
//! for each source, the plans of `When` — only their positions matter here.

/// Calls `$m!` once per supported tuple arity, one to eight, so that every tuple impl of the crate
/// covers the same arities. Each element comes as its position and three names for the type
/// parameters it needs. The names `A`, `B` and `C` have no role of their own: a caller gives them
/// the ones it needs, `A` to the tuple's own elements so that its impls read `(A0, A1)`, and skips
/// the rest with a `$($_names:tt)*`.
///
/// `for_each_arity!(@reversed $m)` also passes the elements again in the reverse order, after
/// `; reversed:`. Only [`IntoReversed`] needs them: anything else that goes from the last element
/// to the first reverses its tuple with it.
///
/// The callers: `positional::position`'s `AtPosition`, the combinations in
/// `positional::combination`, the reservation storage in `positional::slots`, tuple reversal, the
/// tuples of sources of `observable::sources` and the subscription of their cursor, the tuple
/// `Disposable` their subscriptions need, and `When`'s sources, inputs, plans and patterns.
macro_rules! for_each_arity {
    ($m:ident) => {
        $crate::utils::positional::tuple::for_each_arity!(@start forward $m);
    };
    (@reversed $m:ident) => {
        $crate::utils::positional::tuple::for_each_arity!(@start reversed $m);
    };
    (@start $order:ident $m:ident) => {
        $crate::utils::positional::tuple::for_each_arity!(
            @prefixes $order $m [] [];
            (0, A0, B0, C0), (1, A1, B1, C1), (2, A2, B2, C2), (3, A3, B3, C3),
            (4, A4, B4, C4), (5, A5, B5, C5), (6, A6, B6, C6), (7, A7, B7, C7)
        );
    };
    (
        @prefixes $order:ident $m:ident [$($forward:tt,)*] [$($reverse:tt,)*];
        $next:tt $(, $rest:tt)*
    ) => {
        $crate::utils::positional::tuple::for_each_arity!(
            @emit $order $m [$($forward,)* $next] [$next $(, $reverse)*]
        );
        $crate::utils::positional::tuple::for_each_arity!(
            @prefixes $order $m [$($forward,)* $next,] [$next, $($reverse,)*]; $($rest),*
        );
    };
    (@prefixes $order:ident $m:ident $forward:tt $reverse:tt;) => {};
    (@emit forward $m:ident [$($forward:tt),+] $reverse:tt) => {
        $m!($($forward),+);
    };
    (@emit reversed $m:ident [$($forward:tt),+] [$($reverse:tt),+]) => {
        $m!($($forward),+; reversed: $($reverse),+);
    };
}

pub(crate) use for_each_arity;

/// Reverses a tuple, the last element first.
///
/// Dropping the result drops the elements from the last to the first, the rest still dropped
/// during unwinding if one panics: that is the order everything kept per position goes away in.
/// What must do more than drop wraps its elements first, as the tuple `Disposable` wraps each
/// disposal in a `DisposeOnDrop`; calling a method on each element of the result in turn would
/// skip the rest after a panic.
#[doc(hidden)]
pub trait IntoReversed {
    /// The tuple of the same elements, the last first.
    type Reversed;

    /// Moves the elements into a tuple of the reverse order.
    fn into_reversed(self) -> Self::Reversed;
}

macro_rules! impl_into_reversed {
    (
        $(($_position:tt, $element:ident, $($_names:tt)*)),+;
        reversed: $(($reversed:tt, $reversed_element:ident, $($_reversed:tt)*)),+
    ) => {
        impl<$($element),+> IntoReversed for ($($element,)+) {
            type Reversed = ($($reversed_element,)+);

            fn into_reversed(self) -> Self::Reversed {
                ($(self.$reversed,)+)
            }
        }
    };
}

for_each_arity!(@reversed impl_into_reversed);

#[cfg(test)]
mod tests {
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
}
