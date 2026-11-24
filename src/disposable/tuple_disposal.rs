//! Tuples of disposals, disposed from the last element to the first.
//!
//! Each disposal is wrapped in a [`DisposeOnDrop`] and the tuple reversed and dropped, so the drop
//! glue keeps this order for the remaining disposals during unwinding if one of them panics.

use crate::{
    disposable::{Disposable, dispose_on_drop::DisposeOnDrop},
    utils::positional::tuple::{IntoReversed, for_each_arity},
};

/// [`Disposable`] for a tuple of disposals, called by [`for_each_arity!`] once per arity.
macro_rules! impl_tuple_disposal {
    ($(($position:tt, $disposal:ident, $($_names:tt)*)),+) => {
        impl<$($disposal: Disposable),+> Disposable for ($($disposal,)+) {
            fn dispose(self) {
                drop(($(DisposeOnDrop::new(self.$position),)+).into_reversed());
            }
        }
    };
}

for_each_arity!(impl_tuple_disposal);

#[cfg(test)]
mod tests;
