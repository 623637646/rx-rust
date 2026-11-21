//! The [`Concat`] operator, behind
//! [`ObservableExt::concat`](crate::observable::ObservableExt::concat).

use crate::{
    delegate_disposal,
    disposable::{
        Disposable, chain_disposal::ChainDisposal, dispose_on_drop::DisposeOnDrop,
        replaceable_disposal::ReplaceableDisposal,
    },
    observable::{
        IterSources, JoinedModeOf, Observable, ObservableTypes, SameItemSources, SourceSet,
    },
    observer::{Flow, Observer, Termination},
    operators::combining::merge::Merge,
    thread_mode::ThreadMode,
    utils::positional::tuple::for_each_arity,
};
use educe::Educe;
use std::num::NonZeroUsize;

/// Emits all of the values of the first observable, then, once it completes, all of the values of
/// the next, and so on to the last.
/// See <https://reactivex.io/documentation/operators/concat.html>
///
/// The sources are a tuple of one to eight observables of one item type and one error type, which
/// may be observables of different types: no boxing is needed to concatenate them.
/// [`ObservableExt::concat`](crate::observable::ObservableExt::concat) is the form for two;
/// [`Concat::new_from_iter`] is the one for any number of observables of one type, decided as
/// the program runs. The stream completes once the last source has completed; an error of the
/// one that is running ends it, and the ones after it are never subscribed.
///
/// For a tuple, the downstream observer passes from one source to the next and values are
/// forwarded without locking. A list is pulled from its iterator,
/// the next source only once the one before it has completed, so that an iterator that never ends
/// can be concatenated: it is [`Merge`] with a `max_concurrency` of one. An observable of
/// observables, which pushes them rather than being pulled, goes to
/// [`ConcatAll`](super::concat_all::ConcatAll).
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt,
///     observer::Termination,
///     operators::{
///         combining::concat::Concat,
///         creating::from_iter::FromIter,
///     },
/// };
///
/// let mut values = Vec::new();
///
/// let observable = Concat::new((
///     FromIter::new([1, 2]),
///     FromIter::new([3, 4]),
///     FromIter::new([5]).map(|value| value * 10),
/// ));
/// observable.subscribe_with_callback(
///     |value| values.push(value),
///     |termination| assert_eq!(termination, Termination::Completed),
/// );
///
/// assert_eq!(values, vec![1, 2, 3, 4, 50]);
/// ```
///
/// A list, pulled as each source completes:
///
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt,
///     observer::Termination,
///     operators::{
///         combining::concat::Concat,
///         creating::from_iter::FromIter,
///     },
/// };
///
/// let mut values = Vec::new();
///
/// let observable =
///     Concat::new_from_iter((0..3).map(|index| FromIter::new([index * 10, index * 10 + 1])));
/// observable.subscribe_with_callback(
///     |value| values.push(value),
///     |termination| assert_eq!(termination, Termination::Completed),
/// );
///
/// assert_eq!(values, vec![0, 1, 10, 11, 20, 21]);
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct Concat<S> {
    sources: S,
}

impl<S> Concat<S> {
    /// Creates a [`Concat`] over the tuple `sources`;
    /// [`ObservableExt::concat`](crate::observable::ObservableExt::concat) is the fluent
    /// form for two.
    pub fn new(sources: S) -> Self
    where
        S: ConcatSources,
    {
        Self { sources }
    }
}

impl<I> Concat<IterSources<I>> {
    /// Concatenates the observables of an iterator, pulling the next one only once the one
    /// before it has completed.
    ///
    /// Stops pulling sources when the stream stops; an empty iterator completes immediately.
    pub fn new_from_iter<OE>(sources: I) -> Self
    where
        I: IntoIterator<Item = OE>,
        OE: ObservableTypes,
    {
        Self {
            sources: IterSources(sources),
        }
    }
}

/// The sources of a [`Concat`]: a tuple of one to eight observables of one item type and one
/// error type, or the list of [`Concat::new_from_iter`]. Like [`SourceSet`], it is sealed.
pub trait ConcatSources: SameItemSources {
    /// The observable that emits the sources one after the other: the pairs of a tuple, nested,
    /// or the [`Merge`] of a list, one source at a time.
    #[doc(hidden)]
    type Chained: ObservableTypes<
            Item = Self::SameItem,
            Error = <Self as SourceSet>::Error,
            Mode = <Self as SourceSet>::Mode,
        >;

    /// Makes that observable of the sources.
    #[doc(hidden)]
    fn chain(self) -> Self::Chained;
}

/// The [`ConcatSources::Chained`] of a tuple: the first source, then the others chained.
macro_rules! chained_type {
    ($last:ident) => { $last };
    ($first:ident, $($rest:ident),+) => { ConcatPair<$first, chained_type!($($rest),+)> };
}

/// The value of [`chained_type!`], from the elements of `$sources` at the given positions.
macro_rules! chained_value {
    ($sources:ident; $last:tt) => { $sources.$last };
    ($sources:ident; $first:tt, $($rest:tt),+) => {
        ConcatPair {
            source_1: $sources.$first,
            source_2: chained_value!($sources; $($rest),+),
        }
    };
}

/// [`ConcatSources`] for a tuple, called by [`for_each_arity!`] once per arity.
macro_rules! impl_concat_sources {
    ($(($position:tt, $source:ident, $($_names:tt)*)),+) => {
        impl<T, E, $($source),+> ConcatSources for ($($source,)+)
        where
            $($source: ObservableTypes<Item = T, Error = E>,)+
        {
            type Chained = chained_type!($($source),+);

            fn chain(self) -> Self::Chained {
                chained_value!(self; $($position),+)
            }
        }
    };
}

for_each_arity!(impl_concat_sources);

impl<I, OE> ConcatSources for IterSources<I>
where
    I: IntoIterator<Item = OE>,
    OE: ObservableTypes,
{
    type Chained = Merge<Self, NonZeroUsize>;

    fn chain(self) -> Self::Chained {
        // One source at a time.
        Merge::new(self).max_concurrency(NonZeroUsize::MIN)
    }
}

impl<S: ConcatSources> ObservableTypes for Concat<S> {
    type Item = S::SameItem;
    type Error = <S as SourceSet>::Error;
    type Mode = <S as SourceSet>::Mode;
    /// The disposal of the pairs a tuple is chained into, or of the [`Merge`] of a list.
    type Disposal = <S::Chained as ObservableTypes>::Disposal;
}

impl<S, OR> Observable<OR> for Concat<S>
where
    S: ConcatSources,
    OR: Observer<<S as SameItemSources>::SameItem, <S as SourceSet>::Error>,
    <S as ConcatSources>::Chained: Observable<OR>,
{
    fn subscribe(self, observer: OR) -> DisposeOnDrop<Self::Disposal> {
        self.sources.chain().subscribe(observer)
    }
}

/// Two sources, one after the other: what a tuple of them is chained into, the second being the
/// chain of those after the first.
#[doc(hidden)]
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct ConcatPair<OE1, OE2> {
    source_1: OE1,
    source_2: OE2,
}

delegate_disposal!(
    Disposal<M, D1, D2>,
    ChainDisposal<ReplaceableDisposal<M, D2>, D1>,
    where M: ThreadMode, D1: Disposable, D2: Disposable
);

impl<T, E, OE1, OE2> ObservableTypes for ConcatPair<OE1, OE2>
where
    OE1: ObservableTypes<Item = T, Error = E>,
    OE2: ObservableTypes<Item = T, Error = E>,
{
    type Item = T;
    type Error = E;
    type Mode = JoinedModeOf<OE1, OE2>;
    type Disposal = Disposal<JoinedModeOf<OE1, OE2>, OE1::Disposal, OE2::Disposal>;
}

impl<T, E, OE1, OE2, OR> Observable<OR> for ConcatPair<OE1, OE2>
where
    OR: Observer<T, E>,
    OE1: Observable<
            ConcatObserver<JoinedModeOf<OE1, OE2>, OR, OE2, <OE2 as ObservableTypes>::Disposal>,
            Item = T,
            Error = E,
        >,
    OE2: Observable<OR, Item = T, Error = E>,
{
    fn subscribe(self, observer: OR) -> DisposeOnDrop<Self::Disposal> {
        let subscription_2 = ReplaceableDisposal::default();
        let observer = ConcatObserver {
            observer,
            source_2: self.source_2,
            subscription_2: subscription_2.clone(),
        };
        self.source_1
            .subscribe(observer)
            .preceded_by(subscription_2)
            .map_inner_into()
    }
}

#[doc(hidden)]
pub struct ConcatObserver<M: ThreadMode, OR, OE2, D: Disposable> {
    observer: OR,
    source_2: OE2,
    subscription_2: ReplaceableDisposal<M, D>,
}

impl<M, T, E, OR, OE2> Observer<T, E> for ConcatObserver<M, OR, OE2, OE2::Disposal>
where
    M: ThreadMode,
    OR: Observer<T, E>,
    OE2: Observable<OR, Item = T, Error = E>,
{
    fn on_next(&mut self, value: T) -> Flow {
        self.observer.on_next(value)
    }

    fn on_termination(self, termination: Termination<E>) {
        match termination {
            Termination::Completed => {
                // `replace_with` does not run the builder once the subscription was disposed,
                // so the second source is never subscribed after downstream unsubscribed, and a
                // subscription built while downstream unsubscribes is disposed right away.
                self.subscription_2
                    .replace_with(|| self.source_2.subscribe(self.observer));
            }
            error @ Termination::Error(_) => {
                self.observer.on_termination(error);
            }
        }
    }
}
