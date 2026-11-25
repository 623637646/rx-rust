//! Combines several sources by join patterns: ReactiveX's `And` / `Then` / `When`.
//!
//! The [`When`] operator and the [`Pattern`]s and [`Plan`]s it is built from.

use crate::{
    delegate_disposal,
    disposable::{Disposable, dispose_on_drop::DisposeOnDrop},
    observable::{
        Observable, ObservableTypes, SourceSet,
        sources::{SourceOps, SourceSubscriber, SubscribeEach},
    },
    observer::{Flow, Observer, Termination},
    thread_mode::ThreadMode,
    utils::{
        MarkerType,
        positional::{
            position::{AtPosition, TupleIndex},
            tuple::{IntoReversed, for_each_arity},
        },
        serialized_delivery::{StopView, UpdateOutcome},
        subscribe_with_context::{
            self, SubscriptionContext, subscribe_with_context_with_stop_view,
        },
    },
};
use educe::Educe;
use std::{collections::VecDeque, convert::Infallible, marker::PhantomData, ops::ControlFlow};

/// Combines several observables through join patterns: each [`Plan`] waits until every source of
/// its [`Pattern`] has an item, takes the first item of each, and emits what its selector makes of
/// them. Plans may share a source, and an item goes to the first plan, in the order given, that it
/// completes.
/// See <https://reactivex.io/documentation/operators/and-then-when.html>
///
/// The sources are a tuple of one to eight observables with one error type. `build` receives a
/// tuple of [`SourceHandle`]s, one per source and in the same order, which
/// [`and`](SourceHandle::and) joins into a pattern and [`then`](Pattern::then) turns into a plan.
/// It returns one plan, or a tuple of up to eight plans with one output type.
///
/// Each source is subscribed once, in order, and its items wait in a queue of its own until a plan
/// takes them. A plan retires once it can no longer match, because one of its sources has
/// completed with nothing left in its queue: it takes no more items, though its selector is kept,
/// with the others, until the subscription ends. A source no remaining plan uses is then disposed,
/// and what it queued is dropped; one that no plan names, or none uses any more by its turn to be
/// subscribed, is never subscribed. The stream completes once every plan has retired, and fails with
/// the first error of a source still in use. With one plan, `When` pairs its sources as
/// [`Zip`](super::zip::Zip) does.
///
/// A selector runs outside the lock. If it disposes the subscription, what it returns is not
/// emitted, and nothing queued behind it is either, a completion included.
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt, observer::Observer, operators::combining::when::When,
///     subject::publish_subject::PublishSubject,
/// };
/// use std::convert::Infallible;
///
/// let mut receipts = Vec::new();
/// let mut orders = PublishSubject::<_, Infallible, _>::local();
/// let mut cash = PublishSubject::local();
/// let mut cards = PublishSubject::local();
///
/// let subscription = When::new(
///     (orders.clone(), cash.clone(), cards.clone()),
///     |(order, cash, card)| {
///         (
///             order.and(cash).then(|order, amount| format!("{order}: {amount} in cash")),
///             order.and(card).then(|order, number| format!("{order}: card {number}")),
///         )
///     },
/// )
/// .subscribe_with_callback(|receipt| receipts.push(receipt), |_| {});
///
/// let _ = orders.on_next("tea");
/// let _ = cards.on_next(1234); // The order goes to the plan whose payment comes first.
/// let _ = cash.on_next(5); // Waits for an order.
/// let _ = orders.on_next("cake");
///
/// drop((subscription, orders, cash, cards));
/// assert_eq!(receipts, ["tea: card 1234", "cake: 5 in cash"]);
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct When<S, P> {
    sources: S,
    plans: P,
}

impl<S, P> When<S, P> {
    /// Creates a [`When`] over `sources`, with the plans `build` makes of their handles.
    pub fn new<F>(sources: S, build: F) -> Self
    where
        S: Sources,
        F: FnOnce(S::Handles) -> P,
    {
        Self {
            sources,
            plans: build(S::handles()),
        }
    }
}

/// A handle to the source at `INDEX` of a [`When`], whose items are `T`: what [`When::new`] hands
/// to its `build`, one per source, to be joined into [`Pattern`]s.
///
/// A handle is `Copy`, so that several patterns can name the same source.
#[derive(Educe)]
#[educe(Debug(bound()), Clone(bound()), Copy(bound()))]
pub struct SourceHandle<const INDEX: usize, T>(MarkerType<T>);

impl<const INDEX: usize, T> SourceHandle<INDEX, T> {
    fn new() -> Self {
        Self(PhantomData)
    }

    /// Joins this source and `other` into a [`Pattern`], which matches once each has an item.
    ///
    /// A pattern names each source once. That is checked as the code is built, after type
    /// checking, so `cargo build` and `cargo test` report a source named twice, but `cargo check`
    /// does not:
    /// ```rust
    /// use rx_rust::operators::{combining::when::When, creating::from_iter::FromIter};
    ///
    /// When::new((FromIter::new([1]), FromIter::new([2])), |(a, b)| {
    ///     a.and(b).then(|a, b| a + b)
    /// });
    /// ```
    /// ```rust,compile_fail
    /// use rx_rust::operators::{combining::when::When, creating::from_iter::FromIter};
    ///
    /// When::new((FromIter::new([1]), FromIter::new([2])), |(a, b)| {
    ///     a.and(a).then(|a, b| a + b)
    /// });
    /// ```
    pub fn and<const OTHER: usize, U>(
        self,
        _other: SourceHandle<OTHER, U>,
    ) -> Pattern<(Self, SourceHandle<OTHER, U>)> {
        const { assert!(OTHER != INDEX, "a pattern names each source once") };
        Pattern::new()
    }

    /// A [`Plan`] of this source alone, which passes each of its items through `selector`.
    pub fn then<R, F>(self, selector: F) -> Plan<(Self,), F>
    where
        F: FnMut(T) -> R,
    {
        Plan::new(selector)
    }
}

/// The sources a [`Plan`] waits on, all of them at once: made by [`SourceHandle::and`], extended
/// by [`and`](Pattern::and) up to eight sources, and turned into a plan by
/// [`then`](Pattern::then). `H` is the tuple of their [`SourceHandle`]s.
///
/// Every added source must differ from all the sources already in the pattern:
/// ```rust
/// use rx_rust::operators::{combining::when::When, creating::from_iter::FromIter};
///
/// When::new(
///     (FromIter::new([1]), FromIter::new([2]), FromIter::new([3])),
///     |(a, b, c)| a.and(b).and(c).then(|a, b, c| a + b + c),
/// );
/// ```
/// ```rust,compile_fail
/// use rx_rust::operators::{combining::when::When, creating::from_iter::FromIter};
///
/// When::new(
///     (FromIter::new([1]), FromIter::new([2]), FromIter::new([3])),
///     |(a, b, c)| a.and(b).and(a).then(|a, b, c| a + b + c),
/// );
/// ```
#[derive(Educe)]
#[educe(Debug(bound()), Clone(bound()), Copy(bound()))]
pub struct Pattern<H>(#[educe(Debug(ignore))] MarkerType<H>);

impl<H> Pattern<H> {
    fn new() -> Self {
        Self(PhantomData)
    }
}

/// A [`Pattern`] and the selector its matches go through: one of the plans of a [`When`].
///
/// Made by [`Pattern::then`], or by [`SourceHandle::then`] for a single source. `H` is the tuple of
/// the [`SourceHandle`]s of its sources, and `F` its selector.
#[derive(Educe)]
#[educe(Debug, Clone(bound(F: Clone)))]
pub struct Plan<H, F> {
    selector: F,
    #[educe(Debug(ignore))]
    _pattern: MarkerType<H>,
}

impl<H, F> Plan<H, F> {
    fn new(selector: F) -> Self {
        Self {
            selector,
            _pattern: PhantomData,
        }
    }
}

mod sealed {
    pub trait Sealed {}
}

impl<H, F> sealed::Sealed for Plan<H, F> {}

/// The sources of a [`When`]: a tuple of one to eight observables with one error type.
pub trait Sources: SourceSet {
    /// The tuple of the [`SourceHandle`]s of the sources, in their order.
    type Handles;

    /// The queue and subscription of each source, as a subscription keeps them.
    #[doc(hidden)]
    type Inputs;

    #[doc(hidden)]
    fn handles() -> Self::Handles;
}

/// The plans of a [`When`]: a [`Plan`], or a tuple of one to eight plans with one output type.
///
/// It is sealed: a [`When`] matches its plans against the queues of its sources through a
/// crate-private trait that only these implement.
///
// No twin compiles here, so nothing proves that the sealing is what fails it: keep every item of
// `Plans` implemented as the trait changes, or a missing one would fail the example too, and it
// would pass with the sealing gone.
/// ```compile_fail
/// use rx_rust::operators::combining::when::Plans;
///
/// struct Custom;
///
/// impl Plans for Custom {
///     type Output = i32;
///     type Matched = i32;
///
///     fn select(&mut self, matched: i32) -> i32 {
///         matched
///     }
/// }
/// ```
pub trait Plans: sealed::Sealed {
    /// What the selectors return: the items of the [`When`].
    type Output;

    /// What a match hands from under the lock to have its selector run outside it: the items it
    /// took, which a tuple of plans records with the path its search took: one
    /// [`ControlFlow::Continue`] per plan passed over, then the [`ControlFlow::Break`] of the one
    /// that matched. The last plan's is not wrapped.
    #[doc(hidden)]
    type Matched;

    /// Runs the selector of the plan that `matched`.
    #[doc(hidden)]
    fn select(&mut self, matched: Self::Matched) -> Self::Output;
}

/// [`Plans`] matched against the queues `Q` of the sources.
///
/// A plan retires once a source of its pattern has completed with nothing left in its queue, or
/// has been released: that lasts, as a completed source queues nothing more and a released one
/// stays released, so whether a plan can still match is read from the queues each time, never
/// stored.
trait PlansOver<Q>: Plans {
    /// Takes the items of the first plan that can match.
    ///
    /// Called each time a source queues an item, it finds one plan at most: none could match
    /// before the item came, so only a plan that waits on that source can now, and only if the
    /// item is the one its queue lacked — which the first such plan takes. A retired plan never
    /// matches, as the source that retired it stays empty.
    fn take(inputs: &mut Q) -> Option<Self::Matched>;

    /// The sources that the plans not retired wait on, as a set of bits: bit `i` for the source
    /// at `i`. None is left once every plan has retired.
    fn needed(inputs: &Q) -> u8;
}

/// The queues of the sources of a [`When`]: a tuple of one [`Input`] per source, `None` once the
/// source is released — no plan waits on it any more, or none ever did. Its subscription is then
/// disposed, or never made, and its items are rejected.
trait Inputs: IntoReversed {
    /// The inputs of a new subscription, none released.
    fn new() -> Self;

    /// Releases every input not released yet that is not `needed` any more: bit `i` keeps the
    /// source at `i`. What it takes out is to drop outside the lock, reversed, so that it drops
    /// the last source first, as a disposal does.
    fn release_unused(&mut self, needed: u8) -> Self::Reversed;
}

/// What a [`When`] keeps of one source still in use: its queue, and its subscription. Released,
/// it is dropped whole, outside the lock: the queue first, then the subscription.
#[doc(hidden)]
#[derive(Educe)]
#[educe(Default(bound()))]
pub struct Input<T, D: Disposable> {
    items: VecDeque<T>,
    completed: bool,
    /// `None` until the source's `subscribe` returns, which happens once: a subscription that
    /// returns after a release is dropped there instead of stored.
    subscription: Option<DisposeOnDrop<D>>,
}

impl<T, D: Disposable> Input<T, D> {
    /// Whether the input of a source can give no more items: it completed with nothing left in
    /// its queue, or it is released.
    fn is_exhausted(input: &Option<Self>) -> bool {
        input
            .as_ref()
            .is_none_or(|input| input.completed && input.items.is_empty())
    }

    /// Whether the input of a source has an item queued, which a released one never has.
    fn has_item(input: &Option<Self>) -> bool {
        input.as_ref().is_some_and(|input| !input.items.is_empty())
    }

    /// Takes the first item queued by a source that [`has_item`](Self::has_item) accepted.
    fn pop_item(input: &mut Option<Self>) -> T {
        input
            .as_mut()
            .and_then(|input| input.items.pop_front())
            .expect("a matched source has a queued item")
    }
}

delegate_disposal!(
    Disposal<M, MT, E, Q>,
    subscribe_with_context::Disposal<M, MT, E, Model<Q>, ()>,
    where M: ThreadMode
);

/// The model of the context: the inputs of the sources.
///
/// It takes no [`Inputs`] bound, which its `Drop` would then need too, and so would the public
/// [`Disposal`] naming it, making [`Inputs`] public: what `Drop` runs is a function `new` picks,
/// where the bound holds.
struct Model<Q> {
    inputs: Q,
    /// Releases every input, the last source first.
    release_all: fn(&mut Q),
}

impl<Q: Inputs> Model<Q> {
    /// The model of a new subscription, whose sources that no plan names are released already,
    /// never to be subscribed.
    fn new<P: PlansOver<Q>>() -> Self {
        let mut model = Self {
            inputs: Q::new(),
            release_all: |inputs| drop(inputs.release_unused(0)),
        };
        // Nothing is queued or subscribed yet, so the release has nothing to drop, and every plan
        // waits on some source, so none has retired.
        let _ = model.settle::<P>();
        model
    }

    /// Releases the sources that no plan still able to match waits on. Returns what to drop
    /// outside the lock, and whether every plan has retired.
    fn settle<P: PlansOver<Q>>(&mut self) -> (Q::Reversed, bool) {
        let needed = P::needed(&self.inputs);
        (self.inputs.release_unused(needed), needed == 0)
    }
}

impl<Q> Drop for Model<Q> {
    fn drop(&mut self) {
        // Disposes what is still subscribed, the last source first, as a release does: a plain
        // drop of the tuple would dispose the first source first.
        (self.release_all)(&mut self.inputs);
    }
}

/// The observer of the context: runs the selectors of what matched, outside the lock and in order,
/// and sends what they return downstream unless disposed before or during the selection.
struct SelectObserver<M: ThreadMode, OR, P> {
    observer: OR,
    plans: P,
    stop_view: StopView<M>,
}

impl<M, E, OR, P> Observer<P::Matched, E> for SelectObserver<M, OR, P>
where
    M: ThreadMode,
    OR: Observer<P::Output, E>,
    P: Plans,
{
    fn on_next(&mut self, matched: P::Matched) -> Flow {
        self.stop_view
            .send(&mut self.observer, || self.plans.select(matched))
    }

    fn on_termination(self, termination: Termination<E>) {
        self.observer.on_termination(termination);
    }
}

type Context<M, E, OR, Q, P> =
    SubscriptionContext<M, <P as Plans>::Matched, E, SelectObserver<M, OR, P>, Model<Q>, ()>;

#[doc(hidden)]
pub struct WhenObserver<const INDEX: usize, M: ThreadMode, E, OR, Q, P: Plans>(
    Context<M, E, OR, Q, P>,
);

impl<const INDEX: usize, M, T, E, OR, Q, D, P> Observer<T, E>
    for WhenObserver<INDEX, M, E, OR, Q, P>
where
    M: ThreadMode,
    OR: Observer<P::Output, E>,
    Q: Inputs + AtPosition<TupleIndex<INDEX>, Element = Option<Input<T, D>>>,
    D: Disposable,
    P: PlansOver<Q>,
{
    fn on_next(&mut self, value: T) -> Flow {
        self.0
            .update(|model| {
                let Some(input) = model.inputs.at_mut(TupleIndex::<INDEX>) else {
                    return UpdateOutcome::empty()
                        .without_events()
                        .with_drop_outside((Some(value), None))
                        .answering_stop();
                };
                input.items.push_back(value);
                let Some(matched) = P::take(&mut model.inputs) else {
                    // Nothing taken, so no plan retires: a plan retires on a source completed with
                    // an empty queue, and queueing an item empties no queue.
                    return UpdateOutcome::without_effects();
                };
                let (released, finished) = model.settle::<P>();
                let outcome = UpdateOutcome::empty();
                let outcome = if finished {
                    outcome.with_next_and_termination_events(matched, Termination::Completed)
                } else {
                    outcome.with_next_event(matched)
                }
                .with_drop_outside((None, Some(released)));
                // The plans that waited on this source have retired: it must send nothing more.
                if model.inputs.at(TupleIndex::<INDEX>).is_none() {
                    outcome.answering_stop()
                } else {
                    outcome
                }
            })
            .flow()
    }

    fn on_termination(self, termination: Termination<E>) {
        let _ = self.0.update(|model| {
            let Some(input) = model.inputs.at_mut(TupleIndex::<INDEX>) else {
                return UpdateOutcome::empty()
                    .without_events()
                    .with_drop_outside((Some(termination), None));
            };
            match termination {
                Termination::Completed => {
                    input.completed = true;
                    let (released, finished) = model.settle::<P>();
                    if finished {
                        UpdateOutcome::empty().with_termination_event(Termination::Completed)
                    } else {
                        UpdateOutcome::empty().without_events()
                    }
                    .with_drop_outside((None, Some(released)))
                }
                error @ Termination::Error(_) => UpdateOutcome::empty()
                    .with_termination_event(error)
                    .without_drop_outside(),
            }
        });
    }
}

/// Subscribes the source at `INDEX`, unless the stream is over or no plan waits on it any more.
impl<const INDEX: usize, OE, M, T, E, OR, Q, D, P> SourceSubscriber<TupleIndex<INDEX>, OE>
    for Context<M, E, OR, Q, P>
where
    M: ThreadMode,
    OR: Observer<P::Output, E>,
    Q: Inputs + AtPosition<TupleIndex<INDEX>, Element = Option<Input<T, D>>>,
    D: Disposable,
    P: PlansOver<Q>,
    OE: ObservableTypes<Item = T, Error = E, Disposal = D>,
{
    type Observer = WhenObserver<INDEX, M, E, OR, Q, P>;

    fn still_running(&self) -> ControlFlow<Infallible> {
        // Not asked: the sources are a tuple. `observer` looks at the stream for each of them.
        ControlFlow::Continue(())
    }

    fn observer(&mut self, _: TupleIndex<INDEX>) -> Option<Self::Observer> {
        let wanted = self
            .update(|model| UpdateOutcome::new(model.inputs.at(TupleIndex::<INDEX>).is_some()))
            .result()?;
        wanted.then(|| WhenObserver(self.clone()))
    }

    fn store(
        &mut self,
        _: TupleIndex<INDEX>,
        subscription: DisposeOnDrop<D>,
    ) -> ControlFlow<Infallible> {
        let _ = self.update(|model| {
            // Dropped when no plan waits on the source any more, whatever it did while being
            // subscribed. Otherwise it fills the slot, still empty as the source is subscribed once.
            let rejected = match model.inputs.at_mut(TupleIndex::<INDEX>) {
                Some(input) => input.subscription.replace(subscription),
                None => Some(subscription),
            };
            UpdateOutcome::empty()
                .without_events()
                .with_drop_outside(rejected)
        });
        ControlFlow::Continue(())
    }
}

impl<S, P> ObservableTypes for When<S, P>
where
    S: Sources,
    P: Plans,
{
    type Item = P::Output;
    type Error = S::Error;
    type Mode = S::Mode;
    type Disposal = Disposal<S::Mode, P::Matched, S::Error, S::Inputs>;
}

impl<S, P, OR> Observable<OR> for When<S, P>
where
    S: Sources + SourceOps,
    <S as SourceSet>::Cursor: SubscribeEach<
        Context<<S as SourceSet>::Mode, <S as SourceSet>::Error, OR, <S as Sources>::Inputs, P>,
    >,
    <S as Sources>::Inputs: Inputs,
    P: PlansOver<<S as Sources>::Inputs>,
    OR: Observer<P::Output, S::Error>,
{
    fn subscribe(self, observer: OR) -> DisposeOnDrop<Self::Disposal> {
        subscribe_with_context_with_stop_view(
            |stop_view| SelectObserver {
                observer,
                plans: self.plans,
                stop_view,
            },
            Model::new::<P>(),
            |mut context| {
                // The context never breaks: every source is handed to it.
                let ControlFlow::Continue(()) =
                    self.sources.into_cursor().subscribe_each(&mut context);
                DisposeOnDrop::new(())
            },
        )
        .map_inner_into()
    }
}

/// [`Sources`] for a tuple of sources, called by [`for_each_arity!`] once per arity.
macro_rules! impl_sources {
    (
        $(($position:tt, $source:ident, $item:ident, $($_names:tt)*)),+
    ) => {
        impl<E, $($source, $item),+> Sources for ($($source,)+)
        where
            $($source: ObservableTypes<Item = $item, Error = E>,)+
        {
            type Handles = ($(SourceHandle<$position, $item>,)+);
            type Inputs = ($(Option<Input<$item, <$source as ObservableTypes>::Disposal>>,)+);

            fn handles() -> Self::Handles {
                ($(SourceHandle::<$position, _>::new(),)+)
            }
        }
    };
}

for_each_arity!(impl_sources);

/// [`Inputs`] for a tuple of inputs, the [`Sources::Inputs`] of as many sources, called by
/// [`for_each_arity!`] once per arity.
macro_rules! impl_inputs {
    ($(($position:tt, $item:ident, $disposal:ident, $($_names:tt)*)),+) => {
        // `needed` and `release_unused` hold a set of sources in a `u8`, bit `i` for the source at
        // `i`.
        const _: () = assert!($($position < u8::BITS)&&+, "a set of sources is a `u8` of bits");

        impl<$($item, $disposal: Disposable),+> Inputs for ($(Option<Input<$item, $disposal>>,)+) {
            fn new() -> Self {
                ($(Some(Input::<$item, $disposal>::default()),)+)
            }

            fn release_unused(&mut self, needed: u8) -> Self::Reversed {
                ($(
                    if needed & (1 << $position) == 0 {
                        self.$position.take()
                    } else {
                        None
                    },
                )+)
                    .into_reversed()
            }
        }
    };
}

for_each_arity!(impl_inputs);

/// [`Plans`] and [`PlansOver`] for the [`Plan`] of a pattern of as many sources as the arity,
/// called by [`for_each_arity!`] once per arity: a pattern names each source once, so it has no
/// more sources than the [`When`]. The first arity is the plan of a single source, which
/// [`SourceHandle::then`] makes.
macro_rules! impl_plan {
    (
        $(($position:tt, $index:ident, $item:ident, $disposal:ident)),+
    ) => {
        impl<$(const $index: usize, $item,)+ R, F> Plans
            for Plan<($(SourceHandle<$index, $item>,)+), F>
        where
            F: FnMut($($item),+) -> R,
        {
            type Matched = ($($item,)+);
            type Output = R;

            fn select(&mut self, matched: Self::Matched) -> R {
                (self.selector)($(matched.$position),+)
            }
        }

        impl<Q, $(const $index: usize, $item, $disposal: Disposable,)+ R, F> PlansOver<Q>
            for Plan<($(SourceHandle<$index, $item>,)+), F>
        where
            F: FnMut($($item),+) -> R,
            $(Q: AtPosition<TupleIndex<$index>, Element = Option<Input<$item, $disposal>>>,)+
        {
            fn take(inputs: &mut Q) -> Option<Self::Matched> {
                if !($(Input::has_item(inputs.at(TupleIndex::<$index>)))&&+) {
                    return None;
                }
                // Every slot was checked above, and a pattern names each source once.
                Some(($(Input::pop_item(inputs.at_mut(TupleIndex::<$index>)),)+))
            }

            fn needed(inputs: &Q) -> u8 {
                if $(Input::is_exhausted(inputs.at(TupleIndex::<$index>)))||+ {
                    0
                } else {
                    $(1 << $index)|+
                }
            }
        }
    };
}

for_each_arity!(impl_plan);

/// The methods of a [`Pattern`] of as many sources as the arity, called by [`for_each_arity!`]
/// once per arity: the [`then`](Pattern::then) that makes its [`Plan`], and the
/// [`and`](Pattern::and) that makes it from the pattern of a source fewer.
///
/// A single source has no pattern: its [`SourceHandle`] makes its plan ([`SourceHandle::then`])
/// and joins it to a second source ([`SourceHandle::and`]), so the first arity generates nothing,
/// and the second no `and`.
macro_rules! impl_pattern {
    ($_source:tt) => {};
    ($($source:tt),+) => {
        impl_pattern!(@then $($source),+);
        impl_pattern!(@and []; $($source),+);
    };
    (@then $(($_position:tt, $index:ident, $item:ident, $($_names:tt)*)),+) => {
        impl<$(const $index: usize, $item),+> Pattern<($(SourceHandle<$index, $item>,)+)> {
            /// The [`Plan`] that passes each match of this pattern, one item of each source in
            /// the order of the pattern, through `selector`.
            pub fn then<R, F>(self, selector: F) -> Plan<($(SourceHandle<$index, $item>,)+), F>
            where
                F: FnMut($($item),+) -> R,
            {
                Plan::new(selector)
            }
        }
    };
    // Moves the sources into the brackets until the last is left: `and` goes on the pattern of
    // the others, and adds a source of any position, not that one in particular. With one other,
    // there is no such pattern: `SourceHandle::and` is that `and`.
    (@and [$_first:tt]; $_last:tt) => {};
    (@and [$($init:tt)+]; $_last:tt) => {
        impl_pattern!(@and_on $($init),+);
    };
    (@and [$($init:tt)*]; $next:tt, $($rest:tt),+) => {
        impl_pattern!(@and [$($init)* $next]; $($rest),+);
    };
    (@and_on $(($_position:tt, $index:ident, $item:ident, $($_names:tt)*)),+) => {
        impl<$(const $index: usize, $item),+> Pattern<($(SourceHandle<$index, $item>,)+)> {
            /// Adds `other` to the sources of this pattern, which then matches once each has an
            /// item. A pattern names each source once, as [`SourceHandle::and`] says.
            pub fn and<const OTHER: usize, U>(
                self,
                _other: SourceHandle<OTHER, U>,
            ) -> Pattern<($(SourceHandle<$index, $item>,)+ SourceHandle<OTHER, U>)> {
                const { assert!($(OTHER != $index)&&+, "a pattern names each source once") };
                Pattern::new()
            }
        }
    };
}

for_each_arity!(impl_pattern);

/// [`Plans`] and [`PlansOver`] for a tuple of plans, called by [`for_each_arity!`] once per arity.
/// The recursive arms nest a [`ControlFlow`] per plan but the last — `Break` with what the first
/// plan took, `Continue` with what the plans after it hand — then take and select through it in
/// the same order, the first plan first.
macro_rules! impl_plan_tuple {
    ($(($position:tt, $plan:ident, $($_names:tt)*)),+) => {
        impl<$($plan),+> sealed::Sealed for ($($plan,)+) {}

        impl<R, $($plan),+> Plans for ($($plan,)+)
        where
            $($plan: Plans<Output = R>,)+
        {
            type Output = R;
            type Matched = impl_plan_tuple!(@matched $($plan),+);

            fn select(&mut self, matched: Self::Matched) -> R {
                impl_plan_tuple!(@select self, matched; $($position),+)
            }
        }

        impl<Q, R, $($plan),+> PlansOver<Q> for ($($plan,)+)
        where
            $($plan: PlansOver<Q, Output = R>,)+
        {
            fn take(inputs: &mut Q) -> Option<Self::Matched> {
                impl_plan_tuple!(@take inputs, Q; $($plan),+)
            }

            fn needed(inputs: &Q) -> u8 {
                $(<$plan as PlansOver<Q>>::needed(inputs))|+
            }
        }
    };
    (@matched $last:ident) => {
        <$last as Plans>::Matched
    };
    (@matched $first:ident, $($rest:ident),+) => {
        ControlFlow<<$first as Plans>::Matched, impl_plan_tuple!(@matched $($rest),+)>
    };
    (@select $self:ident, $matched:ident; $last:tt) => {
        $self.$last.select($matched)
    };
    (@select $self:ident, $matched:ident; $first:tt, $($rest:tt),+) => {
        match $matched {
            ControlFlow::Break(args) => $self.$first.select(args),
            ControlFlow::Continue(rest) => impl_plan_tuple!(@select $self, rest; $($rest),+),
        }
    };
    (@take $inputs:ident, $q:ident; $last:ident) => {
        <$last as PlansOver<$q>>::take($inputs)
    };
    (@take $inputs:ident, $q:ident; $first:ident, $($rest:ident),+) => {
        match <$first as PlansOver<$q>>::take($inputs) {
            Some(args) => Some(ControlFlow::Break(args)),
            None => impl_plan_tuple!(@take $inputs, $q; $($rest),+).map(ControlFlow::Continue),
        }
    };
}

for_each_arity!(impl_plan_tuple);
