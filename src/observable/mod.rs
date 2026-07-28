pub mod boxed_observable;
pub mod cloneable_boxed_observable;
pub mod either_observable;

#[cfg(feature = "futures")]
use crate::operators::others::observable_stream::ObservableStream;
use crate::{
    disposable::{Disposable, bound_drop_disposal::BoundDropDisposal},
    observable::{
        boxed_observable::BoxedObservable, cloneable_boxed_observable::CloneableBoxedObservable,
        either_observable::EitherObservable,
    },
    observer::{
        Observer, Termination, boxed_observer::BoxedObserver, callback_observer::CallbackObserver,
    },
    operators::{
        backpressure::{
            on_backpressure::{BackpressureCollection, OnBackpressure},
            on_backpressure_buffer::OnBackpressureBuffer,
            on_backpressure_latest::OnBackpressureLatest,
        },
        combining::{
            combine_latest::CombineLatest, concat::Concat, concat_all::ConcatAll, merge::Merge,
            merge_all::MergeAll, start_with::StartWith, switch::Switch, zip::Zip,
        },
        conditional_boolean::{
            all::All, amb::Amb, contains::Contains, default_if_empty::DefaultIfEmpty,
            sequence_equal::SequenceEqual, skip_until::SkipUntil, skip_while::SkipWhile,
            take_until::TakeUntil, take_while::TakeWhile,
        },
        connectable::{connectable_controller::ConnectableController, ref_count::RefCount},
        error_handling::{
            catch::Catch,
            map_err::MapErr,
            retry::{Retry, RetryAction},
        },
        filtering::{
            debounce::Debounce, distinct::Distinct, distinct_until_changed::DistinctUntilChanged,
            element_at::ElementAt, filter::Filter, first::First, ignore_elements::IgnoreElements,
            last::Last, sample::Sample, skip::Skip, skip_last::SkipLast, take::Take,
            take_last::TakeLast, throttle::Throttle,
        },
        mathematical_aggregate::{
            average::Average, count::Count, max::Max, min::Min, reduce::Reduce, sum::Sum,
        },
        others::{
            debug::{Debug, DebugEvent, DefaultPrintType},
            hook_on_next::HookOnNext,
            hook_on_subscription::HookOnSubscription,
            hook_on_termination::HookOnTermination,
            with_error_type::WithErrorType,
            with_item_type::WithItemType,
        },
        transforming::{
            buffer::Buffer, buffer_with_count::BufferWithCount, buffer_with_time::BufferWithTime,
            buffer_with_time_or_count::BufferWithTimeOrCount, concat_map::ConcatMap,
            flat_map::FlatMap, group_by::GroupBy, map::Map, scan::Scan, switch_map::SwitchMap,
            window::Window, window_with_count::WindowWithCount,
        },
        utility::{
            delay::Delay, dematerialize::Dematerialize, do_after_disposal::DoAfterDisposal,
            do_after_next::DoAfterNext, do_after_subscription::DoAfterSubscription,
            do_after_termination::DoAfterTermination, do_before_disposal::DoBeforeDisposal,
            do_before_next::DoBeforeNext, do_before_subscription::DoBeforeSubscription,
            do_before_termination::DoBeforeTermination, materialize::Materialize,
            observe_on::ObserveOn, subscribe_on::SubscribeOn, time_interval::TimeInterval,
            timeout::Timeout, timestamp::Timestamp,
        },
    },
    subject::{
        async_subject::AsyncSubject, publish_subject::PublishSubject, replay_subject::ReplaySubject,
    },
    utils::types::{MaybeSend, MaybeSync},
};
use std::convert::Infallible;
use std::{fmt::Display, num::NonZeroUsize, time::Duration};

pub type Subscription<D> = BoundDropDisposal<D>;

/// The `Observable` trait represents a source of events that can be observed by an `Observer`.
/// See <https://reactivex.io/documentation/observable.html>
pub trait Observable<'or, T, E> {
    type D: Disposable;

    /// Subscribes an observer to this observable. When an observer is subscribed, it will start receiving events from the observable.
    /// The `subscribe` method returns a `Subscription` which can be used to unsubscribe the observer from the observable.
    /// We use `Subscription` struct instead of trait like `impl Cancellable`, because we need to cancel the subscription when the `Subscription` is dropped. It's not possible to implement Drop for a trait object.
    fn subscribe(self, observer: impl Observer<T, E> + MaybeSend + 'or) -> Subscription<Self::D>;
}

/// Extension trait that exposes the full suite of RxRust operators on any type that
/// implements [`Observable`]. Each method forwards to the corresponding operator
/// constructor, allowing a fluent, ergonomic style when composing observable pipelines.
pub trait ObservableExt<'or, T, E>: Observable<'or, T, E> + Sized {
    /// Emits a single `bool` indicating whether every item satisfies the provided predicate.
    fn all<F>(self, callback: F) -> All<T, Self, F>
    where
        F: FnMut(T) -> bool,
    {
        All::new(self, callback)
    }

    /// Competes two observables and mirrors whichever one produces an item or error first.
    fn amb_with<OE1>(self, other: OE1) -> Amb<[EitherObservable<Self, OE1>; 2]>
    where
        OE1: Observable<'or, T, E>,
    {
        Amb::new([EitherObservable::Left(self), EitherObservable::Right(other)])
    }

    /// Calculates the arithmetic mean of all numeric items emitted by the source.
    fn average(self) -> Average<T, Self> {
        Average::new(self)
    }

    /// Collects the items emitted by the source into buffers delimited by another observable.
    fn buffer<OE1>(self, boundary: OE1) -> Buffer<Self, OE1>
    where
        OE1: Observable<'or, (), E>,
    {
        Buffer::new(self, boundary)
    }

    /// Collects items into fixed-size buffers and emits each buffer as soon as it fills up.
    fn buffer_with_count(self, count: NonZeroUsize) -> BufferWithCount<Self> {
        BufferWithCount::new(self, count)
    }

    /// Collects items into time-based buffers driven by the provided scheduler.
    fn buffer_with_time<S>(
        self,
        time_span: Duration,
        scheduler: S,
        delay: Option<Duration>,
    ) -> BufferWithTime<'or, Self, S> {
        BufferWithTime::new(self, time_span, scheduler, delay)
    }

    /// Collects items into buffers using both size and time boundaries whichever occurs first.
    fn buffer_with_time_or_count<S>(
        self,
        count: NonZeroUsize,
        time_span: Duration,
        scheduler: S,
        delay: Option<Duration>,
    ) -> BufferWithTimeOrCount<Self, S> {
        BufferWithTimeOrCount::new(self, count, time_span, scheduler, delay)
    }

    /// Recovers from errors by switching to another observable yielded by the callback.
    fn catch<E1, OE1, F>(self, callback: F) -> Catch<E, Self, F>
    where
        OE1: Observable<'or, T, E1>,
        F: FnOnce(E) -> OE1,
    {
        Catch::new(self, callback)
    }

    /// Combines the latest values from both observables whenever either produces a new item.
    fn combine_latest<T1, OE2>(self, another_source: OE2) -> CombineLatest<Self, OE2>
    where
        OE2: Observable<'or, T1, E>,
    {
        CombineLatest::new(self, another_source)
    }

    /// Flattens an observable-of-observables by concatenating each inner observable sequentially.
    fn concat_all<T1>(self) -> ConcatAll<Self, T>
    where
        T: Observable<'or, T1, E>,
    {
        ConcatAll::new(self)
    }

    /// Maps each item to an observable and concatenates the resulting inner sequences.
    fn concat_map<T1, OE1, F>(self, callback: F) -> ConcatMap<T, Self, OE1, F>
    where
        OE1: Observable<'or, T1, E>,
        F: FnMut(T) -> OE1,
    {
        ConcatMap::new(self, callback)
    }

    /// Concatenates the source with another observable, waiting for the first to complete.
    fn concat_with<OE2>(self, source_2: OE2) -> Concat<Self, OE2>
    where
        OE2: Observable<'or, T, E>,
    {
        Concat::new(self, source_2)
    }

    /// Emits `true` if the sequence contains the provided item, `false` otherwise.
    fn contains(self, item: T) -> Contains<T, Self> {
        Contains::new(self, item)
    }

    /// Counts the number of items emitted and emits that count as a single value.
    fn count(self) -> Count<T, Self> {
        Count::new(self)
    }

    /// Emits an item from the source Observable only after a particular time span has passed without another source emission.
    fn debounce<S>(self, time_span: Duration, scheduler: S) -> Debounce<'or, Self, S> {
        Debounce::new(self, time_span, scheduler)
    }

    /// Attaches a label to the stream and logs lifecycle events for debugging purposes using the provided callback.
    fn debug<C, F>(self, context: C, callback: F) -> Debug<Self, C, F>
    where
        F: Fn(C, DebugEvent<'_, T, E>),
    {
        Debug::new(self, context, callback)
    }

    /// Attaches a label to the stream and logs lifecycle events for debugging purposes using the default print.
    fn debug_default_print<L>(self, label: L) -> Debug<Self, L, DefaultPrintType<L, T, E>>
    where
        L: Display,
        T: std::fmt::Debug,
        E: std::fmt::Debug,
    {
        Debug::new_default_print(self, label)
    }

    /// Emits a default value if the source completes without emitting any items.
    fn default_if_empty(self, default_value: T) -> DefaultIfEmpty<T, Self> {
        DefaultIfEmpty::new(self, default_value)
    }

    /// Offsets the emission of items by the specified duration using the given scheduler.
    fn delay<S>(self, delay: Duration, scheduler: S) -> Delay<'or, Self, S> {
        Delay::new(self, delay, scheduler)
    }

    /// Converts a stream of notifications back into a normal observable sequence.
    fn dematerialize(self) -> Dematerialize<Self> {
        Dematerialize::new(self)
    }

    /// Filters out duplicate items, keeping only the first occurrence of each value.
    fn distinct(self) -> Distinct<Self, fn(&T) -> T>
    where
        T: Clone,
    {
        Distinct::new(self)
    }

    /// Suppresses consecutive duplicate items, comparing the values directly.
    fn distinct_until_changed(self) -> DistinctUntilChanged<Self, fn(&T) -> T>
    where
        T: Clone,
    {
        DistinctUntilChanged::new(self)
    }

    /// Suppresses consecutive duplicate items using a custom key selector.
    fn distinct_until_changed_with_key_selector<F, K>(
        self,
        key_selector: F,
    ) -> DistinctUntilChanged<Self, F>
    where
        F: FnMut(&T) -> K,
    {
        DistinctUntilChanged::new_with_key_selector(self, key_selector)
    }

    /// Filters out duplicates based on a key selector, keeping only unique keys.
    fn distinct_with_key_selector<F, K>(self, key_selector: F) -> Distinct<Self, F>
    where
        F: FnMut(&T) -> K,
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
        F: FnMut(T),
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
    fn do_after_termination<F>(self, callback: F) -> DoAfterTermination<Self, F>
    where
        F: FnOnce(Termination<E>),
    {
        DoAfterTermination::new(self, callback)
    }

    /// Invokes a callback right before the downstream subscription is disposed.
    fn do_before_disposal<F>(self, callback: F) -> DoBeforeDisposal<Self, F>
    where
        F: FnOnce(),
    {
        DoBeforeDisposal::new(self, callback)
    }

    /// Invokes a callback with a reference to each item before it is sent downstream.
    fn do_before_next<F>(self, callback: F) -> DoBeforeNext<Self, F>
    where
        F: FnMut(&T),
    {
        DoBeforeNext::new(self, callback)
    }

    /// Invokes a callback just before the observer subscribes to the source.
    fn do_before_subscription<F>(self, callback: F) -> DoBeforeSubscription<Self, F>
    where
        F: FnOnce(),
    {
        DoBeforeSubscription::new(self, callback)
    }

    /// Invokes a callback before the stream terminates, receiving the termination reason.
    fn do_before_termination<F>(self, callback: F) -> DoBeforeTermination<Self, F>
    where
        F: FnOnce(&Termination<E>),
    {
        DoBeforeTermination::new(self, callback)
    }

    /// Emits only the item at the given zero-based index and then completes.
    fn element_at(self, index: usize) -> ElementAt<Self> {
        ElementAt::new(self, index)
    }

    /// Filters items using a predicate, forwarding only values that return `true`.
    fn filter<F>(self, callback: F) -> Filter<Self, F>
    where
        F: FnMut(&T) -> bool,
    {
        Filter::new(self, callback)
    }

    /// Emits only the first item from the source, then completes.
    fn first(self) -> First<Self> {
        First::new(self)
    }

    /// Maps each item to an observable and merges the resulting inner sequences concurrently.
    fn flat_map<T1, OE1, F>(self, callback: F) -> FlatMap<T, Self, OE1, F>
    where
        OE1: Observable<'or, T1, E>,
        F: FnMut(T) -> OE1,
    {
        FlatMap::new(self, callback)
    }

    /// Groups items by key into multiple observable sequences.
    fn group_by<F, K>(self, callback: F) -> GroupBy<Self, F, K>
    where
        F: FnMut(T) -> K,
    {
        GroupBy::new(self, callback)
    }

    /// Hooks into the emission of items, allowing mutation of the downstream observer.
    fn hook_on_next<F>(self, callback: F) -> HookOnNext<Self, F>
    where
        F: FnMut(&mut dyn Observer<T, E>, T),
    {
        HookOnNext::new(self, callback)
    }

    /// Hooks into subscription, letting you override how the source subscribes observers.
    fn hook_on_subscription<D, F>(self, callback: F) -> HookOnSubscription<Self, F>
    where
        D: Disposable,
        F: FnOnce(Self, BoxedObserver<'or, T, E>) -> Subscription<D>,
    {
        HookOnSubscription::new(self, callback)
    }

    /// Hooks into termination, providing access to the observer and termination payload.
    fn hook_on_termination<F>(self, callback: F) -> HookOnTermination<Self, F>
    where
        F: FnOnce(BoxedObserver<'or, T, E>, Termination<E>),
    {
        HookOnTermination::new(self, callback)
    }

    /// Ignores all items from the source, only relaying termination events.
    fn ignore_elements(self) -> IgnoreElements<Self> {
        IgnoreElements::new(self)
    }

    /// Boxes the observable, erasing its concrete type while preserving lifetime bounds.
    fn into_boxed<'sub, 'oe>(self) -> BoxedObservable<'or, 'sub, 'oe, T, E>
    where
        T: 'or,
        E: 'or,
        Self: MaybeSend + 'oe,
        Self::D: MaybeSend + 'sub,
    {
        BoxedObservable::new(self)
    }

    /// Boxes the observable and makes it cloneable, erasing its concrete type while preserving lifetime bounds.
    fn into_cloneable_boxed<'sub, 'oe>(self) -> CloneableBoxedObservable<'or, 'sub, 'oe, T, E>
    where
        T: 'or,
        E: 'or,
        Self: MaybeSend + MaybeSync + Clone + 'oe,
        Self::D: MaybeSend + 'sub,
    {
        CloneableBoxedObservable::new(self)
    }

    /// Converts the observable into an async stream.
    #[cfg(feature = "futures")]
    fn into_stream(self) -> ObservableStream<'or, T, Self>
    where
        Self: Observable<'or, T, Infallible>,
    {
        ObservableStream::new(self)
    }

    /// Emits only the final item produced by the source before completion.
    fn last(self) -> Last<Self> {
        Last::new(self)
    }

    /// Transforms each item by applying a user-supplied mapping function.
    fn map<T1, F>(self, callback: F) -> Map<T, Self, F>
    where
        F: FnMut(T) -> T1,
    {
        Map::new(self, callback)
    }

    /// Transforms an error emitted by the source while leaving its items unchanged.
    fn map_err<E1, F>(self, callback: F) -> MapErr<E, Self, F>
    where
        F: FnOnce(E) -> E1,
    {
        MapErr::new(self, callback)
    }

    /// Wraps each item into a notification, turning the stream into explicit events.
    fn materialize(self) -> Materialize<Self> {
        Materialize::new(self)
    }

    /// Emits the maximum item produced by the source according to the natural order.
    fn max(self) -> Max<Self> {
        Max::new(self)
    }

    /// Merges an observable-of-observables by interleaving items from inner streams.
    fn merge_all<T1>(self) -> MergeAll<Self, T>
    where
        T: Observable<'or, T1, E>,
    {
        MergeAll::new(self)
    }

    /// Merges the source with another observable, interleaving both streams concurrently.
    fn merge_with<OE2>(self, source_2: OE2) -> Merge<Self, OE2>
    where
        OE2: Observable<'or, T, E>,
    {
        Merge::new(self, source_2)
    }

    /// Emits the minimum item produced by the source according to the natural order.
    fn min(self) -> Min<Self> {
        Min::new(self)
    }

    /// Converts the source into a connectable observable using a subject factory.
    fn multicast<S, F>(self, subject_maker: F) -> ConnectableController<Self, S>
    where
        F: FnOnce() -> S,
    {
        ConnectableController::new(self, subject_maker())
    }

    /// Schedules downstream observation on the provided scheduler.
    fn observe_on<S>(self, scheduler: S) -> ObserveOn<'or, Self, S> {
        ObserveOn::new(self, scheduler)
    }

    fn on_backpressure<C>(self, collection: C) -> OnBackpressure<Self, C>
    where
        C: BackpressureCollection<Input = T>,
    {
        OnBackpressure::new(self, collection)
    }

    fn on_backpressure_buffer(self) -> OnBackpressureBuffer<Self> {
        OnBackpressureBuffer::new(self)
    }

    fn on_backpressure_latest(self) -> OnBackpressureLatest<Self> {
        OnBackpressureLatest::new(self)
    }

    /// Multicasts the source using a `PublishSubject`.
    fn publish(self) -> ConnectableController<Self, PublishSubject<'or, T, E>> {
        self.multicast(PublishSubject::default)
    }

    /// Multicasts the source using an `AsyncSubject`, emitting only the last value.
    fn publish_last(self) -> ConnectableController<Self, AsyncSubject<'or, T, E>> {
        self.multicast(AsyncSubject::default)
    }

    /// Aggregates the sequence using an initial seed and an accumulator function.
    fn reduce<T0, F>(self, initial_value: T0, callback: F) -> Reduce<T0, T, Self, F>
    where
        F: FnMut(T0, T) -> T0,
    {
        Reduce::new(self, initial_value, callback)
    }

    /// Multicasts the source using a `ReplaySubject` configured with the given buffer size.
    fn replay(
        self,
        buffer_size: Option<usize>,
    ) -> ConnectableController<Self, ReplaySubject<'or, T, E>> {
        self.multicast(|| ReplaySubject::new(buffer_size))
    }

    /// Re-subscribes to the source based on the retry strategy returned by the callback.
    fn retry<OE1, F>(self, callback: F) -> Retry<Self, F>
    where
        OE1: Observable<'or, T, E>,
        F: FnMut(E) -> RetryAction<E, OE1>,
    {
        Retry::new(self, callback)
    }

    /// Samples the source whenever the sampler observable emits an event.
    fn sample<OE1>(self, sampler: OE1) -> Sample<Self, OE1>
    where
        OE1: Observable<'or, (), E>,
    {
        Sample::new(self, sampler)
    }

    /// Accumulates values over time, emitting each intermediate result.
    fn scan<T0, F>(self, initial_value: T0, callback: F) -> Scan<T0, T, Self, F>
    where
        F: FnMut(T0, T) -> T0,
    {
        Scan::new(self, initial_value, callback)
    }

    /// Compares two sequences element by element for equality.
    fn sequence_equal<OE2>(self, another_source: OE2) -> SequenceEqual<T, Self, OE2>
    where
        OE2: Observable<'or, T, E>,
    {
        SequenceEqual::new(self, another_source)
    }

    /// Shares a single subscription to the source using `PublishSubject` semantics.
    fn share(self) -> RefCount<'or, T, E, Self, PublishSubject<'or, T, E>> {
        self.publish().ref_count()
    }

    /// Shares a single subscription, replaying only the last item to new subscribers.
    fn share_last(self) -> RefCount<'or, T, E, Self, AsyncSubject<'or, T, E>> {
        self.publish_last().ref_count()
    }

    /// Shares a single subscription while replaying a bounded history to future subscribers.
    fn share_replay(
        self,
        buffer_size: Option<usize>,
    ) -> RefCount<'or, T, E, Self, ReplaySubject<'or, T, E>> {
        self.replay(buffer_size).ref_count()
    }

    /// Skips the first `count` items before emitting the remainder of the sequence.
    fn skip(self, count: usize) -> Skip<Self> {
        Skip::new(self, count)
    }

    /// Skips the last `count` items emitted by the source.
    fn skip_last(self, count: usize) -> SkipLast<Self> {
        SkipLast::new(self, count)
    }

    /// Ignores items from the source until the notifier observable fires.
    fn skip_until<OE1>(self, start: OE1) -> SkipUntil<Self, OE1>
    where
        OE1: Observable<'or, (), E>,
    {
        SkipUntil::new(self, start)
    }

    /// Skips items while the predicate returns `true`, then emits the remaining items.
    fn skip_while<F>(self, callback: F) -> SkipWhile<Self, F>
    where
        F: FnMut(&T) -> bool,
    {
        SkipWhile::new(self, callback)
    }

    /// Pre-pends the provided values before the source starts emitting.
    fn start_with<I>(self, values: I) -> StartWith<Self, I>
    where
        I: IntoIterator<Item = T>,
    {
        StartWith::new(self, values)
    }

    /// Subscribes to the source on the provided scheduler.
    fn subscribe_on<S>(self, scheduler: S) -> SubscribeOn<'or, Self, S> {
        SubscribeOn::new(self, scheduler)
    }

    /// Convenience helper for subscribing with plain callbacks instead of a full observer.
    fn subscribe_with_callback<FN, FT>(
        self,
        on_next: FN,
        on_termination: FT,
    ) -> Subscription<Self::D>
    where
        FN: FnMut(T) + MaybeSend + 'or,
        FT: FnOnce(Termination<E>) + MaybeSend + 'or,
    {
        self.subscribe(CallbackObserver::new(on_next, on_termination))
    }

    /// Sums all numeric items and emits the accumulated total.
    fn sum(self) -> Sum<Self> {
        Sum::new(self)
    }

    /// Switches to the most recent inner observable emitted by the source.
    fn switch<T1>(self) -> Switch<Self, T>
    where
        T: Observable<'or, T1, E>,
    {
        Switch::new(self)
    }

    /// Maps each item to an observable and switches to the latest inner sequence.
    fn switch_map<T1, OE1, F>(self, callback: F) -> SwitchMap<T, Self, OE1, F>
    where
        OE1: Observable<'or, T1, E>,
        F: FnMut(T) -> OE1,
    {
        SwitchMap::new(self, callback)
    }

    /// Emits only the first `count` items from the source before completing.
    fn take(self, count: usize) -> Take<Self> {
        Take::new(self, count)
    }

    /// Emits only the last `count` items produced by the source.
    fn take_last(self, count: usize) -> TakeLast<Self> {
        TakeLast::new(self, count)
    }

    /// Relays items until the notifier observable emits, then completes.
    fn take_until<OE1>(self, stop: OE1) -> TakeUntil<Self, OE1>
    where
        OE1: Observable<'or, (), E>,
    {
        TakeUntil::new(self, stop)
    }

    /// Emits items while the predicate holds `true`, then completes.
    fn take_while<F>(self, callback: F) -> TakeWhile<Self, F>
    where
        F: FnMut(&T) -> bool,
    {
        TakeWhile::new(self, callback)
    }

    /// Throttles emissions to at most one item per specified timespan.
    ///
    /// Leading-edge and scheduler-free: the cooldown is decided by comparing
    /// item arrival times, so no timer is spawned.
    fn throttle(self, time_span: Duration) -> Throttle<Self> {
        Throttle::new(self, time_span)
    }

    /// Emits elapsed time between consecutive items as they flow through the stream.
    fn time_interval(self) -> TimeInterval<Self> {
        TimeInterval::new(self)
    }

    /// Errors if the next item does not arrive within the specified duration.
    fn timeout<S>(self, duration: Duration, scheduler: S) -> Timeout<'or, Self, S> {
        Timeout::new(self, duration, scheduler)
    }

    /// Annotates each item with the current timestamp when it is emitted.
    fn timestamp(self) -> Timestamp<Self> {
        Timestamp::new(self)
    }

    /// Collects items into windows that are opened and closed by another observable.
    /// Completing the boundary stops future window rotation without terminating the source.
    fn window<OE1>(self, boundary: OE1) -> Window<Self, OE1>
    where
        OE1: Observable<'or, (), Infallible>,
    {
        Window::new(self, boundary)
    }

    /// Collects items into windows containing a fixed number of elements.
    fn window_with_count(self, count: NonZeroUsize) -> WindowWithCount<Self> {
        WindowWithCount::new(self, count)
    }

    /// Gives an Observable whose error type is `Infallible` a concrete error type.
    fn with_error_type<E1>(self) -> WithErrorType<E1, Self> {
        WithErrorType::new(self)
    }

    /// Gives an Observable whose item type is `Infallible` a concrete item type.
    fn with_item_type<T1>(self) -> WithItemType<T1, Self> {
        WithItemType::new(self)
    }

    /// Pairs items from both observables by index and emits tuples of corresponding values.
    fn zip<T1, OE2>(self, another_source: OE2) -> Zip<Self, OE2>
    where
        OE2: Observable<'or, T1, E>,
    {
        Zip::new(self, another_source)
    }
}

impl<'or, T, E, OE> ObservableExt<'or, T, E> for OE where OE: Observable<'or, T, E> {}
