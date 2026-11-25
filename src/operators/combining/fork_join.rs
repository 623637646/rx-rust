//! Emits the last item of every source once all of them have completed.
//!
//! The [`ForkJoin`] operator, behind
//! [`ObservableExt::fork_join`](crate::observable::ObservableExt::fork_join).

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
            combination::{Combination, LatestOps},
            position::{AtPosition, Position},
        },
        serialized_delivery::UpdateOutcome,
        subscribe_with_context::{self, SubscriptionContext, subscribe_with_context},
    },
};
use educe::Educe;

/// Waits for every observable of a tuple to complete, then emits the last item of each as a tuple
/// and completes.
/// See <https://reactivex.io/documentation/operators/zip.html> (`forkJoin`).
///
/// The way to run several pieces of work side by side and act once all of them are done,
/// typically requests, each an observable of one value: the sources are a tuple of one to eight
/// observables with one error type, of items of any types, and the result is the tuple of their
/// last items, in the same order. Nothing is cloned.
/// [`ObservableExt::fork_join`](crate::observable::ObservableExt::fork_join) is the form for two;
/// [`ForkJoin::new_from_iter`] is the form for any number of observables of one type, decided as
/// the program runs, and emits a `Vec`.
///
/// An error of any of them ends the stream at once. One that completes without having emitted
/// anything leaves no result to report, so the stream completes at once, without a value.
///
/// The tuple and iterator forms share the operator's state machine and retain their own storage
/// and output types.
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt,
///     observer::{Observer, Termination},
///     operators::combining::fork_join::ForkJoin,
///     subject::publish_subject::PublishSubject,
/// };
/// use std::convert::Infallible;
///
/// let mut values = Vec::new();
/// let mut user = PublishSubject::<_, Infallible, _>::local();
/// let mut unread = PublishSubject::local();
///
/// let subscription = ForkJoin::new((user.clone(), unread.clone())).subscribe_with_callback(
///     |value| values.push(value),
///     |termination| assert_eq!(termination, Termination::Completed),
/// );
///
/// let _ = unread.on_next(3);
/// let _ = user.on_next("draft");
/// let _ = user.on_next("alice"); // Only the last item counts.
/// user.on_termination(Termination::Completed);
/// unread.on_termination(Termination::Completed); // Both are done.
///
/// drop(subscription);
/// assert_eq!(values, [("alice", 3)]);
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct ForkJoin<S> {
    sources: S,
}

impl<S> ForkJoin<S> {
    /// Creates a [`ForkJoin`] over the tuple `sources`;
    /// [`ObservableExt::fork_join`](crate::observable::ObservableExt::fork_join) is the fluent form
    /// for two.
    pub fn new(sources: S) -> Self
    where
        S: SourceSet,
    {
        Self { sources }
    }
}

impl<I> ForkJoin<IterSources<I>> {
    /// Creates a [`ForkJoin`] over a finite iterator of observables of one type.
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

impl<S> ObservableTypes for ForkJoin<S>
where
    S: SourceSet,
{
    type Item = S::Item;
    type Error = S::Error;
    type Mode = S::Mode;
    /// The sources are disposed the last subscribed first.
    type Disposal = Disposal<S::Mode, S::Item, S::Error, S::SubscriptionSlots>;
}

/// The context of a [`ForkJoin`] over `S`, subscribed with `OR`.
type ContextOf<S, OR> = Context<
    <S as SourceSet>::Mode,
    <S as SourceSet>::Item,
    <S as SourceSet>::Error,
    OR,
    <S as SourceSet>::SubscriptionSlots,
>;

impl<S, OR> Observable<OR> for ForkJoin<S>
where
    S: SourceOps,
    <S as SourceSet>::CollectedCursor:
        SubscribeBatch<ContextOf<S, OR>, <S as SourceSet>::SubscriptionSlots>,
    OR: Observer<<S as SourceSet>::Item, <S as SourceSet>::Error>,
    <S as SourceSet>::Item: LatestOps,
{
    fn subscribe(self, observer: OR) -> DisposeOnDrop<Self::Disposal> {
        let (sources, remaining) = self.sources.into_collected_cursor();
        let model = Model {
            last: <<S as SourceSet>::Item as LatestOps>::empty(remaining),
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
    /// The last item of each source, `None` until it has one.
    last: T::Latest,
    /// Sources that have not completed.
    remaining: usize,
}

/// The context of a [`ForkJoin`] whose items are `T`.
type Context<M, T, E, OR, D> = SubscriptionContext<M, T, E, OR, Model<T>, D>;

/// Builds the observer of each source.
impl<M: ThreadMode, T: Combination, E, OR, D: Disposable> ObserverFamily
    for Context<M, T, E, OR, D>
{
    type At<K: Position> = ForkJoinObserver<K, M, T, E, OR, D>;

    fn at<K: Position>(&self, position: K) -> Self::At<K> {
        ForkJoinObserver(self.clone(), position)
    }

    fn is_active(&self) -> bool {
        SubscriptionContext::is_active(self)
    }
}

/// The observer of a source at a statically or dynamically typed position.
#[doc(hidden)]
pub struct ForkJoinObserver<K, M: ThreadMode, T: Combination, E, OR, D: Disposable>(
    Context<M, T, E, OR, D>,
    K,
);

impl<K: Copy, M, T, TI, E, OR, D> Observer<TI, E> for ForkJoinObserver<K, M, T, E, OR, D>
where
    M: ThreadMode,
    OR: Observer<T, E>,
    D: Disposable,
    T: LatestOps,
    T::Latest: AtPosition<K, Element = Option<TI>>,
{
    fn on_next(&mut self, value: TI) -> Flow {
        // The item this one replaces is dropped outside the lock.
        self.0
            .update(|model| {
                UpdateOutcome::empty()
                    .with_drop_outside(model.last.at_mut(self.1).replace(value))
                    .without_events()
            })
            .flow()
    }

    fn on_termination(self, termination: Termination<E>) {
        let _ = self.0.update(|model| match termination {
            Termination::Completed if model.last.at(self.1).is_some() => {
                model.remaining -= 1;
                if model.remaining == 0 {
                    // Every other source completed with a value, or the stream would have ended.
                    let combination = T::take_all(&mut model.last);
                    UpdateOutcome::empty()
                        .with_next_and_termination_events(combination, termination)
                } else {
                    UpdateOutcome::empty().without_events()
                }
            }
            // An error or a source completing without a value leaves no result to report.
            _ => UpdateOutcome::empty().with_termination_event(termination),
        });
    }
}
