//! The list backend: [`IterSources`] over any homogeneous iterator, stored in `Vec`s and
//! addressed by a `usize`.

use super::{CollectedCursor, InternalSourceSet, SameItemSources, SourceSet, sealed};
use crate::{
    disposable::{Disposable, dispose_on_drop::DisposeOnDrop},
    observable::ObservableTypes,
    utils::positional::slots::ListSlots,
};
use educe::Educe;
use std::iter::Enumerate;

/// Adapts any homogeneous [`IntoIterator`] to a [`SourceSet`]: what the `new_from_iter`
/// constructors take.
///
/// No iteration or allocation occurs until subscription. Operators that need all sources collect
/// them into a `Vec` before subscribing any; merge and amb enumerate them directly and stop
/// early. The subscriptions are stored by position, without reserving an iterator's
/// potentially infinite size hint.
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct IterSources<I>(pub(crate) I);

impl<I> sealed::Sealed for IterSources<I> {}

impl<I, OE> SourceSet for IterSources<I>
where
    I: IntoIterator<Item = OE>,
    OE: ObservableTypes,
{
    type Item = Vec<OE::Item>;
    type Error = OE::Error;
    type Mode = OE::Mode;
    type Cursor = Enumerate<I::IntoIter>;
    type SubscriptionSlots = ListSlots<DisposeOnDrop<OE::Disposal>>;
}

impl<I, OE> InternalSourceSet for IterSources<I>
where
    I: IntoIterator<Item = OE>,
    OE: ObservableTypes,
{
    type CollectedCursor = CollectedCursor<Enumerate<std::vec::IntoIter<OE>>>;

    fn into_collected_cursor(self) -> (Self::CollectedCursor, usize) {
        let sources: Vec<OE> = self.0.into_iter().collect();
        let position_count = sources.len();
        (
            CollectedCursor(sources.into_iter().enumerate()),
            position_count,
        )
    }

    fn into_cursor(self) -> Self::Cursor {
        self.0.into_iter().enumerate()
    }
}

impl<I, OE> SameItemSources for IterSources<I>
where
    I: IntoIterator<Item = OE>,
    OE: ObservableTypes,
{
    type SameItem = OE::Item;
}

/// The slots of a list hold guarded subscriptions: dropping them is their disposal, so that an
/// unwind while they are being filled disposes them too.
impl<D: Disposable> Disposable for ListSlots<DisposeOnDrop<D>> {
    fn dispose(self) {}
}
