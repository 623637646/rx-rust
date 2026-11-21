//! The [`Zip`] operator, behind [`ObservableExt::zip`](crate::observable::ObservableExt::zip).

use crate::{
    delegate_disposal,
    disposable::{Disposable, dispose_on_drop::DisposeOnDrop},
    observable::{
        IterSources, Observable, ObservableTypes, SourceSet,
        sources::{ObserverFamily, SourceOps, SubscribeBatch},
    },
    observer::{Flow, Observer, Termination},
    thread_mode::ThreadMode,
    utils::{
        positional::{
            combination::{Combination, QueueOps},
            position::{AtPosition, Position},
        },
        serialized_delivery::UpdateOutcome,
        subscribe_with_context::{self, SubscriptionContext, subscribe_with_context},
    },
};
use educe::Educe;
use std::collections::VecDeque;

/// Combines the items of several observables in order, as tuples: the first of each, then the
/// second of each, and so on. It completes once a source has completed and every item it emitted
/// was combined.
/// See <https://reactivex.io/documentation/operators/zip.html>
///
/// The sources are a tuple of one to eight observables with one error type, of items of any types,
/// and an item waits until every other source has emitted its counterpart. Nothing is cloned.
/// [`ObservableExt::zip`](crate::observable::ObservableExt::zip) is the form for two;
/// [`Zip::new_from_iter`] is the one for any number of observables of one type, decided as the
/// program runs, and emits a `Vec`.
///
/// The tuple and iterator forms share the operator's state machine and retain their own storage
/// and output types.
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt,
///     observer::Termination,
///     operators::{
///         combining::zip::Zip,
///         creating::from_iter::FromIter,
///     },
/// };
///
/// let mut values = Vec::new();
///
/// let observable = Zip::new((
///     FromIter::new([1, 2, 3]),
///     FromIter::new(["a", "b"]),
///     FromIter::new([0.1, 0.2, 0.3]),
/// ));
/// observable.subscribe_with_callback(
///     |value| values.push(value),
///     |termination| assert_eq!(termination, Termination::Completed),
/// );
///
/// assert_eq!(values, vec![(1, "a", 0.1), (2, "b", 0.2)]);
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct Zip<S> {
    sources: S,
}

impl<S> Zip<S> {
    /// Creates a [`Zip`] over the tuple `sources`;
    /// [`ObservableExt::zip`](crate::observable::ObservableExt::zip) is the fluent form for two.
    pub fn new(sources: S) -> Self
    where
        S: SourceSet,
    {
        Self { sources }
    }
}

impl<I> Zip<IterSources<I>> {
    /// Creates a [`Zip`] over a finite iterator of observables of one type.
    ///
    /// Collects the sources at subscription before subscribing to any of them. Each output is a
    /// `Vec` in iterator order; an empty iterator completes without emitting a value.
    pub fn new_from_iter(sources: I) -> Self {
        Self {
            sources: IterSources(sources),
        }
    }
}

delegate_disposal!(
    Disposal<M, T, E, Q, D>,
    subscribe_with_context::Disposal<M, T, E, Model<Q>, D>,
    where M: ThreadMode, D: Disposable
);

impl<S> ObservableTypes for Zip<S>
where
    S: SourceSet,
{
    type Item = S::Item;
    type Error = S::Error;
    type Mode = S::Mode;
    /// The sources are disposed the last subscribed first.
    type Disposal = Disposal<
        S::Mode,
        S::Item,
        S::Error,
        <S::Item as Combination>::Queues,
        S::SubscriptionSlots,
    >;
}

/// The context of a [`Zip`] over `S`, subscribed with `OR`.
type ContextOf<S, OR> = Context<
    <S as SourceSet>::Mode,
    <S as SourceSet>::Item,
    <S as SourceSet>::Error,
    OR,
    <<S as SourceSet>::Item as Combination>::Queues,
    <S as SourceSet>::SubscriptionSlots,
>;

impl<S, OR> Observable<OR> for Zip<S>
where
    S: SourceOps,
    <S as SourceSet>::CollectedCursor:
        SubscribeBatch<ContextOf<S, OR>, <S as SourceSet>::SubscriptionSlots>,
    OR: Observer<<S as SourceSet>::Item, <S as SourceSet>::Error>,
    <S as SourceSet>::Item: QueueOps,
{
    fn subscribe(self, observer: OR) -> DisposeOnDrop<Self::Disposal> {
        let (sources, position_count) = self.sources.into_collected_cursor();
        let model = Model::empty::<<S as SourceSet>::Item>(position_count);
        // Every queue starts empty: none to be empty means there is no source.
        let no_source = model.empty == 0;
        subscribe_with_context(observer, model, |context| {
            if no_source {
                context.send_termination(Termination::Completed);
            }
            sources.subscribe_batch(&context)
        })
        .map_inner_into()
    }
}

struct Model<Q> {
    /// The items of each source waiting for their counterparts.
    queues: Q,
    /// How many queues are empty: a combination is made as soon as none is.
    empty: usize,
    /// How many more combinations can be made, once a source has completed: the fewest items
    /// left in the queue of a completed source, since such a queue only shrinks.
    remaining: Option<usize>,
}

impl<Q> Model<Q> {
    /// Creates `position_count` empty queues: a tuple's arity, or the length of a list.
    fn empty<T: QueueOps<Queues = Q>>(position_count: usize) -> Self {
        Self {
            queues: T::empty(position_count),
            empty: position_count,
            remaining: None,
        }
    }
}

/// The context of a [`Zip`] whose items are `T` and queues `Q`.
type Context<M, T, E, OR, Q, D> = SubscriptionContext<M, T, E, OR, Model<Q>, D>;

/// Builds the observer of each source.
impl<M: ThreadMode, T, E, OR, Q, D: Disposable> ObserverFamily for Context<M, T, E, OR, Q, D> {
    type At<K: Position> = ZipObserver<K, M, T, E, OR, Q, D>;

    fn at<K: Position>(&self, position: K) -> Self::At<K> {
        ZipObserver(self.clone(), position)
    }

    fn is_active(&self) -> bool {
        SubscriptionContext::is_active(self)
    }
}

/// The observer of a source at a statically or dynamically typed position.
#[doc(hidden)]
pub struct ZipObserver<K, M: ThreadMode, T, E, OR, Q, D: Disposable>(Context<M, T, E, OR, Q, D>, K);

impl<K: Copy, M, T, TI, E, OR, Q, D> Observer<TI, E> for ZipObserver<K, M, T, E, OR, Q, D>
where
    M: ThreadMode,
    OR: Observer<T, E>,
    D: Disposable,
    T: QueueOps<Queues = Q>,
    Q: AtPosition<K, Element = VecDeque<TI>>,
{
    fn on_next(&mut self, value: TI) -> Flow {
        self.0
            .update(|model| {
                let queue = model.queues.at_mut(self.1);
                if queue.is_empty() {
                    model.empty -= 1;
                }
                queue.push_back(value);
                if model.empty != 0 {
                    return UpdateOutcome::empty().without_events();
                }
                let (combination, empty) = T::pop_all(&mut model.queues);
                model.empty = empty;
                match &mut model.remaining {
                    Some(remaining) => {
                        *remaining -= 1;
                        if *remaining == 0 {
                            UpdateOutcome::empty().with_next_and_termination_events(
                                combination,
                                Termination::Completed,
                            )
                        } else {
                            UpdateOutcome::empty().with_next_event(combination)
                        }
                    }
                    None => UpdateOutcome::empty().with_next_event(combination),
                }
            })
            .flow()
    }

    fn on_termination(self, termination: Termination<E>) {
        let _ = self.0.update(|model| match termination {
            completion @ Termination::Completed => {
                let left = model.queues.at(self.1).len();
                let remaining = model
                    .remaining
                    .map_or(left, |remaining| remaining.min(left));
                if remaining == 0 {
                    UpdateOutcome::empty().with_termination_event(completion)
                } else {
                    model.remaining = Some(remaining);
                    UpdateOutcome::empty().without_events()
                }
            }
            error @ Termination::Error(_) => UpdateOutcome::empty().with_termination_event(error),
        });
    }
}
