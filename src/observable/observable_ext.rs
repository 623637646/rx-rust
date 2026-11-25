#[cfg(feature = "futures")]
use super::stream::{ObservableStream, ObservableTryStream, StreamBuffer};
use super::{
    BoxedObservableOf, BoxedObserverOf, CloneableBoxedObservableOf, Observable, ObservableTypes,
    boxed_observable::{BoxedObservableMode, IntoBoxedObservable, IntoCloneableBoxedObservable},
    future::{ObservableFuture, ObservableTryFuture},
};
use crate::{
    disposable::{Disposable, dispose_on_drop::DisposeOnDrop},
    observer::{
        Flow, IntoFlow, Observer, Termination, boxed_observer::ObserverMode,
        callback_observer::CallbackObserver, emitter::Emitter,
    },
    operators::{
        combining::{
            combine_latest::CombineLatest, concat::Concat, concat_all::ConcatAll,
            concat_eager::ConcatEager, concat_eager_all::ConcatEagerAll, exhaust_all::ExhaustAll,
            fork_join::ForkJoin, group_join::GroupJoin, join::Join, merge::Merge,
            merge_all::MergeAll, start_with::StartWith, switch::Switch,
            with_latest_from::WithLatestFrom, zip::Zip,
        },
        conditional_boolean::{
            all::All, amb::Amb, contains::Contains, default_if_empty::DefaultIfEmpty,
            sequence_equal::SequenceEqual, skip_until::SkipUntil, skip_while::SkipWhile,
            switch_if_empty::SwitchIfEmpty, take_until::TakeUntil, take_while::TakeWhile,
        },
        connectable::{connectable_controller::ConnectableController, ref_count::RefCount},
        error_handling::{
            catch::Catch,
            map_err::MapErr,
            retry::{Retry, RetryAction},
        },
        filtering::{
            audit::Audit, debounce::Debounce, distinct::Distinct,
            distinct_until_changed::DistinctUntilChanged, element_at::ElementAt, filter::Filter,
            filter_map::FilterMap, first::First, ignore_elements::IgnoreElements, last::Last,
            sample::Sample, single::Single, skip::Skip, skip_last::SkipLast, take::Take,
            take_last::TakeLast, throttle::Throttle, throttle_latest::ThrottleLatest,
        },
        mathematical_aggregate::{
            average::Average, collect::Collect, count::Count, max::Max, min::Min, reduce::Reduce,
            sum::Sum,
        },
        others::{
            debug::{Debug, DebugEvent, DebugPrintCallback},
            hook_on_next::HookOnNext,
            hook_on_subscription::HookOnSubscription,
            hook_on_termination::HookOnTermination,
            into_shared::IntoShared,
            with_error_type::WithErrorType,
            with_item_type::WithItemType,
        },
        transforming::{
            buffer::Buffer, buffer_with_count::BufferWithCount,
            buffer_with_count_and_skip::BufferWithCountAndSkip, buffer_with_time::BufferWithTime,
            buffer_with_time_and_shift::BufferWithTimeAndShift,
            buffer_with_time_or_count::BufferWithTimeOrCount, concat_eager_map::ConcatEagerMap,
            concat_map::ConcatMap, exhaust_map::ExhaustMap, flat_map::FlatMap, group_by::GroupBy,
            map::Map, pairwise::Pairwise, scan::Scan, switch_map::SwitchMap, window::Window,
            window_with_count::WindowWithCount, window_with_count_and_skip::WindowWithCountAndSkip,
            window_with_time::WindowWithTime, window_with_time_and_shift::WindowWithTimeAndShift,
            window_with_time_or_count::WindowWithTimeOrCount,
        },
        utility::{
            delay::Delay, delay_subscription::DelaySubscription, dematerialize::Dematerialize,
            do_after_disposal::DoAfterDisposal, do_after_next::DoAfterNext,
            do_after_subscription::DoAfterSubscription, do_after_termination::DoAfterTermination,
            do_before_disposal::DoBeforeDisposal, do_before_next::DoBeforeNext,
            do_before_subscription::DoBeforeSubscription,
            do_before_termination::DoBeforeTermination, do_finally::DoFinally, hold::Hold,
            materialize::Materialize, observe_on::ObserveOn, repeat_source::RepeatSource,
            stop_on_disposal::StopOnDisposal, subscribe_on::SubscribeOn,
            time_interval::TimeInterval, timeout::Timeout, timestamp::Timestamp,
        },
    },
    subject::{
        async_subject::{AsyncSubject, AsyncSubjectOf},
        behavior_subject::{BehaviorSubject, BehaviorSubjectOf},
        publish_subject::{PublishSubject, PublishSubjectOf},
        replay_subject::{ReplaySubject, ReplaySubjectOf},
    },
};
use std::{fmt::Display, num::NonZeroUsize, time::Duration};

/// The operators, as methods on every [`Observable`].
///
/// Each method builds the operator of the same name over `self`; the operator's own documentation
/// in [`operators`](crate::operators) describes its behavior in detail and has an example. See the
/// [module documentation](crate::observable) for a pipeline, and
/// [finding an operator](crate::operators#finding-an-operator) to look one up by what it does.
pub trait ObservableExt: ObservableTypes + Sized {
    /// Emits a single `bool` indicating whether every item satisfies the provided predicate.
    #[doc(alias("every", "forall", "is_empty"))]
    fn all<F>(self, callback: F) -> All<Self::Item, Self, F>
    where
        F: FnMut(Self::Item) -> bool,
    {
        All::new(self, callback)
    }

    /// Races two observables and mirrors whichever one emits an item or terminates first.
    #[doc(alias("amb_with", "race", "race_with"))]
    fn amb<OE2>(self, source_2: OE2) -> Amb<(Self, OE2)>
    where
        OE2: ObservableTypes<Item = Self::Item, Error = Self::Error>,
    {
        Amb::new((self, source_2))
    }

    /// Emits the most recent item once `time_span` has passed since the item that started the
    /// span: a trailing-edge throttle.
    #[doc(alias = "audit_time")]
    fn audit<S>(self, time_span: Duration, scheduler: S) -> Audit<Self, S> {
        Audit::new(self, time_span, scheduler)
    }

    /// Calculates the arithmetic mean of all numeric items emitted by the source.
    fn average(self) -> Average<Self::Item, Self> {
        Average::new(self)
    }

    /// Collects the items emitted by the source into buffers delimited by another observable.
    /// Terminating the boundary terminates the outer observable: completing it emits the pending
    /// buffer when non-empty and then completes, while an error from it discards the pending
    /// buffer and errors. Unlike [`window`](ObservableExt::window), a completed boundary does not
    /// leave the current buffer open.
    #[doc(alias = "tumbling_buffer")]
    fn buffer<OE1>(self, boundary: OE1) -> Buffer<Self, OE1>
    where
        OE1: ObservableTypes<Error = Self::Error>,
    {
        Buffer::new(self, boundary)
    }

    /// Collects items into fixed-size buffers and emits each buffer as soon as it fills up.
    #[doc(alias("buffer_count", "chunks", "partition_all", "tumbling_buffer"))]
    fn buffer_with_count(self, count: NonZeroUsize) -> BufferWithCount<Self> {
        BufferWithCount::new(self, count)
    }

    /// Gathers items into bundles of `count`, starting a new bundle every `skip` items: overlapping
    /// bundles for a `skip` below `count`, gaps for one above.
    #[doc(alias("buffer_count", "sliding_buffer"))]
    fn buffer_with_count_and_skip(
        self,
        count: NonZeroUsize,
        skip: NonZeroUsize,
    ) -> BufferWithCountAndSkip<Self>
    where
        Self::Item: Clone,
    {
        BufferWithCountAndSkip::new(self, count, skip)
    }

    /// Collects items into a buffer emitted every `time_span`, the first one `time_span` after the
    /// subscription, on the provided scheduler.
    #[doc(alias("buffer_time", "tumbling_buffer"))]
    fn buffer_with_time<S>(self, time_span: Duration, scheduler: S) -> BufferWithTime<Self, S> {
        BufferWithTime::new(self, time_span, scheduler)
    }

    /// Opens a bundle at subscription and every `time_shift` thereafter on the provided
    /// scheduler. Each lasts `time_span`. A shorter shift creates overlap. A longer shift
    /// leaves gaps.
    ///
    /// See [`BufferWithTimeAndShift`] for completion and error behavior.
    ///
    /// # Panics
    ///
    /// Subscribing panics if `time_span` or `time_shift` is zero.
    #[doc(alias("buffer_time", "sliding_buffer"))]
    fn buffer_with_time_and_shift<S>(
        self,
        time_span: Duration,
        time_shift: Duration,
        scheduler: S,
    ) -> BufferWithTimeAndShift<Self, S>
    where
        Self::Item: Clone,
    {
        BufferWithTimeAndShift::new(self, time_span, time_shift, scheduler)
    }

    /// Collects items into buffers emitted when they reach `count` items or every `time_span`,
    /// whichever comes first; the timer first fires one `time_span` after the subscription.
    #[doc(alias("buffer_time", "buffer_timeout", "chunks_timeout", "tumbling_buffer"))]
    fn buffer_with_time_or_count<S>(
        self,
        count: NonZeroUsize,
        time_span: Duration,
        scheduler: S,
    ) -> BufferWithTimeOrCount<Self, S> {
        BufferWithTimeOrCount::new(self, count, time_span, scheduler)
    }

    /// Recovers from errors by switching to another observable yielded by the callback.
    #[doc(alias(
        "catch_error",
        "catch_exception",
        "on_error_resume_next",
        "on_error_return",
        "on_error_return_item",
        "rescue_error"
    ))]
    fn catch<OE1, F>(self, callback: F) -> Catch<Self::Error, Self, F>
    where
        OE1: ObservableTypes<Item = Self::Item>,
        F: FnOnce(Self::Error) -> OE1,
    {
        Catch::new(self, callback)
    }

    /// Gathers all the items into a collection built with `Default` and `Extend`, and emits it
    /// when the source completes. See [`to_vec`](ObservableExt::to_vec) for the `Vec<T>` case.
    #[doc(alias("collect_into", "to_dict", "to_dictionary", "to_map", "to_set"))]
    fn collect<C>(self) -> Collect<C, Self::Item, Self> {
        Collect::new(self)
    }

    /// Combines the latest values from both observables whenever either produces a new item.
    #[doc(alias = "combine_latest_with")]
    fn combine_latest<OE2>(self, source_2: OE2) -> CombineLatest<(Self, OE2)>
    where
        OE2: ObservableTypes<Error = Self::Error>,
    {
        CombineLatest::new((self, source_2))
    }

    /// Concatenates the source with another observable, waiting for the first to complete.
    #[doc(alias("chain", "concat_with", "end_with"))]
    fn concat<OE2>(self, source_2: OE2) -> Concat<(Self, OE2)>
    where
        OE2: ObservableTypes<Item = Self::Item, Error = Self::Error>,
    {
        Concat::new((self, source_2))
    }

    /// Flattens an observable-of-observables by concatenating each inner observable sequentially.
    #[doc(alias = "flatten")]
    fn concat_all(self) -> ConcatAll<Self, Self::Item>
    where
        Self::Item: ObservableTypes<Error = Self::Error>,
    {
        ConcatAll::new(self)
    }

    /// Concatenates the source with another observable, subscribing both at once and keeping the
    /// values of the other until the source has completed.
    /// [`ConcatEager::max_concurrency`] bounds how many are subscribed at a time.
    fn concat_eager<OE2>(self, source_2: OE2) -> ConcatEager<(Self, OE2)>
    where
        OE2: ObservableTypes<Item = Self::Item, Error = Self::Error>,
    {
        ConcatEager::new((self, source_2))
    }

    /// Flattens an observable-of-observables by subscribing each inner observable as it arrives,
    /// and emitting their values one inner observable after the other.
    /// [`ConcatEagerAll::max_concurrency`] bounds how many are subscribed at a time.
    #[doc(alias = "buffered")]
    fn concat_eager_all(self) -> ConcatEagerAll<Self, Self::Item>
    where
        Self::Item: ObservableTypes<Error = Self::Error>,
    {
        ConcatEagerAll::new(self)
    }

    /// Maps each item to an observable, subscribes it at once, and emits the values of the
    /// resulting inner sequences one after the other.
    /// [`ConcatEagerMap::max_concurrency`] bounds how many are subscribed at a time.
    #[doc(alias("concat_map_eager", "flat_map_sequential"))]
    fn concat_eager_map<OE1, F>(self, callback: F) -> ConcatEagerMap<Self::Item, Self, OE1, F>
    where
        OE1: ObservableTypes<Error = Self::Error>,
        F: FnMut(Self::Item) -> OE1,
    {
        ConcatEagerMap::new(self, callback)
    }

    /// Maps each item to an observable and concatenates the resulting inner sequences.
    #[doc(alias("concat_map_iterable", "map_cat", "select_concat"))]
    fn concat_map<OE1, F>(self, callback: F) -> ConcatMap<Self::Item, Self, OE1, F>
    where
        OE1: ObservableTypes<Error = Self::Error>,
        F: FnMut(Self::Item) -> OE1,
    {
        ConcatMap::new(self, callback)
    }

    /// Emits `true` if the sequence contains the provided item, `false` otherwise.
    fn contains(self, item: Self::Item) -> Contains<Self::Item, Self> {
        Contains::new(self, item)
    }

    /// Counts the number of items emitted and emits that count as a single value.
    #[doc(alias("length", "long_count", "size"))]
    fn count(self) -> Count<Self::Item, Self> {
        Count::new(self)
    }

    /// Emits an item only once `time_span` has passed without the source emitting another one.
    #[doc(alias("debounce_time", "throttle_with_timeout"))]
    fn debounce<S>(self, time_span: Duration, scheduler: S) -> Debounce<Self, S> {
        Debounce::new(self, time_span, scheduler)
    }

    /// Reports every event of the stream — subscription, values, termination, disposal — to
    /// `callback`, together with `context`.
    #[doc(alias = "log")]
    fn debug<C, F>(self, context: C, callback: F) -> Debug<Self, C, F>
    where
        F: Fn(C, DebugEvent<'_, Self::Item, Self::Error>),
    {
        Debug::new(self, context, callback)
    }

    /// [`debug`](ObservableExt::debug) that prints every event with `println!`, prefixed by
    /// `label`.
    #[doc(alias("log", "print"))]
    fn debug_to_stdout<L>(
        self,
        label: L,
    ) -> Debug<Self, L, DebugPrintCallback<L, Self::Item, Self::Error>>
    where
        L: Display,
        Self::Item: std::fmt::Debug,
        Self::Error: std::fmt::Debug,
    {
        Debug::with_stdout(self, label)
    }

    /// Emits a default value if the source completes without emitting any items.
    #[doc(alias = "if_empty")]
    fn default_if_empty(self, default_value: Self::Item) -> DefaultIfEmpty<Self::Item, Self> {
        DefaultIfEmpty::new(self, default_value)
    }

    /// Offsets the emission of items by the specified duration using the given scheduler.
    fn delay<S>(self, delay: Duration, scheduler: S) -> Delay<Self, S> {
        Delay::new(self, delay, scheduler)
    }

    /// Subscribes to the source only once `delay` has passed on the provided scheduler.
    fn delay_subscription<S>(self, delay: Duration, scheduler: S) -> DelaySubscription<Self, S> {
        DelaySubscription::new(self, delay, scheduler)
    }

    /// Turns a stream of [`Event`](crate::observer::Event)s back into the events they describe.
    fn dematerialize(self) -> Dematerialize<Self> {
        Dematerialize::new(self)
    }

    /// Filters out duplicate items, keeping only the first occurrence of each value.
    #[doc(alias = "unique")]
    #[allow(clippy::type_complexity)]
    fn distinct(self) -> Distinct<Self, fn(&Self::Item) -> Self::Item>
    where
        Self::Item: Clone,
    {
        Distinct::new(self)
    }

    /// Suppresses consecutive duplicate items, comparing the values directly.
    #[doc(alias = "dedup")]
    #[allow(clippy::type_complexity)]
    fn distinct_until_changed(self) -> DistinctUntilChanged<Self, fn(&Self::Item) -> Self::Item>
    where
        Self::Item: Clone,
    {
        DistinctUntilChanged::new(self)
    }

    /// Suppresses consecutive duplicate items using a custom key selector.
    #[doc(alias("dedup_by_key", "distinct_until_key_changed"))]
    fn distinct_until_changed_with_key_selector<F, K>(
        self,
        key_selector: F,
    ) -> DistinctUntilChanged<Self, F>
    where
        F: FnMut(&Self::Item) -> K,
    {
        DistinctUntilChanged::new_with_key_selector(self, key_selector)
    }

    /// Filters out duplicates based on a key selector, keeping only unique keys.
    #[doc(alias("distinct_key", "unique_by"))]
    fn distinct_with_key_selector<F, K>(self, key_selector: F) -> Distinct<Self, F>
    where
        F: FnMut(&Self::Item) -> K,
    {
        Distinct::new_with_key_selector(self, key_selector)
    }

    /// Invokes a callback after the downstream subscription is disposed.
    fn do_after_disposal<F>(self, callback: F) -> DoAfterDisposal<Self, F>
    where
        F: FnOnce(),
    {
        DoAfterDisposal::new(self, callback)
    }

    /// Invokes a callback after each item is forwarded downstream.
    fn do_after_next<F>(self, callback: F) -> DoAfterNext<Self, F>
    where
        F: FnMut(Self::Item),
    {
        DoAfterNext::new(self, callback)
    }

    /// Invokes a callback after the observer subscribes to the source.
    fn do_after_subscription<F>(self, callback: F) -> DoAfterSubscription<Self, F>
    where
        F: FnOnce(),
    {
        DoAfterSubscription::new(self, callback)
    }

    /// Invokes a callback after the source terminates, regardless of completion or error.
    #[doc(alias = "do_after_terminate")]
    fn do_after_termination<F>(self, callback: F) -> DoAfterTermination<Self, F>
    where
        F: FnOnce(Termination<Self::Error>),
    {
        DoAfterTermination::new(self, callback)
    }

    /// Invokes a callback right before the downstream subscription is disposed.
    #[doc(alias("do_on_cancel", "do_on_dispose", "do_on_unsubscribe"))]
    fn do_before_disposal<F>(self, callback: F) -> DoBeforeDisposal<Self, F>
    where
        F: FnOnce(),
    {
        DoBeforeDisposal::new(self, callback)
    }

    /// Invokes a callback with a reference to each item before it is sent downstream.
    #[doc(alias("do_action", "do_on_next", "inspect", "tap", "tap_on_next"))]
    fn do_before_next<F>(self, callback: F) -> DoBeforeNext<Self, F>
    where
        F: FnMut(&Self::Item),
    {
        DoBeforeNext::new(self, callback)
    }

    /// Invokes a callback just before the observer subscribes to the source.
    #[doc(alias = "do_on_subscribe")]
    fn do_before_subscription<F>(self, callback: F) -> DoBeforeSubscription<Self, F>
    where
        F: FnOnce(),
    {
        DoBeforeSubscription::new(self, callback)
    }

    /// Invokes a callback before the stream terminates, receiving the termination reason.
    #[doc(alias(
        "do_on_complete",
        "do_on_completed",
        "do_on_error",
        "do_on_terminate",
        "inspect_err",
        "tap_on_completed",
        "tap_on_error"
    ))]
    fn do_before_termination<F>(self, callback: F) -> DoBeforeTermination<Self, F>
    where
        F: FnOnce(&Termination<Self::Error>),
    {
        DoBeforeTermination::new(self, callback)
    }

    /// Invokes a callback once the stream is over: terminated, disposed, or let go of by its
    /// source, whichever comes first.
    #[doc(alias("finalize", "finally", "finally_action", "finally_do"))]
    fn do_finally<F>(self, callback: F) -> DoFinally<Self, F>
    where
        F: FnOnce(),
    {
        DoFinally::new(self, callback)
    }

    /// Emits only the item at the given zero-based index and then completes.
    #[doc(alias = "nth")]
    fn element_at(self, index: usize) -> ElementAt<Self> {
        ElementAt::new(self, index)
    }

    /// Flattens an observable of observables, ignoring the inner observables that arrive while
    /// one is still running.
    #[doc(alias("exclusive", "exhaust"))]
    fn exhaust_all(self) -> ExhaustAll<Self, Self::Item>
    where
        Self::Item: ObservableTypes<Error = Self::Error>,
    {
        ExhaustAll::new(self)
    }

    /// Maps each item to an observable, ignoring the items that arrive while the previous one is
    /// still running.
    #[doc(alias("flat_map_first", "select_switch_first"))]
    fn exhaust_map<OE1, F>(self, callback: F) -> ExhaustMap<Self::Item, Self, OE1, F>
    where
        OE1: ObservableTypes<Error = Self::Error>,
        F: FnMut(Self::Item) -> OE1,
    {
        ExhaustMap::new(self, callback)
    }

    /// Filters items using a predicate, forwarding only values that return `true`.
    #[doc(alias("keep", "select", "where"))]
    fn filter<F>(self, callback: F) -> Filter<Self, F>
    where
        F: FnMut(&Self::Item) -> bool,
    {
        Filter::new(self, callback)
    }

    /// Maps each item to an `Option`, forwarding only the values that are `Some`.
    fn filter_map<T1, F>(self, callback: F) -> FilterMap<Self::Item, Self, F>
    where
        F: FnMut(Self::Item) -> Option<T1>,
    {
        FilterMap::new(self, callback)
    }

    /// Emits only the first item from the source, then completes; nothing if there was none.
    #[doc(alias("first_element", "head"))]
    fn first(self) -> First<Self> {
        First::new(self)
    }

    /// Maps each item to an observable and merges the resulting inner sequences concurrently.
    /// [`FlatMap::max_concurrency`] bounds how many are subscribed at a time.
    #[doc(alias(
        "flat_map_iterable",
        "flat_map_unordered",
        "flat_map_with_max_concurrent",
        "merge_map",
        "select_many",
        "select_with_max_concurrent"
    ))]
    fn flat_map<OE1, F>(self, callback: F) -> FlatMap<Self::Item, Self, OE1, F>
    where
        OE1: ObservableTypes<Error = Self::Error>,
        F: FnMut(Self::Item) -> OE1,
    {
        FlatMap::new(self, callback)
    }

    /// Waits for both observables to complete, then emits the last item of each as a pair and
    /// completes.
    #[doc(alias = "join_all")]
    fn fork_join<OE2>(self, source_2: OE2) -> ForkJoin<(Self, OE2)>
    where
        OE2: ObservableTypes<Error = Self::Error>,
    {
        ForkJoin::new((self, source_2))
    }

    /// Groups items by key into multiple observable sequences.
    #[doc(alias = "partition")]
    fn group_by<'a, F, K>(self, key_selector: F) -> GroupBy<'a, Self, F, K>
    where
        Self::Mode: ObserverMode,
        F: FnMut(&Self::Item) -> K,
    {
        GroupBy::new(self, key_selector)
    }

    /// Pairs every item with a window of the items of `right` that overlap it, each item being
    /// open until the observable its duration selector returns for it emits or completes.
    fn group_join<'a, OE2, FL, FR, DL, DR>(
        self,
        right: OE2,
        left_duration_selector: FL,
        right_duration_selector: FR,
    ) -> GroupJoin<'a, Self, OE2, FL, FR, DL, DR>
    where
        OE2: ObservableTypes<Error = Self::Error>,
        FL: FnMut(&Self::Item) -> DL,
        FR: FnMut(&OE2::Item) -> DR,
        DL: ObservableTypes<Error = Self::Error>,
        DR: ObservableTypes<Error = Self::Error>,
    {
        GroupJoin::new(self, right, left_duration_selector, right_duration_selector)
    }

    /// Holds `resource` for as long as the subscription lasts, and drops it when the subscription
    /// ends; see [`Hold`].
    fn hold<R>(self, resource: R) -> Hold<Self, R> {
        Hold::new(self, resource)
    }

    /// Hands each item, together with the downstream observer, to a callback that decides what to
    /// forward.
    ///
    /// The callback returns the [`Flow`] the operator answers, which is normally the one the
    /// downstream observer it was handed answered.
    fn hook_on_next<F>(self, callback: F) -> HookOnNext<Self, F>
    where
        F: FnMut(&mut dyn Observer<Self::Item, Self::Error>, Self::Item) -> Flow,
    {
        HookOnNext::new(self, callback)
    }

    /// Hands each subscription to a callback, which gets the source and the downstream observer
    /// and subscribes them itself.
    ///
    /// The callback gets the downstream observer unboxed, so the operator subscribes that one
    /// observer type only; see [`HookOnSubscription`] and
    /// [`hook_on_subscription_boxed`](ObservableExt::hook_on_subscription_boxed).
    fn hook_on_subscription<OR, D, F>(self, callback: F) -> HookOnSubscription<Self, F, D>
    where
        OR: Observer<Self::Item, Self::Error>,
        D: Disposable,
        F: FnOnce(Self, Emitter<Self::Mode, OR>) -> DisposeOnDrop<D>,
    {
        HookOnSubscription::new(self, callback)
    }

    /// Like [`hook_on_subscription`](ObservableExt::hook_on_subscription), but the callback gets
    /// the boxed observer of the source's mode, so the operator subscribes any observer.
    fn hook_on_subscription_boxed<'a, D, F>(
        self,
        callback: F,
    ) -> HookOnSubscription<Self, F, D, true>
    where
        D: Disposable,
        Self::Mode: ObserverMode,
        F: FnOnce(Self, BoxedObserverOf<'a, Self>) -> DisposeOnDrop<D>,
    {
        HookOnSubscription::new_boxed(self, callback)
    }

    /// Hands the termination, together with the downstream observer, to a callback that decides
    /// what to deliver.
    ///
    /// The callback gets the downstream observer unboxed, so the operator subscribes that one
    /// observer type only; see [`HookOnTermination`] and
    /// [`hook_on_termination_boxed`](ObservableExt::hook_on_termination_boxed).
    fn hook_on_termination<OR, F>(self, callback: F) -> HookOnTermination<Self, F>
    where
        OR: Observer<Self::Item, Self::Error>,
        F: FnOnce(Emitter<Self::Mode, OR>, Termination<Self::Error>),
    {
        HookOnTermination::new(self, callback)
    }

    /// Like [`hook_on_termination`](ObservableExt::hook_on_termination), but the callback gets the
    /// boxed observer of the source's mode, so the operator subscribes any observer.
    fn hook_on_termination_boxed<'a, F>(self, callback: F) -> HookOnTermination<Self, F, true>
    where
        Self::Mode: ObserverMode,
        F: FnOnce(BoxedObserverOf<'a, Self>, Termination<Self::Error>),
    {
        HookOnTermination::new_boxed(self, callback)
    }

    /// Ignores all items from the source, only relaying termination events.
    fn ignore_elements(self) -> IgnoreElements<Self> {
        IgnoreElements::new(self)
    }

    /// Erases the observable's concrete type into the box its mode picks: a
    /// [`BoxedObservable`](super::boxed_observable::BoxedObservable) for a `Local` source, a
    /// [`SendBoxedObservable`](super::boxed_observable::SendBoxedObservable) for a `Shared` one,
    /// which must then be `Send`, and so must its disposal and observers. See
    /// [`boxed_observable`](super::boxed_observable) for the box of the other flavor; the lifetimes
    /// bound the observer (`'or`), the disposal (`'sub`) and the observable itself (`'oe`).
    ///
    /// The erased observable keeps what it borrows, so it can borrow from the stack, and it cannot
    /// outlive what it borrows:
    ///
    /// ```rust
    /// use rx_rust::{observable::ObservableExt, operators::creating::just::Just};
    ///
    /// let offset = 41;
    /// let mut result = 0;
    /// let source = Just::new(1).map(|value| value + offset).into_boxed();
    /// source.subscribe_with_callback(|value| result = value, |_| {});
    /// assert_eq!(result, 42);
    /// ```
    ///
    /// ```compile_fail
    /// use rx_rust::{
    ///     observable::{ObservableExt, boxed_observable::BoxedObservable},
    ///     operators::creating::just::Just,
    ///     thread_mode::Local,
    /// };
    /// use std::convert::Infallible;
    ///
    /// fn dangling() -> BoxedObservable<'static, 'static, 'static, i32, Infallible, Local> {
    ///     let offset = 41;
    ///     let offset = &offset;
    ///     Just::new(1).map(move |value| value + *offset).into_boxed()
    /// }
    /// ```
    ///
    /// Owning what it uses instead, it can be returned:
    ///
    /// ```rust
    /// use rx_rust::{
    ///     observable::{ObservableExt, boxed_observable::BoxedObservable},
    ///     operators::creating::just::Just,
    ///     thread_mode::Local,
    /// };
    /// use std::convert::Infallible;
    ///
    /// fn owned() -> BoxedObservable<'static, 'static, 'static, i32, Infallible, Local> {
    ///     let offset = 41;
    ///     Just::new(1).map(move |value| value + offset).into_boxed()
    /// }
    /// ```
    ///
    /// A `Shared` source gets the `Send` box, so its observers must be `Send`:
    ///
    /// ```compile_fail
    /// use rx_rust::{observable::ObservableExt, operators::creating::just::Just};
    /// use std::{cell::RefCell, rc::Rc};
    ///
    /// let values = Rc::new(RefCell::new(Vec::new()));
    /// let _subscription = Just::new(1)
    ///     .into_shared()
    ///     .into_boxed()
    ///     .subscribe_with_callback(move |value| values.borrow_mut().push(value), |_| {});
    /// ```
    ///
    /// A `Local` one gets the plain box, which takes any observer:
    ///
    /// ```rust
    /// use rx_rust::{observable::ObservableExt, operators::creating::just::Just};
    /// use std::{cell::RefCell, rc::Rc};
    ///
    /// let values = Rc::new(RefCell::new(Vec::new()));
    /// let _subscription = Just::new(1)
    ///     .into_boxed()
    ///     .subscribe_with_callback(move |value| values.borrow_mut().push(value), |_| {});
    /// ```
    #[doc(alias("as_observable", "boxed", "erase_to_any_publisher"))]
    fn into_boxed<'or, 'sub, 'oe>(self) -> BoxedObservableOf<'or, 'sub, 'oe, Self>
    where
        Self: IntoBoxedObservable<'or, 'sub, 'oe, <Self as ObservableTypes>::Mode>,
        <Self as ObservableTypes>::Mode: BoxedObservableMode,
    {
        IntoBoxedObservable::box_observable(self)
    }

    /// Erases the observable's concrete type and makes it cloneable, into the box its mode picks:
    /// a [`CloneableBoxedObservable`](super::boxed_observable::CloneableBoxedObservable) for a
    /// `Local` source, a
    /// [`SendCloneableBoxedObservable`](super::boxed_observable::SendCloneableBoxedObservable) for a
    /// `Shared` one, which must then be `Send` and `Sync`. The source must be `Clone`, as it is
    /// cloned on every subscription; see [`into_boxed`](Self::into_boxed) for the rest.
    fn into_cloneable_boxed<'or, 'sub, 'oe>(
        self,
    ) -> CloneableBoxedObservableOf<'or, 'sub, 'oe, Self>
    where
        Self: IntoCloneableBoxedObservable<'or, 'sub, 'oe, <Self as ObservableTypes>::Mode>,
        <Self as ObservableTypes>::Mode: BoxedObservableMode,
    {
        IntoCloneableBoxedObservable::box_cloneable_observable(self)
    }

    /// Converts the observable into a future of its first item: `Some(item)`, or `None` when the
    /// source completes without one. The source is stopped as soon as the item is in.
    ///
    /// This is only for a source that cannot fail; a fallible one goes through
    /// [`into_try_future`](Self::into_try_future). An operator that picks another item, such as
    /// `last`, or one that always emits, such as `collect`, goes in front of it.
    ///
    /// A source that never sends an item or a completion keeps the future pending forever; see
    /// [`ObservableTryFuture`] for how to bound the wait.
    #[doc(alias("first_value_from", "to_future"))]
    fn into_future(self) -> ObservableFuture<Self>
    where
        Self: ObservableTypes<Error = std::convert::Infallible>,
    {
        ObservableFuture::new(self)
    }

    /// Declares the observable [`Shared`](crate::thread_mode::Shared), so that a `Local` source
    /// can be erased into the same type as a `Shared` one.
    fn into_shared(self) -> IntoShared<Self> {
        IntoShared::new(self)
    }

    /// Converts the observable into an async stream.
    ///
    /// A `Stream` has no error channel, so this is only for a source that cannot fail; a
    /// fallible one goes through [`into_try_stream`](Self::into_try_stream).
    ///
    /// The items that arrive between two polls are all kept, so a source faster than the
    /// consumer grows the buffer without bound;
    /// [`into_stream_with_buffer`](Self::into_stream_with_buffer) takes a buffer of your choice.
    ///
    /// A source that never completes keeps the stream pending after its last item; see
    /// [`ObservableTryStream`] for how to bound the wait.
    #[doc(alias = "to_stream")]
    #[cfg(feature = "futures")]
    fn into_stream(self) -> ObservableStream<Self::Item, Self>
    where
        Self: ObservableTypes<Error = std::convert::Infallible>,
    {
        ObservableStream::new(self)
    }

    /// Converts the observable into an async stream that keeps the items arriving between two
    /// polls in `buffer`, which decides what a source faster than the consumer costs.
    ///
    /// [`Latest`](crate::observable::stream::Latest) keeps only the newest
    /// item, [`Bounded`](crate::observable::stream::Bounded) a fixed number
    /// of them and [`Unbounded`](crate::observable::stream::Unbounded) —
    /// what [`into_stream`](Self::into_stream) uses — everything; a [`StreamBuffer`] of your own
    /// can fold them instead. Whatever the buffer, the source is never slowed down: a `Stream`
    /// only pulls from the buffer, not from the source.
    ///
    /// # Examples
    /// ```rust
    /// use futures::{FutureExt, StreamExt};
    /// use rx_rust::{
    ///     observable::{ObservableExt, stream::Latest},
    ///     observer::Observer,
    ///     subject::publish_subject::PublishSubject,
    /// };
    ///
    /// let mut subject = PublishSubject::local();
    /// let mut stream = subject.clone().into_stream_with_buffer(Latest::new());
    /// assert_eq!(stream.next().now_or_never(), None); // subscribes
    ///
    /// assert!(subject.on_next(1).is_continue());
    /// assert!(subject.on_next(2).is_continue());
    /// assert!(subject.on_next(3).is_continue());
    /// assert_eq!(stream.next().now_or_never(), Some(Some(3)));
    /// assert_eq!(stream.next().now_or_never(), None);
    /// ```
    #[cfg(feature = "futures")]
    fn into_stream_with_buffer<B>(self, buffer: B) -> ObservableStream<Self::Item, Self, B>
    where
        Self: ObservableTypes<Error = std::convert::Infallible>,
        B: StreamBuffer<Self::Item>,
    {
        ObservableStream::with_buffer(self, buffer)
    }

    /// Converts the observable into a future of its first item: `Ok(Some(item))`, `Ok(None)` when
    /// the source completes without one, or `Err(error)` when it fails first. The source is
    /// stopped as soon as the item is in.
    ///
    /// The output is the `Maybe` of ReactiveX; an operator that always emits, such as `collect`,
    /// in front of it makes it a `Single`, and `last` picks the last item instead of the first.
    ///
    /// A source that never sends an item or a termination keeps the future pending forever; see
    /// [`ObservableTryFuture`] for how to bound the wait.
    #[doc(alias("first_value_from", "to_future"))]
    fn into_try_future(self) -> ObservableTryFuture<Self> {
        ObservableTryFuture::new(self)
    }

    /// Converts the observable into an async stream of `Result`s: each item as `Ok`, and an error
    /// as the last item, `Err`, before the stream ends.
    ///
    /// The items that arrive between two polls are all kept, so a source faster than the
    /// consumer grows the buffer without bound;
    /// [`into_try_stream_with_buffer`](Self::into_try_stream_with_buffer) takes a buffer of your
    /// choice.
    ///
    /// A source that never terminates keeps the stream pending after its last item; see
    /// [`ObservableTryStream`] for how to bound the wait.
    #[doc(alias = "to_stream")]
    #[cfg(feature = "futures")]
    fn into_try_stream(self) -> ObservableTryStream<Self::Item, Self::Error, Self> {
        ObservableTryStream::new(self)
    }

    /// Converts the observable into an async stream of `Result`s that keeps the items arriving
    /// between two polls in `buffer`. This is
    /// [`into_stream_with_buffer`](Self::into_stream_with_buffer) for a source that can fail;
    /// see there for the buffers.
    #[cfg(feature = "futures")]
    fn into_try_stream_with_buffer<B>(
        self,
        buffer: B,
    ) -> ObservableTryStream<Self::Item, Self::Error, Self, B>
    where
        B: StreamBuffer<Self::Item>,
    {
        ObservableTryStream::with_buffer(self, buffer)
    }

    /// Combines every two items, of this observable and of `right`, that are open at the same
    /// time, each item being open until the observable its duration selector returns for it emits
    /// or completes.
    fn join<OE2, FL, FR, F, DL, DR, T>(
        self,
        right: OE2,
        left_duration_selector: FL,
        right_duration_selector: FR,
        result_selector: F,
    ) -> Join<Self, OE2, FL, FR, F, DL, DR, T>
    where
        OE2: ObservableTypes<Error = Self::Error>,
        FL: FnMut(&Self::Item) -> DL,
        FR: FnMut(&OE2::Item) -> DR,
        DL: ObservableTypes<Error = Self::Error>,
        DR: ObservableTypes<Error = Self::Error>,
        F: FnMut(&Self::Item, &OE2::Item) -> T,
    {
        Join::new(
            self,
            right,
            left_duration_selector,
            right_duration_selector,
            result_selector,
        )
    }

    /// Emits only the last item of the source, on completion; nothing if there was none.
    #[doc(alias = "last_element")]
    fn last(self) -> Last<Self> {
        Last::new(self)
    }

    /// Transforms each item by applying a user-supplied mapping function.
    #[doc(alias("map_to", "pluck", "select"))]
    fn map<T1, F>(self, callback: F) -> Map<Self::Item, Self, F>
    where
        F: FnMut(Self::Item) -> T1,
    {
        Map::new(self, callback)
    }

    /// Transforms an error emitted by the source while leaving its items unchanged.
    #[doc(alias("map_error", "on_error_map"))]
    fn map_err<E1, F>(self, callback: F) -> MapErr<Self::Error, Self, F>
    where
        F: FnOnce(Self::Error) -> E1,
    {
        MapErr::new(self, callback)
    }

    /// Turns every event, the termination included, into an [`Event`](crate::observer::Event)
    /// item, then completes.
    fn materialize(self) -> Materialize<Self> {
        Materialize::new(self)
    }

    /// Emits the maximum item produced by the source according to the natural order.
    fn max(self) -> Max<Self> {
        Max::new(self)
    }

    /// Merges the source with another observable, interleaving both streams concurrently.
    #[doc(alias("merge_with", "select_all"))]
    fn merge<OE2>(self, source_2: OE2) -> Merge<(Self, OE2)>
    where
        OE2: ObservableTypes<Item = Self::Item, Error = Self::Error>,
    {
        Merge::new((self, source_2))
    }

    /// Merges an observable-of-observables by interleaving items from inner streams.
    /// [`MergeAll::max_concurrency`] bounds how many are subscribed at a time.
    #[doc(alias("buffer_unordered", "flatten", "flatten_unordered", "merge_concurrent"))]
    fn merge_all(self) -> MergeAll<Self, Self::Item>
    where
        Self::Item: ObservableTypes<Error = Self::Error>,
    {
        MergeAll::new(self)
    }

    /// Emits the minimum item produced by the source according to the natural order.
    fn min(self) -> Min<Self> {
        Min::new(self)
    }

    /// Converts the source into a connectable observable, multicast through the subjects
    /// `subject_factory` creates from `context`: one for each connection, and a new one whenever
    /// the previous one is reset.
    ///
    /// The factory is a function pointer, a closure that captures nothing: what it needs goes in
    /// `context`, `()` when nothing. It runs under the controller's lock, and so does the
    /// subject's `Clone`: see [`ConnectableController`] for what they must not do.
    ///
    /// ```rust
    /// use rx_rust::{
    ///     observable::ObservableExt, operators::creating::from_iter::FromIter,
    ///     subject::replay_subject::ReplaySubject,
    /// };
    ///
    /// let mut seen = Vec::new();
    /// FromIter::new([1, 2, 3])
    ///     .multicast(Some(1), |buffer_size| ReplaySubject::local(*buffer_size))
    ///     .ref_count()
    ///     .subscribe_with_callback(|value| seen.push(value), |_| {});
    /// assert_eq!(seen, [1, 2, 3]);
    /// ```
    fn multicast<S, C>(
        self,
        context: C,
        subject_factory: fn(&mut C) -> S,
    ) -> ConnectableController<Self, S, C>
    where
        S: ObservableTypes,
    {
        ConnectableController::new(self, context, subject_factory)
    }

    /// Delivers the events downstream from tasks of the provided scheduler.
    #[doc(alias("publish_on", "receive_on"))]
    fn observe_on<S>(self, scheduler: S) -> ObserveOn<Self, S> {
        ObserveOn::new(self, scheduler)
    }

    /// Emits each item paired with the one before it, as `(previous, current)`.
    #[doc(alias = "tuple_windows")]
    fn pairwise(self) -> Pairwise<Self>
    where
        Self::Item: Clone,
    {
        Pairwise::new(self)
    }

    /// Multicasts the source using a `PublishSubject`.
    fn publish<'a>(self) -> ConnectableController<Self, PublishSubjectOf<'a, Self>>
    where
        Self::Mode: ObserverMode,
        Self::Item: Clone,
        Self::Error: Clone,
    {
        self.multicast((), |_| PublishSubject::new())
    }

    /// Multicasts the source using a `BehaviorSubject`: every subscriber receives the current
    /// value — `initial_value` until the source emits — before what follows it.
    ///
    /// Each connection starts over from `initial_value`, which is cloned for it under the
    /// controller's lock, so its `Clone` must not reach back into this controller: see
    /// [`ConnectableController`].
    #[doc(alias = "publish_value")]
    fn publish_behavior<'a>(
        self,
        initial_value: Self::Item,
    ) -> ConnectableController<Self, BehaviorSubjectOf<'a, Self>, Self::Item>
    where
        Self::Mode: ObserverMode,
        Self::Item: Clone,
        Self::Error: Clone,
    {
        self.multicast(initial_value, |initial_value| {
            BehaviorSubject::new(initial_value.clone())
        })
    }

    /// Multicasts the source using an `AsyncSubject`, emitting only the last value.
    fn publish_last<'a>(self) -> ConnectableController<Self, AsyncSubjectOf<'a, Self>>
    where
        Self::Mode: ObserverMode,
        Self::Item: Clone,
        Self::Error: Clone,
    {
        self.multicast((), |_| AsyncSubject::new())
    }

    /// Aggregates the sequence using an initial seed and an accumulator function.
    #[doc(alias("aggregate", "fold", "fold_left"))]
    fn reduce<T0, F>(self, initial_value: T0, callback: F) -> Reduce<T0, Self::Item, Self, F>
    where
        F: FnMut(T0, Self::Item) -> T0,
    {
        Reduce::new(self, initial_value, callback)
    }

    /// Re-subscribes after a completion, to the observable the callback returns, until the
    /// callback returns `None`.
    #[doc(alias(
        "cycle",
        "do_while",
        "repeat",
        "repeat_infinitely",
        "repeat_until",
        "repeat_when"
    ))]
    fn repeat_source<OE1, F>(self, callback: F) -> RepeatSource<Self, F>
    where
        OE1: ObservableTypes<Item = Self::Item, Error = Self::Error>,
        F: FnMut() -> Option<OE1>,
    {
        RepeatSource::new(self, callback)
    }

    /// Multicasts the source using a `ReplaySubject` that keeps the last `buffer_size` values, or
    /// every value for `None`.
    #[doc(alias = "publish_replay")]
    fn replay<'a>(
        self,
        buffer_size: Option<usize>,
    ) -> ConnectableController<Self, ReplaySubjectOf<'a, Self>, Option<usize>>
    where
        Self::Mode: ObserverMode,
        Self::Item: Clone,
        Self::Error: Clone,
    {
        self.multicast(buffer_size, |buffer_size| ReplaySubject::new(*buffer_size))
    }

    /// Re-subscribes to the source based on the retry strategy returned by the callback.
    #[doc(alias("retry_infinitely", "retry_until", "retry_when"))]
    fn retry<OE1, F>(self, callback: F) -> Retry<Self, F>
    where
        OE1: ObservableTypes<Item = Self::Item, Error = Self::Error>,
        F: FnMut(Self::Error) -> RetryAction<Self::Error, OE1>,
    {
        Retry::new(self, callback)
    }

    /// Samples the source whenever the sampler observable emits an event.
    #[doc(alias("sample_time", "throttle_last"))]
    fn sample<OE1>(self, sampler: OE1) -> Sample<Self, OE1>
    where
        OE1: ObservableTypes<Error = Self::Error>,
    {
        Sample::new(self, sampler)
    }

    /// Accumulates values over time, emitting each intermediate result.
    #[doc(alias = "reductions")]
    fn scan<T0, F>(self, initial_value: T0, callback: F) -> Scan<T0, Self::Item, Self, F>
    where
        F: FnMut(T0, Self::Item) -> T0,
    {
        Scan::new(self, initial_value, callback)
    }

    /// Compares two sequences element by element for equality.
    #[doc(alias = "eq")]
    fn sequence_equal<OE2>(self, source_2: OE2) -> SequenceEqual<Self::Item, Self, OE2>
    where
        OE2: ObservableTypes<Item = Self::Item, Error = Self::Error>,
    {
        SequenceEqual::new(self, source_2)
    }

    /// Shares a single subscription to the source using `PublishSubject` semantics.
    ///
    /// The subscription ends, and the next subscriber starts over, when the source terminates or
    /// the last subscriber leaves; `multicast(..).ref_count()` with the `reset_on_*` options of
    /// [`ConnectableController`] and [`RefCount`] chooses otherwise.
    fn share<'a>(self) -> RefCount<Self, PublishSubjectOf<'a, Self>>
    where
        Self::Mode: ObserverMode,
        Self::Item: Clone,
        Self::Error: Clone,
    {
        self.publish().ref_count()
    }

    /// Shares a single subscription using `BehaviorSubject` semantics: every subscriber first
    /// receives the current value, `initial_value` until the source emits.
    ///
    /// Like [`share`](Self::share), it starts over when the source terminates or the last
    /// subscriber leaves, and the current value with it. `initial_value` is cloned for each
    /// connection under the locks of the controller and of the [`RefCount`], so its `Clone` must
    /// not subscribe to this observable nor dispose a subscription of it: see
    /// [`ConnectableController`].
    #[doc(alias = "share_value")]
    fn share_behavior<'a>(
        self,
        initial_value: Self::Item,
    ) -> RefCount<Self, BehaviorSubjectOf<'a, Self>, Self::Item>
    where
        Self::Mode: ObserverMode,
        Self::Item: Clone,
        Self::Error: Clone,
    {
        self.publish_behavior(initial_value).ref_count()
    }

    /// Shares a single subscription using `AsyncSubject` semantics: only the last item, on
    /// completion.
    ///
    /// Like [`share`](Self::share), it starts over when the source terminates or the last
    /// subscriber leaves.
    fn share_last<'a>(self) -> RefCount<Self, AsyncSubjectOf<'a, Self>>
    where
        Self::Mode: ObserverMode,
        Self::Item: Clone,
        Self::Error: Clone,
    {
        self.publish_last().ref_count()
    }

    /// Shares a single subscription using `ReplaySubject` semantics: later subscribers first get
    /// the last `buffer_size` values, or every value for `None`.
    ///
    /// Like [`share`](Self::share), it starts over when the source terminates or the last
    /// subscriber leaves, and the buffer with it.
    fn share_replay<'a>(
        self,
        buffer_size: Option<usize>,
    ) -> RefCount<Self, ReplaySubjectOf<'a, Self>, Option<usize>>
    where
        Self::Mode: ObserverMode,
        Self::Item: Clone,
        Self::Error: Clone,
    {
        self.replay(buffer_size).ref_count()
    }

    /// Emits the only item of the source once it completes; fails if there is none or more than
    /// one.
    #[doc(alias = "exactly_one")]
    fn single(self) -> Single<Self> {
        Single::new(self)
    }

    /// Skips the first `count` items before emitting the remainder of the sequence.
    #[doc(alias("drop", "rest", "tail"))]
    fn skip(self, count: usize) -> Skip<Self> {
        Skip::new(self, count)
    }

    /// Skips the last `count` items emitted by the source.
    #[doc(alias("drop_last", "drop_right"))]
    fn skip_last(self, count: usize) -> SkipLast<Self> {
        SkipLast::new(self, count)
    }

    /// Ignores items from the source until the notifier observable emits; a notifier that
    /// completes without emitting completes the result.
    #[doc(alias("drop_until", "skip_until_with_time", "skip_with_time"))]
    fn skip_until<OE1>(self, start: OE1) -> SkipUntil<Self, OE1>
    where
        OE1: ObservableTypes<Error = Self::Error>,
    {
        SkipUntil::new(self, start)
    }

    /// Skips items while the predicate returns `true`, then emits the remaining items.
    #[doc(alias = "drop_while")]
    fn skip_while<F>(self, callback: F) -> SkipWhile<Self, F>
    where
        F: FnMut(&Self::Item) -> bool,
    {
        SkipWhile::new(self, callback)
    }

    /// Emits the provided values first, then subscribes to the source.
    #[doc(alias("cons", "prepend"))]
    fn start_with<I>(self, values: I) -> StartWith<Self, I>
    where
        I: IntoIterator<Item = Self::Item>,
    {
        StartWith::new(self, values)
    }

    /// Delivers nothing more once the subscription is disposed, and answers [`Flow::Break`] from
    /// then on: for an [`Observer`] that may be disposed from inside one of its notifications
    /// without answering [`Flow::Break`] itself.
    fn stop_on_disposal(self) -> StopOnDisposal<Self> {
        StopOnDisposal::new(self)
    }

    /// Subscribes to the source on the provided scheduler.
    fn subscribe_on<S>(self, scheduler: S) -> SubscribeOn<Self, S> {
        SubscribeOn::new(self, scheduler)
    }

    /// Subscribes with two closures instead of an [`Observer`].
    ///
    /// `on_next` may return nothing, which keeps the source going, or a [`Flow`], which lets it
    /// end its own stream with [`Flow::Break`]: the source then stops pushing — a synchronous one
    /// stops iterating — and drops the callbacks without calling `on_termination`.
    ///
    /// A callback that disposes its own subscription answers [`Flow::Break`] as well: the
    /// disposal alone does not hold back what an operator still has to send for the event it is
    /// handling. [`stop_on_disposal`](Self::stop_on_disposal) does, where the callback cannot
    /// answer.
    #[doc(alias = "for_each")]
    fn subscribe_with_callback<FN, FT, R>(
        self,
        on_next: FN,
        on_termination: FT,
    ) -> DisposeOnDrop<Self::Disposal>
    where
        Self: Observable<CallbackObserver<FN, FT>>,
        FN: FnMut(Self::Item) -> R,
        R: IntoFlow,
        FT: FnOnce(Termination<Self::Error>),
    {
        self.subscribe(CallbackObserver::new(on_next, on_termination))
    }

    /// Sums all numeric items and emits the accumulated total.
    fn sum(self) -> Sum<Self> {
        Sum::new(self)
    }

    /// Switches to the most recent inner observable emitted by the source.
    #[doc(alias("switch_all", "switch_latest", "switch_on_next"))]
    fn switch(self) -> Switch<Self, Self::Item>
    where
        Self::Item: ObservableTypes<Error = Self::Error>,
    {
        Switch::new(self)
    }

    /// Continues with `fallback` if the source completes without emitting any item.
    #[doc(alias = "if_empty")]
    fn switch_if_empty<OE1>(self, fallback: OE1) -> SwitchIfEmpty<Self, OE1>
    where
        OE1: ObservableTypes<Item = Self::Item, Error = Self::Error>,
    {
        SwitchIfEmpty::new(self, fallback)
    }

    /// Maps each item to an observable and switches to the latest inner sequence.
    #[doc(alias("flat_map_latest", "select_switch"))]
    fn switch_map<OE1, F>(self, callback: F) -> SwitchMap<Self::Item, Self, OE1, F>
    where
        OE1: ObservableTypes<Error = Self::Error>,
        F: FnMut(Self::Item) -> OE1,
    {
        SwitchMap::new(self, callback)
    }

    /// Emits only the first `count` items from the source before completing.
    #[doc(alias = "limit")]
    fn take(self, count: usize) -> Take<Self> {
        Take::new(self, count)
    }

    /// Emits only the last `count` items produced by the source.
    #[doc(alias = "take_right")]
    fn take_last(self, count: usize) -> TakeLast<Self> {
        TakeLast::new(self, count)
    }

    /// Relays items until the notifier observable emits, then completes; a notifier that
    /// terminates first terminates the result the same way.
    #[doc(alias("take_until_with_time", "take_with_time"))]
    fn take_until<OE1>(self, stop: OE1) -> TakeUntil<Self, OE1>
    where
        OE1: ObservableTypes<Error = Self::Error>,
    {
        TakeUntil::new(self, stop)
    }

    /// Emits items while the predicate holds `true`, then completes.
    fn take_while<F>(self, callback: F) -> TakeWhile<Self, F>
    where
        F: FnMut(&Self::Item) -> bool,
    {
        TakeWhile::new(self, callback)
    }

    /// Throttles emissions to at most one item per `time_span`, on `clock`.
    ///
    /// Leading-edge: the cooldown is decided by comparing item arrival times, so no timer is
    /// spawned and a [`Clock`](crate::scheduler::Clock) is all it takes: `SystemClock`, or
    /// `scheduler.clock()`.
    #[doc(alias("throttle_first", "throttle_time"))]
    fn throttle<C>(self, time_span: Duration, clock: C) -> Throttle<Self, C> {
        Throttle::new(self, time_span, clock)
    }

    /// Emits an item at once, then, while items keep coming, the most recent one at the end of
    /// each `time_span`: a leading- and trailing-edge throttle.
    #[doc(alias = "throttle_time")]
    fn throttle_latest<S>(self, time_span: Duration, scheduler: S) -> ThrottleLatest<Self, S> {
        ThrottleLatest::new(self, time_span, scheduler)
    }

    /// Pairs each item with the time elapsed since the previous one, or since the subscription for
    /// the first, on `clock`: `SystemClock`, or `scheduler.clock()`.
    #[doc(alias = "elapsed")]
    fn time_interval<C>(self, clock: C) -> TimeInterval<Self, C> {
        TimeInterval::new(self, clock)
    }

    /// Errors if the next item, or the first since the subscription, does not arrive within
    /// `duration`.
    fn timeout<S>(self, duration: Duration, scheduler: S) -> Timeout<Self, S> {
        Timeout::new(self, duration, scheduler)
    }

    /// Pairs each item with the [`Instant`](std::time::Instant) it arrived at, on `clock`:
    /// `SystemClock`, or `scheduler.clock()`.
    fn timestamp<C>(self, clock: C) -> Timestamp<Self, C> {
        Timestamp::new(self, clock)
    }

    /// Gathers all the items into a `Vec` and emits it when the source completes. This is
    /// [`collect`](ObservableExt::collect) specialized to `Vec<T>`, which is the shape that
    /// [`window`](ObservableExt::window) composes with:
    /// `source.window(boundary).concat_map(|window| window.to_vec())`.
    #[doc(alias("to_array", "to_list", "to_vector"))]
    fn to_vec(self) -> Collect<Vec<Self::Item>, Self::Item, Self> {
        Collect::new(self)
    }

    /// Collects items into windows that are opened and closed by another observable.
    /// Completing the boundary stops future window rotation without terminating the source.
    /// An error from the boundary terminates the current window and the outer observable.
    #[doc(alias = "tumbling")]
    fn window<'a, OE1>(self, boundary: OE1) -> Window<'a, Self, OE1>
    where
        Self::Mode: ObserverMode,
        OE1: ObservableTypes<Error = Self::Error>,
    {
        Window::new(self, boundary)
    }

    /// Collects items into windows containing a fixed number of elements.
    #[doc(alias("tumbling", "window_count"))]
    fn window_with_count<'a>(self, count: NonZeroUsize) -> WindowWithCount<'a, Self>
    where
        Self::Mode: ObserverMode,
    {
        WindowWithCount::new(self, count)
    }

    /// Splits the items into windows of `count`, starting a new window every `skip` items:
    /// overlapping windows for a `skip` below `count`, gaps for one above.
    #[doc(alias("sliding", "window_count"))]
    fn window_with_count_and_skip<'a>(
        self,
        count: NonZeroUsize,
        skip: NonZeroUsize,
    ) -> WindowWithCountAndSkip<'a, Self>
    where
        Self::Mode: ObserverMode,
        Self::Item: Clone,
    {
        WindowWithCountAndSkip::new(self, count, skip)
    }

    /// Collects items into windows that each last `time_span`, on the provided scheduler: the
    /// first one opens at the subscription, and each following one as the one before is
    /// completed.
    #[doc(alias("tumbling", "window_time"))]
    fn window_with_time<'a, S>(
        self,
        time_span: Duration,
        scheduler: S,
    ) -> WindowWithTime<'a, Self, S> {
        WindowWithTime::new(self, time_span, scheduler)
    }

    /// Opens a window at subscription and every `time_shift` thereafter on the provided
    /// scheduler. Each lasts `time_span`. A shorter shift creates overlap. A longer shift
    /// leaves gaps.
    ///
    /// See [`WindowWithTimeAndShift`] for how to subscribe to the windows.
    ///
    /// # Panics
    ///
    /// Subscribing panics if `time_span` or `time_shift` is zero.
    #[doc(alias("sliding", "window_time"))]
    fn window_with_time_and_shift<'a, S>(
        self,
        time_span: Duration,
        time_shift: Duration,
        scheduler: S,
    ) -> WindowWithTimeAndShift<'a, Self, S>
    where
        Self::Item: Clone,
    {
        WindowWithTimeAndShift::new(self, time_span, time_shift, scheduler)
    }

    /// Collects items into windows completed when they reach `count` items or every `time_span`,
    /// whichever comes first; the timer first fires one `time_span` after the subscription.
    #[doc(alias("tumbling", "window_time", "window_timeout"))]
    fn window_with_time_or_count<'a, S>(
        self,
        count: NonZeroUsize,
        time_span: Duration,
        scheduler: S,
    ) -> WindowWithTimeOrCount<'a, Self, S> {
        WindowWithTimeOrCount::new(self, count, time_span, scheduler)
    }

    /// Gives an observable whose error type is `Infallible` a concrete error type, so that it
    /// can be joined to a fallible one (`merge`, `concat`, …), which requires the same error type.
    #[doc(alias = "set_failure_type")]
    fn with_error_type<E1>(self) -> WithErrorType<E1, Self> {
        WithErrorType::new(self)
    }

    /// Gives an observable whose item type is `Infallible` a concrete item type, so that it can
    /// be joined to one that emits (`merge`, `concat`, …), which requires the same item type.
    fn with_item_type<T1>(self) -> WithItemType<T1, Self> {
        WithItemType::new(self)
    }

    /// Pairs each item with the latest item of `other`, emitting only when the source emits.
    fn with_latest_from<OE1>(self, other: OE1) -> WithLatestFrom<Self, (OE1,)>
    where
        OE1: ObservableTypes<Error = Self::Error>,
    {
        WithLatestFrom::new(self, (other,))
    }

    /// Pairs items from both observables by index and emits tuples of corresponding values.
    #[doc(alias = "zip_with")]
    fn zip<OE2>(self, source_2: OE2) -> Zip<(Self, OE2)>
    where
        OE2: ObservableTypes<Error = Self::Error>,
    {
        Zip::new((self, source_2))
    }
}

impl<OE: ObservableTypes> ObservableExt for OE {}
