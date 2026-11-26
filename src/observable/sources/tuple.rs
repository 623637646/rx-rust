//! The tuple backend: tuples of one to eight sources, each element keeping its own item
//! and disposal types, addressed by a
//! [`TupleIndex`](crate::utils::positional::position::TupleIndex).

use super::{CollectedCursor, InternalSourceSet, SameItemSources, SourceSet, sealed};
use crate::{
    disposable::{Disposable, dispose_on_drop::DisposeOnDrop},
    observable::ObservableTypes,
    thread_mode::Joined,
    utils::positional::{
        slots::{ReservationSlot, TupleSlots},
        tuple::for_each_arity,
    },
};

/// The modes of the sources, joined: the [`SourceSet::Mode`] of a tuple of them.
macro_rules! joined_mode {
    ($source:ident) => { <$source as ObservableTypes>::Mode };
    ($source:ident, $($rest:ident),+) => {
        Joined<<$source as ObservableTypes>::Mode, joined_mode!($($rest),+)>
    };
}

/// Everything of this module for one arity.
macro_rules! impl_source_tuple {
    (
        $(($position:tt, $source:ident, $item:ident, $disposal:ident)),+
    ) => {
        impl<$($source),+> sealed::Sealed for ($($source,)+) {}

        impl<E, $($source, $item),+> SourceSet for ($($source,)+)
        where
            $($source: ObservableTypes<Item = $item, Error = E>,)+
        {
            type Item = ($($item,)+);
            type Error = E;
            type Mode = joined_mode!($($source),+);
            type Cursor = ($(Option<$source>,)+);
            type SubscriptionSlots =
                TupleSlots<($(ReservationSlot<DisposeOnDrop<<$source as ObservableTypes>::Disposal>>,)+)>;
        }

        impl<E, $($source),+> InternalSourceSet for ($($source,)+)
        where
            $($source: ObservableTypes<Error = E>,)+
        {
            type CollectedCursor = CollectedCursor<Self::Cursor>;

            fn into_collected_cursor(self) -> (Self::CollectedCursor, usize) {
                (CollectedCursor(self.into_cursor()), const { [$($position),+].len() })
            }

            fn into_cursor(self) -> Self::Cursor {
                ($(Some(self.$position),)+)
            }
        }

        impl<T, E, $($source),+> SameItemSources for ($($source,)+)
        where
            $($source: ObservableTypes<Item = T, Error = E>,)+
        {
            type SameItem = T;
        }

        /// The slots of a tuple hold guarded subscriptions: dropping them is their disposal.
        impl<$($disposal: Disposable),+> Disposable
            for TupleSlots<($(ReservationSlot<DisposeOnDrop<$disposal>>,)+)>
        {
            fn dispose(self) {}
        }
    };
}

for_each_arity!(impl_source_tuple);
