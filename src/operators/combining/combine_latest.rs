//! Emits the latest item of every source, whenever any of them emits.
//!
//! The [`CombineLatest`] operator, behind
//! [`ObservableExt::combine_latest`](crate::observable::ObservableExt::combine_latest).

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
            combination::{CloneLatest, Combination, LatestOps},
            position::{AtPosition, Position},
        },
        serialized_delivery::UpdateOutcome,
        subscribe_with_context::{self, SubscriptionContext, subscribe_with_context},
    },
};
use educe::Educe;

/// Emits the latest values of several observables as a tuple whenever any of them emits, once all
/// of them have emitted.
/// See <https://reactivex.io/documentation/operators/combinelatest.html>
///
/// The sources are a tuple of one to eight observables with one error type, of items of any types:
/// the combinations are tuples of their items, in the same order.
/// [`ObservableExt::combine_latest`](crate::observable::ObservableExt::combine_latest) is the form
/// for two; [`CombineLatest::new_from_iter`] is the one for any number of observables of one
/// type, decided as the program runs, and emits a `Vec`.
///
/// Each combination is cloned from the latest values under the operator's lock, so the items'
/// `Clone` must not reach back into this pipeline.
///
/// The stream completes once every source has completed, or as soon as one completes without
/// having emitted anything, since no combination can be made then; an error of any of them ends
/// it.
///
/// The tuple and iterator forms share the operator's state machine and retain their own storage
/// and output types.
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt,
///     observer::{Observer, Termination},
///     operators::combining::combine_latest::CombineLatest,
///     subject::behavior_subject::BehaviorSubject,
/// };
/// use std::convert::Infallible;
///
/// let mut values = Vec::new();
///
/// let mut subject_1 = BehaviorSubject::local(0);
/// let mut subject_2 = BehaviorSubject::<_, Infallible, _>::local("a");
/// let mut subject_3 = BehaviorSubject::local(0.5);
///
/// let subscription = CombineLatest::new((subject_1.clone(), subject_2.clone(), subject_3.clone()))
///     .subscribe_with_callback(
///         |value| values.push(value),
///         |termination| assert_eq!(termination, Termination::Completed),
///     );
///
/// let _ = subject_1.on_next(1);
/// let _ = subject_2.on_next("b");
/// subject_1.on_termination(Termination::Completed);
/// subject_2.on_termination(Termination::Completed);
/// subject_3.on_termination(Termination::Completed);
/// drop(subscription);
///
/// assert_eq!(values, vec![(0, "a", 0.5), (1, "a", 0.5), (1, "b", 0.5)]);
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct CombineLatest<S> {
    sources: S,
}

impl<S> CombineLatest<S> {
    /// Creates a [`CombineLatest`] over the tuple `sources`;
    /// [`ObservableExt::combine_latest`](crate::observable::ObservableExt::combine_latest) is the
    /// fluent form for two.
    pub fn new(sources: S) -> Self
    where
        S: SourceSet,
    {
        Self { sources }
    }
}

impl<I> CombineLatest<IterSources<I>> {
    /// Creates a [`CombineLatest`] over a finite iterator of observables of one type.
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
    Disposal<M, T, E, D>,
    subscribe_with_context::Disposal<M, T, E, Model<T>, D>,
    where M: ThreadMode, T: Combination, D: Disposable
);

impl<S> ObservableTypes for CombineLatest<S>
where
    S: SourceSet,
{
    type Item = S::Item;
    type Error = S::Error;
    type Mode = S::Mode;
    /// The sources are disposed the last subscribed first.
    type Disposal = Disposal<S::Mode, S::Item, S::Error, S::SubscriptionSlots>;
}

/// The context of a [`CombineLatest`] over `S`, subscribed with `OR`.
type ContextOf<S, OR> = Context<
    <S as SourceSet>::Mode,
    <S as SourceSet>::Item,
    <S as SourceSet>::Error,
    OR,
    <S as SourceSet>::SubscriptionSlots,
>;

impl<S, OR> Observable<OR> for CombineLatest<S>
where
    S: SourceOps,
    <S as SourceSet>::CollectedCursor:
        SubscribeBatch<ContextOf<S, OR>, <S as SourceSet>::SubscriptionSlots>,
    OR: Observer<<S as SourceSet>::Item, <S as SourceSet>::Error>,
    <S as SourceSet>::Item: CloneLatest,
{
    fn subscribe(self, observer: OR) -> DisposeOnDrop<Self::Disposal> {
        let (sources, remaining) = self.sources.into_collected_cursor();
        let model = Model {
            latest: <<S as SourceSet>::Item as LatestOps>::empty(remaining),
            remaining,
        };
        subscribe_with_context(observer, model, |context| {
            if remaining == 0 {
                context.send_termination(Termination::Completed);
            }
            sources.subscribe_batch(&context)
        })
        .map_inner_into()
    }
}

struct Model<T: Combination> {
    /// The latest item of each source.
    latest: T::Latest,
    /// Sources that have not completed.
    remaining: usize,
}

/// The context of a [`CombineLatest`] whose items are `T`.
type Context<M, T, E, OR, D> = SubscriptionContext<M, T, E, OR, Model<T>, D>;

/// Builds the observer of each source.
impl<M: ThreadMode, T: Combination, E, OR, D: Disposable> ObserverFamily
    for Context<M, T, E, OR, D>
{
    type At<K: Position> = CombineLatestObserver<K, M, T, E, OR, D>;

    fn at<K: Position>(&self, position: K) -> Self::At<K> {
        CombineLatestObserver(self.clone(), position)
    }

    fn is_active(&self) -> bool {
        SubscriptionContext::is_active(self)
    }
}

/// The observer of a source at a statically or dynamically typed position.
#[doc(hidden)]
pub struct CombineLatestObserver<K, M: ThreadMode, T: Combination, E, OR, D: Disposable>(
    Context<M, T, E, OR, D>,
    K,
);

impl<K: Copy, M, T, TI, E, OR, D> Observer<TI, E> for CombineLatestObserver<K, M, T, E, OR, D>
where
    M: ThreadMode,
    OR: Observer<T, E>,
    D: Disposable,
    T: CloneLatest,
    T::Latest: AtPosition<K, Element = Option<TI>>,
{
    fn on_next(&mut self, value: TI) -> Flow {
        self.0
            .update(|model| {
                // The latest value this one replaces is handed back, so that the `Drop` of the
                // user's value runs outside the lock. The combination is cloned under it: whether
                // there is one to make at all is only known here.
                let replaced = model.latest.at_mut(self.1).replace(value);
                let outcome = UpdateOutcome::empty().with_drop_outside(replaced);
                match T::clone_all(&model.latest) {
                    Some(combination) => outcome.with_next_event(combination),
                    None => outcome.without_events(),
                }
            })
            .flow()
    }

    fn on_termination(self, termination: Termination<E>) {
        let _ = self.0.update(|model| match termination {
            completion @ Termination::Completed => {
                model.remaining -= 1;
                // A source that completes without having emitted leaves every combination
                // incomplete for good.
                if model.latest.at(self.1).is_none() || model.remaining == 0 {
                    UpdateOutcome::empty().with_termination_event(completion)
                } else {
                    UpdateOutcome::empty().without_events()
                }
            }
            error @ Termination::Error(_) => UpdateOutcome::empty().with_termination_event(error),
        });
    }
}
