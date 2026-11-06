//! The [`When`] operator and the [`Pattern`]s and [`Plan`]s it is built from: ReactiveX's
//! `And` / `Then` / `When`.

use crate::{
    delegate_disposal,
    disposable::{Disposable, dispose_on_drop::DisposeOnDrop},
    observable::{Observable, ObservableTypes},
    observer::{Flow, Observer, Termination},
    thread_mode::{Joined, ThreadMode},
    utils::{
        MarkerType,
        serialized_delivery::{StopView, UpdateOutcome, Updated},
        subscribe_with_context::{
            self, SubscriptionContext, subscribe_with_context_with_stop_view,
        },
    },
};
use educe::Educe;
use std::{collections::VecDeque, marker::PhantomData};

/// Combines several observables through join patterns: each [`Plan`] waits until every source of
/// its [`Pattern`] has an item, takes the first item of each, and emits what its selector makes of
/// them. Plans may share a source, and an item goes to the first plan, in the order given, that it
/// completes.
/// See <https://reactivex.io/documentation/operators/and-then-when.html>
///
/// The sources are a tuple of one to six observables with one error type. `build` receives a
/// tuple of [`SourceHandle`]s, one per source and in the same order, which
/// [`and`](SourceHandle::and) joins into a pattern and [`then`](Pattern::then) turns into a plan.
/// It returns one plan, or a tuple of up to six plans with one output type.
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
        other: SourceHandle<OTHER, U>,
    ) -> Pattern<(Self, SourceHandle<OTHER, U>)> {
        Pattern::<(Self,)>::new().and(other)
    }

    /// A [`Plan`] of this source alone, which passes each of its items through `selector`.
    pub fn then<R, F>(self, selector: F) -> Plan<(Self,), F>
    where
        F: FnMut(T) -> R,
    {
        Pattern::<(Self,)>::new().then(selector)
    }
}

/// The sources a [`Plan`] waits on, all of them at once: made by [`SourceHandle::and`], extended
/// by [`and`](Pattern::and) up to six sources, and turned into a plan by
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

/// The sources of a [`When`]: a tuple of one to six observables with one error type.
pub trait Sources {
    /// The tuple of the [`SourceHandle`]s of the sources, in their order.
    type Handles;

    #[doc(hidden)]
    fn handles() -> Self::Handles;
}

/// The plans of a [`When`]: a [`Plan`], or a tuple of one to six plans with one output type.
pub trait Plans {
    /// What the selectors return: the items of the [`When`].
    type Output;

    /// What a match hands from under the lock to have its selector run outside it: the items it
    /// took, tagged with the plan that matched for a tuple.
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
pub(crate) trait PlansOver<Q>: Plans {
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
#[doc(hidden)]
pub trait Inputs {
    /// What [`release_unused`](Self::release_unused) takes out, to drop outside the lock: a tuple
    /// of the last source first, which is the order it drops them in, as a disposal does.
    type Released;

    /// The inputs of a new subscription, none released.
    fn new() -> Self;

    /// Releases every input not released yet that is not `needed` any more: bit `i` keeps the
    /// source at `i`.
    fn release_unused(&mut self, needed: u8) -> Self::Released;
}

/// The [`Input`] of the source at `INDEX`.
pub(crate) trait InputAt<const INDEX: usize>: Inputs {
    type Item;
    type Disposal: Disposable;

    fn input(&mut self) -> &mut Option<Input<Self::Item, Self::Disposal>>;

    fn input_ref(&self) -> &Option<Input<Self::Item, Self::Disposal>>;
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
}

delegate_disposal!(
    Disposal<M, MT, E, Q>,
    subscribe_with_context::Disposal<M, MT, E, Model<Q>, ()>,
    where M: ThreadMode, Q: Inputs
);

struct Model<Q: Inputs> {
    inputs: Q,
}

impl<Q: Inputs> Model<Q> {
    /// The model of a new subscription, whose sources that no plan names are released already,
    /// never to be subscribed.
    fn new<P: PlansOver<Q>>() -> Self {
        let mut model = Self { inputs: Q::new() };
        // Nothing is queued or subscribed yet, so the release has nothing to drop, and every plan
        // waits on some source, so none has retired.
        let _ = model.settle::<P>();
        model
    }

    /// Releases the sources that no plan still able to match waits on. Returns what to drop
    /// outside the lock, and whether every plan has retired.
    fn settle<P: PlansOver<Q>>(&mut self) -> (Q::Released, bool) {
        let needed = P::needed(&self.inputs);
        (self.inputs.release_unused(needed), needed == 0)
    }
}

impl<Q: Inputs> Drop for Model<Q> {
    fn drop(&mut self) {
        // Disposes what is still subscribed, the last source first, as a release does: a plain
        // drop of the tuple would dispose the first source first.
        drop(self.inputs.release_unused(0));
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
pub struct WhenObserver<const INDEX: usize, M: ThreadMode, E, OR, Q: Inputs, P: Plans>(
    Context<M, E, OR, Q, P>,
);

impl<const INDEX: usize, M, T, E, OR, Q, P> Observer<T, E> for WhenObserver<INDEX, M, E, OR, Q, P>
where
    M: ThreadMode,
    OR: Observer<P::Output, E>,
    Q: InputAt<INDEX, Item = T>,
    P: PlansOver<Q>,
{
    fn on_next(&mut self, value: T) -> Flow {
        self.0
            .update(|model| {
                let Some(input) = <Q as InputAt<INDEX>>::input(&mut model.inputs) else {
                    return UpdateOutcome::empty()
                        .without_events()
                        .with_drop_outside((Some(value), None))
                        .answering_stop();
                };
                input.items.push_back(value);
                let Some(matched) = P::take(&mut model.inputs) else {
                    // Nothing taken, so no plan retires: a plan retires on a source completed with
                    // an empty queue, and queueing an item empties no queue.
                    return UpdateOutcome::empty()
                        .without_events()
                        .without_drop_outside();
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
                if <Q as InputAt<INDEX>>::input_ref(&model.inputs).is_none() {
                    outcome.answering_stop()
                } else {
                    outcome
                }
            })
            .flow()
    }

    fn on_termination(self, termination: Termination<E>) {
        let _ = self.0.update(|model| {
            let Some(input) = <Q as InputAt<INDEX>>::input(&mut model.inputs) else {
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
fn subscribe_input<const INDEX: usize, OE, M, E, OR, Q, P>(
    context: &Context<M, E, OR, Q, P>,
    source: OE,
) where
    M: ThreadMode,
    OR: Observer<P::Output, E>,
    Q: InputAt<INDEX>,
    P: PlansOver<Q>,
    OE: Observable<
            WhenObserver<INDEX, M, E, OR, Q, P>,
            Item = <Q as InputAt<INDEX>>::Item,
            Error = E,
            Disposal = <Q as InputAt<INDEX>>::Disposal,
        >,
{
    // An update that queues nothing answers `Stop` once a termination is queued, so it tells that
    // the stream is over as well.
    let Updated::Ran {
        result: true,
        flow: Flow::Continue,
    } = context.update(|model| {
        UpdateOutcome::new(<Q as InputAt<INDEX>>::input_ref(&model.inputs).is_some())
    })
    else {
        return;
    };
    let subscription = source.subscribe(WhenObserver(context.clone()));
    let _ = context.update(|model| {
        // Dropped when no plan waits on the source any more, whatever it did while being
        // subscribed. Otherwise the slot is still empty, as the source is subscribed once.
        let rejected = match <Q as InputAt<INDEX>>::input(&mut model.inputs) {
            Some(input) => {
                input.subscription = Some(subscription);
                None
            }
            None => Some(subscription),
        };
        UpdateOutcome::empty()
            .without_events()
            .with_drop_outside(rejected)
    });
}

/// The thread mode of the sources, joined.
macro_rules! joined_mode {
    ($source:ident) => { <$source as ObservableTypes>::Mode };
    ($source:ident, $($rest:ident),+) => {
        Joined<<$source as ObservableTypes>::Mode, joined_mode!($($rest),+)>
    };
}

/// [`Sources`], [`Inputs`], [`InputAt`], [`ObservableTypes`] and [`Observable`] for a tuple of
/// sources.
macro_rules! impl_when {
    (
        $(($position:tt, $source:ident, $item:ident, $disposal:ident)),+;
        reversed: $(($reversed:tt, $reversed_item:ident, $reversed_disposal:ident)),+
    ) => {
        impl_when!(
            @impl
            [joined_mode!($($source),+)]
            [($(Option<Input<$item, <$source as ObservableTypes>::Disposal>>,)+)]
            [$(($item, $disposal)),+]
            $(($position, $source, $item, $disposal)),+;
            reversed: $(($reversed, $reversed_item, $reversed_disposal)),+
        );
    };
    (
        @impl [$mode:ty] [$inputs:ty] $all:tt
        $(($position:tt, $source:ident, $item:ident, $disposal:ident)),+;
        reversed: $(($reversed:tt, $reversed_item:ident, $reversed_disposal:ident)),+
    ) => {
        impl<E, $($source),+> Sources for ($($source,)+)
        where
            $($source: ObservableTypes<Error = E>,)+
        {
            type Handles = ($(SourceHandle<$position, <$source as ObservableTypes>::Item>,)+);

            fn handles() -> Self::Handles {
                ($(SourceHandle::<$position, _>::new(),)+)
            }
        }

        impl<$($item, $disposal: Disposable),+> Inputs for ($(Option<Input<$item, $disposal>>,)+) {
            type Released = ($(Option<Input<$reversed_item, $reversed_disposal>>,)+);

            fn new() -> Self {
                ($(Some(Input::<$item, $disposal>::default()),)+)
            }

            fn release_unused(&mut self, needed: u8) -> Self::Released {
                ($(
                    if needed & (1 << $reversed) == 0 {
                        self.$reversed.take()
                    } else {
                        None
                    },
                )+)
            }
        }

        $(impl_when!(@input_at $all $position, $item, $disposal);)+

        impl<E, P, $($source, $item),+> ObservableTypes for When<($($source,)+), P>
        where
            $($source: ObservableTypes<Item = $item, Error = E>,)+
            P: Plans,
        {
            type Item = P::Output;
            type Error = E;
            type Mode = $mode;
            type Disposal = Disposal<$mode, P::Matched, E, $inputs>;
        }

        impl<E, P, OR, $($source, $item),+> Observable<OR> for When<($($source,)+), P>
        where
            OR: Observer<<P as Plans>::Output, E>,
            P: PlansOver<$inputs>,
            $(
                $source: Observable<
                    WhenObserver<$position, $mode, E, OR, $inputs, P>,
                    Item = $item,
                    Error = E,
                >,
            )+
        {
            fn subscribe(self, observer: OR) -> DisposeOnDrop<Self::Disposal> {
                let sources = self.sources;
                subscribe_with_context_with_stop_view(
                    |stop_view| SelectObserver {
                        observer,
                        plans: self.plans,
                        stop_view,
                    },
                    Model::new::<P>(),
                    |context| {
                        $(subscribe_input::<$position, _, _, _, _, _, _>(&context, sources.$position);)+
                        DisposeOnDrop::new(())
                    },
                )
                .map_inner_into()
            }
        }
    };
    (
        @input_at [$(($all_item:ident, $all_disposal:ident)),+]
        $position:tt, $item:ident, $disposal:ident
    ) => {
        impl<$($all_item, $all_disposal: Disposable),+> InputAt<$position>
            for ($(Option<Input<$all_item, $all_disposal>>,)+)
        {
            type Item = $item;
            type Disposal = $disposal;

            fn input(&mut self) -> &mut Option<Input<$item, $disposal>> {
                &mut self.$position
            }

            fn input_ref(&self) -> &Option<Input<$item, $disposal>> {
                &self.$position
            }
        }
    };
}

impl_when!((0, OE0, T0, D0); reversed: (0, T0, D0));
impl_when!((0, OE0, T0, D0), (1, OE1, T1, D1); reversed: (1, T1, D1), (0, T0, D0));
impl_when!(
    (0, OE0, T0, D0), (1, OE1, T1, D1), (2, OE2, T2, D2);
    reversed: (2, T2, D2), (1, T1, D1), (0, T0, D0)
);
impl_when!(
    (0, OE0, T0, D0), (1, OE1, T1, D1), (2, OE2, T2, D2), (3, OE3, T3, D3);
    reversed: (3, T3, D3), (2, T2, D2), (1, T1, D1), (0, T0, D0)
);
impl_when!(
    (0, OE0, T0, D0), (1, OE1, T1, D1), (2, OE2, T2, D2), (3, OE3, T3, D3),
    (4, OE4, T4, D4);
    reversed: (4, T4, D4), (3, T3, D3), (2, T2, D2), (1, T1, D1), (0, T0, D0)
);
impl_when!(
    (0, OE0, T0, D0), (1, OE1, T1, D1), (2, OE2, T2, D2), (3, OE3, T3, D3),
    (4, OE4, T4, D4), (5, OE5, T5, D5);
    reversed: (5, T5, D5), (4, T4, D4), (3, T3, D3), (2, T2, D2), (1, T1, D1), (0, T0, D0)
);

/// [`Plans`] and [`PlansOver`] for a plan of as many sources, and the [`Pattern`] of as many:
/// its [`then`](Pattern::then), and its [`and`](Pattern::and) below six sources. A single source
/// delegates to the same pattern operations through [`SourceHandle`]; no user can make a pattern
/// of one source, so the attributes given, `#[doc(hidden)]`, keep its methods out of the docs.
macro_rules! impl_plan {
    ($(#[$attr:meta])* pattern: $($source:tt),+) => {
        impl_plan!($(#[$attr])* largest pattern: $($source),+);
        impl_plan!($(#[$attr])* @and $($source),+);
    };
    (
        $(#[$attr:meta])*
        largest pattern: $(($index:ident, $item:ident, $value:ident)),+
    ) => {
        impl<$(const $index: usize, $item,)+ R, F> Plans
            for Plan<($(SourceHandle<$index, $item>,)+), F>
        where
            F: FnMut($($item),+) -> R,
        {
            type Matched = ($($item,)+);
            type Output = R;

            fn select(&mut self, ($($value,)+): Self::Matched) -> R {
                (self.selector)($($value),+)
            }
        }

        impl<Q, $(const $index: usize, $item,)+ R, F> PlansOver<Q>
            for Plan<($(SourceHandle<$index, $item>,)+), F>
        where
            F: FnMut($($item),+) -> R,
            Q: Inputs $(+ InputAt<$index, Item = $item>)+,
        {
            fn take(inputs: &mut Q) -> Option<Self::Matched> {
                if !($(Input::has_item(<Q as InputAt<$index>>::input_ref(inputs)))&&+) {
                    return None;
                }
                Some(($(<Q as InputAt<$index>>::input(inputs).as_mut()?.items.pop_front()?,)+))
            }

            fn needed(inputs: &Q) -> u8 {
                if $(Input::is_exhausted(<Q as InputAt<$index>>::input_ref(inputs)))||+ {
                    0
                } else {
                    $(1 << $index)|+
                }
            }
        }

        $(#[$attr])*
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
    ($(#[$attr:meta])* @and $(($index:ident, $item:ident, $value:ident)),+) => {
        $(#[$attr])*
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

impl_plan!(#[doc(hidden)] pattern: (I0, T0, v0));
impl_plan!(pattern: (I0, T0, v0), (I1, T1, v1));
impl_plan!(pattern: (I0, T0, v0), (I1, T1, v1), (I2, T2, v2));
impl_plan!(pattern: (I0, T0, v0), (I1, T1, v1), (I2, T2, v2), (I3, T3, v3));
impl_plan!(pattern: (I0, T0, v0), (I1, T1, v1), (I2, T2, v2), (I3, T3, v3), (I4, T4, v4));
impl_plan!(
    largest pattern: (I0, T0, v0), (I1, T1, v1), (I2, T2, v2), (I3, T3, v3), (I4, T4, v4),
    (I5, T5, v5)
);

/// [`Plans`] and [`PlansOver`] for a tuple of as many plans.
macro_rules! impl_plans {
    ($matched:ident; $(($plan:ident, $position:tt, $variant:ident)),+) => {
        #[doc(hidden)]
        pub enum $matched<$($plan),+> {
            $($variant($plan),)+
        }

        impl<R, $($plan),+> Plans for ($($plan,)+)
        where
            $($plan: Plans<Output = R>,)+
        {
            type Output = R;
            type Matched = $matched<$(<$plan as Plans>::Matched),+>;

            fn select(&mut self, matched: Self::Matched) -> R {
                match matched {
                    $($matched::$variant(args) => self.$position.select(args),)+
                }
            }
        }

        impl<Q, R, $($plan),+> PlansOver<Q> for ($($plan,)+)
        where
            $($plan: PlansOver<Q, Output = R>,)+
        {
            fn take(inputs: &mut Q) -> Option<Self::Matched> {
                $(
                    if let Some(args) = <$plan as PlansOver<Q>>::take(inputs) {
                        return Some($matched::$variant(args));
                    }
                )+
                None
            }

            fn needed(inputs: &Q) -> u8 {
                $(<$plan as PlansOver<Q>>::needed(inputs))|+
            }
        }
    };
}

impl_plans!(Matched1; (P0, 0, Plan0));
impl_plans!(Matched2; (P0, 0, Plan0), (P1, 1, Plan1));
impl_plans!(Matched3; (P0, 0, Plan0), (P1, 1, Plan1), (P2, 2, Plan2));
impl_plans!(Matched4; (P0, 0, Plan0), (P1, 1, Plan1), (P2, 2, Plan2), (P3, 3, Plan3));
impl_plans!(Matched5;
    (P0, 0, Plan0), (P1, 1, Plan1), (P2, 2, Plan2), (P3, 3, Plan3), (P4, 4, Plan4)
);
impl_plans!(Matched6;
    (P0, 0, Plan0), (P1, 1, Plan1), (P2, 2, Plan2), (P3, 3, Plan3), (P4, 4, Plan4),
    (P5, 5, Plan5)
);
