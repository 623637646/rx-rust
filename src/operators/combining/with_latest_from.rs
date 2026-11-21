//! The [`WithLatestFrom`] operator, behind
//! [`ObservableExt::with_latest_from`](crate::observable::ObservableExt::with_latest_from).

use crate::{
    delegate_disposal,
    disposable::{Disposable, dispose_on_drop::DisposeOnDrop, option_disposal::OptionDisposal},
    observable::{
        IterSources, Observable, ObservableTypes, SourceSet,
        sources::{ObserverFamily, SourceOps, SubscribeBatch},
    },
    observer::{Flow, Observer, Termination},
    thread_mode::{Joined, ThreadMode},
    utils::{
        positional::{
            combination::{CloneLatest, Combination, LatestOps, PrependOps},
            position::{AtPosition, Position},
        },
        serialized_delivery::UpdateOutcome,
        subscribe_with_context::{self, SubscriptionContext, subscribe_with_context},
    },
};
use educe::Educe;

/// Combines each item of the source with the latest item of every other observable, emitting only
/// when the source emits.
/// See <https://reactivex.io/documentation/operators/combinelatest.html> (`withLatestFrom`).
///
/// Unlike [`CombineLatest`](crate::operators::combining::combine_latest::CombineLatest), the
/// other observables only supply values: what they emit is remembered, not emitted. They are a
/// tuple of one to eight observables with the source's error type, of items of any types, and each
/// combination is the item of the source followed by their latest items, in their order. An item
/// of the source that arrives before every other observable has emitted something is dropped.
///
/// The other observables are subscribed first, so ones that emit as they are subscribed — a
/// [`BehaviorSubject`](crate::subject::behavior_subject::BehaviorSubject), a
/// [`Just`](crate::operators::creating::just::Just) — have their values in place for the source's
/// first item. The stream ends with the source: its completion or its error. The completion of
/// another observable changes nothing, its latest value is kept, while its error ends the stream.
///
/// The latest values are cloned under the operator's lock. Their `Clone` implementations must
/// avoid calling back into this pipeline.
///
/// [`WithLatestFrom::new_from_iter`] accepts a homogeneous iterator of auxiliary sources and
/// emits `(primary_item, Vec<auxiliary_item>)`. With no auxiliary sources it emits an empty vector
/// alongside each primary item; completion is still controlled by the primary source.
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt,
///     observer::{Observer, Termination},
///     operators::combining::with_latest_from::WithLatestFrom,
///     subject::publish_subject::PublishSubject,
/// };
/// use std::convert::Infallible;
///
/// let mut values = Vec::new();
/// let mut clicks = PublishSubject::<_, Infallible, _>::local();
/// let mut theme = PublishSubject::local();
/// let mut zoom = PublishSubject::local();
///
/// let subscription = WithLatestFrom::new(clicks.clone(), (theme.clone(), zoom.clone()))
///     .subscribe_with_callback(
///         |value| values.push(value),
///         |termination| assert_eq!(termination, Termination::Completed),
///     );
///
/// let _ = theme.on_next("dark");
/// let _ = clicks.on_next(1); // No zoom yet: dropped.
/// let _ = zoom.on_next(100);
/// let _ = clicks.on_next(2);
/// let _ = theme.on_next("light"); // Remembered, not emitted.
/// let _ = clicks.on_next(3);
/// clicks.on_termination(Termination::Completed);
///
/// drop((subscription, theme, zoom));
/// assert_eq!(values, [(2, "dark", 100), (3, "light", 100)]);
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct WithLatestFrom<OE, S> {
    source: OE,
    others: S,
}

impl<OE, S> WithLatestFrom<OE, S> {
    /// Creates a [`WithLatestFrom`] over `source`, with the tuple `others`;
    /// [`ObservableExt::with_latest_from`](crate::observable::ObservableExt::with_latest_from) is
    /// the fluent form for one other observable.
    pub fn new<E>(source: OE, others: S) -> Self
    where
        OE: ObservableTypes<Error = E>,
        S: SourceSet<Error = E>,
    {
        Self { source, others }
    }
}

impl<OE, I> WithLatestFrom<OE, IterSources<I>> {
    /// Combines the primary source with a finite iterator of auxiliary sources.
    ///
    /// Outputs `(primary_item, Vec<auxiliary_item>)` on a primary event. Auxiliary sources are
    /// collected and subscribed first. With no auxiliary sources, the vector is empty.
    pub fn new_from_iter<Other, E>(source: OE, others: I) -> Self
    where
        OE: ObservableTypes<Error = E>,
        I: IntoIterator<Item = Other>,
        Other: ObservableTypes<Error = E>,
    {
        Self {
            source,
            others: IterSources(others),
        }
    }
}

/// What the context owns: the subscriptions of the other observables, then the source's, if it was
/// made — the last subscribed is disposed first.
type Subscriptions<OE, S> = (
    <S as SourceSet>::SubscriptionSlots,
    OptionDisposal<DisposeOnDrop<<OE as ObservableTypes>::Disposal>>,
);

/// The mode of a [`WithLatestFrom`]: the source's and the other observables', joined.
type ModeOf<OE, S> = Joined<<OE as ObservableTypes>::Mode, <S as SourceSet>::Mode>;

/// What a [`WithLatestFrom`] emits: the item of the source, then the latest of the others.
type ItemOf<OE, S> = <<S as SourceSet>::Item as Combination>::After<<OE as ObservableTypes>::Item>;

delegate_disposal!(
    Disposal<M, T, E, I, D>,
    subscribe_with_context::Disposal<M, T, E, Model<I>, D>,
    where M: ThreadMode, I: Combination, D: Disposable
);

impl<OE, S> ObservableTypes for WithLatestFrom<OE, S>
where
    OE: ObservableTypes,
    S: SourceSet<Error = OE::Error>,
{
    /// The item of the source, then the latest of each other observable.
    type Item = <S::Item as Combination>::After<OE::Item>;
    type Error = OE::Error;
    /// The source's and the other observables', joined.
    type Mode = Joined<OE::Mode, S::Mode>;
    /// First the source's subscription, then the other observables', the last subscribed first.
    type Disposal = Disposal<
        Joined<OE::Mode, S::Mode>,
        <S::Item as Combination>::After<OE::Item>,
        OE::Error,
        S::Item,
        (
            S::SubscriptionSlots,
            OptionDisposal<DisposeOnDrop<OE::Disposal>>,
        ),
    >;
}

/// The context of a [`WithLatestFrom`] over `OE` and `S`, subscribed with `OR`.
type ContextOf<OE, S, OR> = Context<
    ModeOf<OE, S>,
    ItemOf<OE, S>,
    <OE as ObservableTypes>::Error,
    OR,
    <S as SourceSet>::Item,
    Subscriptions<OE, S>,
>;

impl<OE, S, OR> Observable<OR> for WithLatestFrom<OE, S>
where
    OR: Observer<ItemOf<OE, S>, <OE as ObservableTypes>::Error>,
    OE: Observable<
        WithLatestFromObserver<
            ModeOf<OE, S>,
            ItemOf<OE, S>,
            <OE as ObservableTypes>::Error,
            OR,
            <S as SourceSet>::Item,
            Subscriptions<OE, S>,
        >,
    >,
    S: SourceSet<Error = <OE as ObservableTypes>::Error> + SourceOps,
    <S as SourceSet>::CollectedCursor:
        SubscribeBatch<ContextOf<OE, S, OR>, <S as SourceSet>::SubscriptionSlots>,
    <S as SourceSet>::Item: CloneLatest + PrependOps,
{
    fn subscribe(self, observer: OR) -> DisposeOnDrop<Self::Disposal> {
        let (others, position_count) = self.others.into_collected_cursor();
        let latest = Model(<<S as SourceSet>::Item as LatestOps>::empty(position_count));
        subscribe_with_context(observer, latest, |context: ContextOf<OE, S, OR>| {
            let others = others.subscribe_batch(&context);
            let source = if context.is_active() {
                OptionDisposal::some(
                    self.source
                        .subscribe(WithLatestFromObserver(context.clone())),
                )
            } else {
                OptionDisposal::none()
            };
            others.map_inner(|others| (others, source))
        })
        .map_inner_into()
    }
}

/// The latest item of each other observable. A struct, so that the impls on the context name
/// the combination `I` itself rather than a projection of it.
struct Model<I: Combination>(I::Latest);

/// The context of a [`WithLatestFrom`] whose outputs are `T` and auxiliary items are `I`.
type Context<M, T, E, OR, I, D> = SubscriptionContext<M, T, E, OR, Model<I>, D>;

/// The observer of the source.
#[doc(hidden)]
pub struct WithLatestFromObserver<M: ThreadMode, T, E, OR, I: Combination, D: Disposable>(
    Context<M, T, E, OR, I, D>,
);

impl<M, T, T0, E, OR, I, D> Observer<T0, E> for WithLatestFromObserver<M, T, E, OR, I, D>
where
    M: ThreadMode,
    OR: Observer<T, E>,
    D: Disposable,
    I: CloneLatest + PrependOps + Combination<After<T0> = T>,
{
    fn on_next(&mut self, value: T0) -> Flow {
        self.0
            .update(|latest| match I::clone_all(&latest.0) {
                Some(combination) => UpdateOutcome::empty()
                    .without_drop_outside()
                    .with_next_event(combination.prepend(value)),
                // Nothing to combine it with yet: the value is dropped outside the lock, since
                // dropping it can run arbitrary code.
                None => UpdateOutcome::empty()
                    .with_drop_outside(value)
                    .without_events(),
            })
            .flow()
    }

    fn on_termination(self, termination: Termination<E>) {
        self.0.send_termination(termination);
    }
}

/// Builds the observer of each other observable.
impl<M: ThreadMode, T, E, OR, I: Combination, D: Disposable> ObserverFamily
    for Context<M, T, E, OR, I, D>
{
    type At<K: Position> = WithLatestFromOtherObserver<K, M, T, E, OR, I, D>;

    fn at<K: Position>(&self, position: K) -> Self::At<K> {
        WithLatestFromOtherObserver(self.clone(), position)
    }

    fn is_active(&self) -> bool {
        SubscriptionContext::is_active(self)
    }
}

/// The observer of an auxiliary source at a typed position.
#[doc(hidden)]
pub struct WithLatestFromOtherObserver<K, M: ThreadMode, T, E, OR, I: Combination, D: Disposable>(
    Context<M, T, E, OR, I, D>,
    K,
);

impl<K: Copy, M, T, TI, E, OR, I, D> Observer<TI, E>
    for WithLatestFromOtherObserver<K, M, T, E, OR, I, D>
where
    M: ThreadMode,
    OR: Observer<T, E>,
    D: Disposable,
    I: Combination,
    I::Latest: AtPosition<K, Element = Option<TI>>,
{
    fn on_next(&mut self, value: TI) -> Flow {
        // The value it replaces is dropped outside the lock.
        self.0
            .update(|latest| {
                let replaced = latest.0.at_mut(self.1).replace(value);
                UpdateOutcome::empty()
                    .with_drop_outside(replaced)
                    .without_events()
            })
            .flow()
    }

    fn on_termination(self, termination: Termination<E>) {
        match termination {
            // The latest value stays, and so does the stream: the source decides when it ends.
            Termination::Completed => {}
            error @ Termination::Error(_) => self.0.send_termination(error),
        }
    }
}
