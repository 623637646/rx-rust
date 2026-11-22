//! What puts the values of sources running side by side back in the order of their positions:
//! the "eager" in [`ConcatEagerAll`](super::super::concat_eager_all::ConcatEagerAll),
//! [`ConcatEagerMap`](crate::operators::transforming::concat_eager_map::ConcatEagerMap) and
//! [`ConcatEager`](super::super::concat_eager::ConcatEager).
//!
//! Each source is an [`IndexedSource`], subscribed with an [`IndexingObserver`], which sends its
//! position with each of its values, as an [`IndexedEvent::Next`], and once more as it completes,
//! as an [`IndexedEvent::Completed`] ahead of its completion. A merge of them — [`flatten`] for an
//! observable of observables, whose [`IndexingSelector`] gives each its position as it makes it
//! ([`subscribe_eager`]), or [`merge`](super::merge) for a tuple or a list, each of whose sources
//! [`IndexSources`] wraps at its position in the source set — is subscribed with a
//! [`ReorderObserver`], which emits the values of the first position that has not completed and
//! keeps the others by position. Positions run from 0 with no gap. The sources are what tell their
//! position, not the merge: neither machine knows of it.
//!
//! The module is private; the types stay `pub`, re-exported by those operators, since the
//! observers their sources are subscribed with, and their disposals, name them.

use super::flatten::{self, Enqueue, Selector};
use crate::{
    disposable::dispose_on_drop::DisposeOnDrop,
    observable::{
        IterSources, JoinedModeOf, Observable, ObservableTypes, SameItemSources, sources::SourceOps,
    },
    observer::{Flow, Observer, Termination},
    thread_mode::ThreadMode,
    utils::{positional::tuple::for_each_arity, serialized_delivery::StopView},
};
use educe::Educe;
use std::{collections::BTreeMap, convert::identity, iter::Enumerate, num::NonZeroUsize};

/// What an [`IndexedSource`] sends the merge: a value or its completion, with its position among
/// the sources.
#[doc(hidden)]
pub enum IndexedEvent<T> {
    /// A value of the source at `position`.
    Next {
        /// The position of the source.
        position: usize,
        /// The value.
        value: T,
    },
    /// The source at `position` has completed.
    Completed {
        /// The position of the source.
        position: usize,
    },
}

/// A source at a position, which sends that position with each of its values and once more as it
/// completes.
#[doc(hidden)]
pub struct IndexedSource<OE> {
    source: OE,
    position: usize,
}

impl<OE: ObservableTypes> ObservableTypes for IndexedSource<OE> {
    type Item = IndexedEvent<OE::Item>;
    type Error = OE::Error;
    type Mode = OE::Mode;
    type Disposal = OE::Disposal;
}

impl<OE, OR> Observable<OR> for IndexedSource<OE>
where
    OR: Observer<IndexedEvent<<OE as ObservableTypes>::Item>, <OE as ObservableTypes>::Error>,
    OE: Observable<IndexingObserver<OR>>,
{
    fn subscribe(self, observer: OR) -> DisposeOnDrop<Self::Disposal> {
        self.source.subscribe(IndexingObserver {
            observer,
            position: self.position,
        })
    }
}

/// A [`SameItemSources`] whose sources can each be made an [`IndexedSource`] at their position: a
/// tuple, or an [`IterSources`]. Only the indexed sources' types differ from its own — the cursor
/// over them, which waits for room in a merge with a limit — so their error, mode and
/// subscriptions are its own.
///
/// [`SourceSet`](crate::observable::SourceSet) seals it.
#[doc(hidden)]
pub trait IndexSources: SameItemSources {
    /// The same sources, each an [`IndexedSource`] at its position.
    type Indexed: SameItemSources<
            SameItem = IndexedEvent<Self::SameItem>,
            Error = Self::Error,
            Mode = Self::Mode,
            SubscriptionSlots = Self::SubscriptionSlots,
        >;
}

/// The operation of [`IndexSources`], apart so that it stays out of the public API, as
/// [`SourceOps`] is apart from [`SourceSet`](crate::observable::SourceSet).
pub(crate) trait IndexSourcesOps: IndexSources<Indexed: SourceOps> {
    /// Makes each source an [`IndexedSource`] at its position, subscribing none.
    fn into_indexed_sources(self) -> Self::Indexed;
}

/// The iterator of an [`IterSources`] made of [`IndexedSource`]s: each one at its position, given
/// as it is pulled.
#[doc(hidden)]
pub struct IndexedSourcesIter<I>(Enumerate<I>);

impl<I: Iterator> Iterator for IndexedSourcesIter<I> {
    type Item = IndexedSource<I::Item>;

    fn next(&mut self) -> Option<Self::Item> {
        self.0
            .next()
            .map(|(position, source)| IndexedSource { source, position })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.size_hint()
    }
}

impl<I> IndexSources for IterSources<I>
where
    I: IntoIterator<Item: ObservableTypes>,
{
    type Indexed = IterSources<IndexedSourcesIter<I::IntoIter>>;
}

impl<I> IndexSourcesOps for IterSources<I>
where
    I: IntoIterator<Item: ObservableTypes>,
{
    fn into_indexed_sources(self) -> Self::Indexed {
        IterSources(IndexedSourcesIter(self.0.into_iter().enumerate()))
    }
}

/// [`IndexSources`] for a tuple, called by [`for_each_arity!`] once per arity.
macro_rules! impl_index_sources {
    ($(($position:tt, $source:ident, $($_names:tt)*)),+) => {
        impl<T, E, $($source),+> IndexSources for ($($source,)+)
        where
            $($source: ObservableTypes<Item = T, Error = E>,)+
        {
            type Indexed = ($(IndexedSource<$source>,)+);
        }

        impl<T, E, $($source),+> IndexSourcesOps for ($($source,)+)
        where
            $($source: ObservableTypes<Item = T, Error = E>,)+
        {
            fn into_indexed_sources(self) -> Self::Indexed {
                ($(IndexedSource { source: self.$position, position: $position },)+)
            }
        }
    };
}

for_each_arity!(impl_index_sources);

/// The observer of the source at a position: sends that position with each of its values, and
/// once more as it completes.
#[doc(hidden)]
pub struct IndexingObserver<OR> {
    observer: OR,
    position: usize,
}

impl<T, E, OR> Observer<T, E> for IndexingObserver<OR>
where
    OR: Observer<IndexedEvent<T>, E>,
{
    fn on_next(&mut self, value: T) -> Flow {
        self.observer.on_next(IndexedEvent::Next {
            position: self.position,
            value,
        })
    }

    fn on_termination(mut self, termination: Termination<E>) {
        // A completion is first a value of the merge, which lets the next source emit; an observer
        // that stops there accepts nothing more, its termination included.
        if let Termination::Completed = termination {
            let completed = IndexedEvent::Completed {
                position: self.position,
            };
            if self.observer.on_next(completed).is_break() {
                return;
            }
        }
        self.observer.on_termination(termination);
    }
}

/// The observer a merge of [`IndexedSource`]s is subscribed with: it emits the values of the first
/// source that has not completed as they arrive, and keeps those of the ones behind it until their
/// turn. The merge sends it one event at a time, so it needs no lock.
#[doc(hidden)]
pub struct ReorderObserver<M: ThreadMode, T, OR> {
    observer: OR,
    /// Whether the subscription was disposed while the values that were kept are being emitted.
    stop_view: StopView<M>,
    /// The position of the first source that has not completed, whose values are emitted as they
    /// arrive.
    current_position: usize,
    /// What the sources behind it have sent, by position: a source has an entry once it has sent
    /// something.
    buffered: BTreeMap<usize, Buffered<T>>,
}

/// What a source has sent before its turn.
#[derive(Educe)]
#[educe(Default(bound()))]
struct Buffered<T> {
    values: Vec<T>,
    completed: bool,
}

impl<M: ThreadMode, T, OR> ReorderObserver<M, T, OR> {
    /// Emits to `observer` in order, from the source at position 0, checking `stop_view` between
    /// the values that were kept.
    pub(crate) fn new(observer: OR, stop_view: StopView<M>) -> Self {
        Self {
            observer,
            stop_view,
            current_position: 0,
            buffered: BTreeMap::new(),
        }
    }

    /// Moves on from the current source, which has just completed, to the next ones, emitting what
    /// each of them sent before its turn, up to the first that has not completed.
    fn advance<E>(&mut self) -> Flow
    where
        OR: Observer<T, E>,
    {
        loop {
            self.current_position += 1;
            let Some(Buffered { values, completed }) = self.buffered.remove(&self.current_position)
            else {
                return Flow::Continue(());
            };
            // Several values for the one event the merge sent: a disposal in between, by the
            // downstream observer or from another thread, is checked for before each of them.
            self.stop_view
                .send_each(&mut self.observer, values, identity)?;
            if !completed {
                return Flow::Continue(());
            }
        }
    }
}

impl<M: ThreadMode, T, E, OR> Observer<IndexedEvent<T>, E> for ReorderObserver<M, T, OR>
where
    OR: Observer<T, E>,
{
    fn on_next(&mut self, event: IndexedEvent<T>) -> Flow {
        match event {
            IndexedEvent::Next { position, value } if position == self.current_position => {
                self.observer.on_next(value)
            }
            IndexedEvent::Next { position, value } => {
                self.buffered
                    .entry(position)
                    .or_default()
                    .values
                    .push(value);
                Flow::Continue(())
            }
            IndexedEvent::Completed { position } if position == self.current_position => {
                self.advance()
            }
            IndexedEvent::Completed { position } => {
                self.buffered.entry(position).or_default().completed = true;
                Flow::Continue(())
            }
        }
    }

    fn on_termination(self, termination: Termination<E>) {
        self.observer.on_termination(termination);
    }
}

/// The selector of a [`ConcatEagerAll`](super::super::concat_eager_all::ConcatEagerAll) or a
/// [`ConcatEagerMap`](crate::operators::transforming::concat_eager_map::ConcatEagerMap): makes
/// each inner observable with `selector` and gives it its position, from 0 in the order the source
/// sent the values.
#[doc(hidden)]
pub struct IndexingSelector<F> {
    selector: F,
    next_position: usize,
}

impl<V, F: Selector<V>> Selector<V> for IndexingSelector<F> {
    type Observable = IndexedSource<F::Observable>;

    fn select(&mut self, value: V) -> Self::Observable {
        // The position is taken once the selector has returned: one that panics leaves no gap,
        // which would keep the values of every later inner observable for good.
        let source = self.selector.select(value);
        let position = self.next_position;
        self.next_position += 1;
        IndexedSource { source, position }
    }
}

/// The observer a [`ConcatEagerAll`](super::super::concat_eager_all::ConcatEagerAll) or a
/// [`ConcatEagerMap`](crate::operators::transforming::concat_eager_map::ConcatEagerMap) over `OE`,
/// whose inner observables are `OE1`s made by the selector `F`, gives its source.
pub(crate) type SourceObserverOf<OE, OE1, T, E, OR, F> = flatten::SourceObserverOf<
    OE,
    IndexedSource<OE1>,
    IndexedEvent<T>,
    E,
    ReorderObserver<JoinedModeOf<OE, OE1>, T, OR>,
    Enqueue<IndexedSource<OE1>>,
    IndexingSelector<F>,
>;

/// The observer a [`ConcatEagerAll`](super::super::concat_eager_all::ConcatEagerAll) or a
/// [`ConcatEagerMap`](crate::operators::transforming::concat_eager_map::ConcatEagerMap) over `OE`,
/// whose inner observables are `OE1`s, gives each of them.
pub(crate) type InnerObserverOf<OE, OE1, T, E, OR> = IndexingObserver<
    flatten::InnerObserverOf<
        OE,
        IndexedSource<OE1>,
        IndexedEvent<T>,
        E,
        ReorderObserver<JoinedModeOf<OE, OE1>, T, OR>,
        Enqueue<IndexedSource<OE1>>,
    >,
>;

/// The disposal of the merge a [`ConcatEagerAll`](super::super::concat_eager_all::ConcatEagerAll)
/// over `OE`, whose inner observables are `OE1`s, is made of.
type ContextDisposalOf<OE, OE1, T, E> = flatten::ContextDisposalOf<
    OE,
    IndexedSource<OE1>,
    IndexedEvent<T>,
    E,
    Enqueue<IndexedSource<OE1>>,
>;

/// Subscribes `source` as a [`ConcatEagerAll`](super::super::concat_eager_all::ConcatEagerAll)
/// does with the inner observables `selector` makes of its values: merged, each sending its values
/// with its position, and put back in order by the observer the merge is subscribed with.
pub(crate) fn subscribe_eager<V, T, E, OE, OE1, OR, F>(
    source: OE,
    selector: F,
    observer: OR,
    max_concurrency: NonZeroUsize,
) -> DisposeOnDrop<ContextDisposalOf<OE, OE1, T, E>>
where
    OR: Observer<T, E>,
    OE: Observable<SourceObserverOf<OE, OE1, T, E, OR, F>, Item = V, Error = E>,
    OE1: Observable<InnerObserverOf<OE, OE1, T, E, OR>, Item = T, Error = E>,
    F: Selector<V, Observable = OE1>,
{
    flatten::subscribe(
        source,
        |stop_view| ReorderObserver::new(observer, stop_view),
        max_concurrency,
        // The inner observables that find no place all wait: none is skipped.
        Enqueue::new(),
        IndexingSelector {
            selector,
            next_position: 0,
        },
    )
}
