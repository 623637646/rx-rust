//! The sending end of a stream: the [`Observable`] trait and the fluent operator API on it.
//!
//! An [`Observable`] is anything that can be subscribed to with an
//! [`Observer`]; subscribing returns a [`Subscription`], which unsubscribes when dropped. Every
//! operator is a struct in [`operators`](crate::operators) that implements `Observable` over its
//! source, and [`ObservableExt`] gives each of them a method, so a pipeline reads as a chain of
//! calls.
//!
//! [`BoxedObservable`] and [`CloneableBoxedObservable`] erase an observable's type;
//! [`EitherObservable`] picks one of two types without boxing.
//!
//! # Examples
//! ```rust
//! use rx_rust::{observable::ObservableExt, observer::Termination, operators::creating::range::Range};
//!
//! let mut seen = Vec::new();
//! Range::new(1..=10)
//!     .filter(|value| value % 2 == 0)
//!     .map(|value| value * value)
//!     .take(3)
//!     .subscribe_with_callback(
//!         |value| seen.push(value),
//!         |termination| assert_eq!(termination, Termination::Completed),
//!     );
//! assert_eq!(seen, [4, 16, 36]);
//! ```

pub mod boxed_observable;
pub mod either_observable;

#[cfg(feature = "futures")]
use crate::operators::others::{
    observable_stream::ObservableStream,
    observable_try_stream::{ObservableTryStream, StreamBuffer},
};
use crate::{
    disposable::{Disposable, bound_drop_disposal::BoundDropDisposal},
    observable::{
        boxed_observable::{
            BoxedObservable, BoxedObservableFor, CloneableBoxedObservable,
            CloneableBoxedObservableFor, SendBoxedObservable, SendBoxedObservableFor,
            SendCloneableBoxedObservable, SendCloneableBoxedObservableFor,
        },
        either_observable::EitherObservable,
    },
    observer::{
        Flow, Observer, Termination,
        boxed_observer::{BoxedObserver, ObserverMode, SendBoxedObserver},
        callback_observer::{CallbackObserver, IntoFlow},
        emitter::Emitter,
    },
    operators::{
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
            average::Average, collect::Collect, count::Count, max::Max, min::Min, reduce::Reduce,
            sum::Sum,
        },
        others::{
            debug::{Debug, DebugEvent, DefaultPrintType},
            hook_on_next::HookOnNext,
            hook_on_subscription::HookOnSubscription,
            hook_on_termination::HookOnTermination,
            into_shared::IntoShared,
            observable_future::ObservableFuture,
            observable_try_future::ObservableTryFuture,
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
    thread_mode::ThreadMode,
};
use std::{fmt::Display, num::NonZeroUsize, time::Duration};

/// What [`Observable::subscribe`] returns: a disposal that unsubscribes when it is dropped.
pub type Subscription<D> = BoundDropDisposal<D>;

/// The part of an observable that does not depend on who observes it: what it emits, the thread
/// its events can arrive on, and the disposal of a subscription to it.
///
/// It is split from [`Observable`] on purpose, so that none of these can mention the observer's
/// type. An operator that owns the subscriptions of two sources, such as `merge`, subscribes each
/// of them with an observer whose type contains the other one's disposal; if a disposal could
/// depend on its observer, the disposal of the second source would appear in its own definition.
pub trait ObservableTypes {
    /// The values.
    type Item;
    /// The error a failed stream ends with.
    type Error;
    /// Whether the events can arrive from another thread than the one that subscribed:
    /// [`Local`](crate::thread_mode::Local) when they cannot,
    /// [`Shared`](crate::thread_mode::Shared) when they can.
    ///
    /// It is computed along the chain from the source down: a synchronous source is `Local`, an
    /// operator delivering through a scheduler takes the scheduler's mode, and an operator with
    /// several sources joins theirs ([`Joined`](crate::thread_mode::Joined)). An operator that
    /// needs shared state picks its pointer from it. See [`thread_mode`](crate::thread_mode).
    type Mode: ThreadMode;
    /// The disposal of a subscription to this observable.
    type Disposal: Disposable;
}

/// A source of [`Item`](ObservableTypes::Item)s that ends with a
/// [`Termination`]`<`[`Error`](ObservableTypes::Error)`>`, subscribed to by an observer of type
/// `OR`. See <https://reactivex.io/documentation/observable.html>.
///
/// The observer is a parameter of the trait rather than of `subscribe`, so that each implementation
/// states what it needs from its observer: only an operator that hands the observer to another
/// thread — through a scheduler, or a `Send` box — asks for `Send`, and a chain that stays on its
/// thread accepts any observer. See [`ObservableTypes`] for why the associated types are not
/// declared here.
///
/// # Examples
/// A synchronous chain takes an observer that is not `Send`:
/// ```rust
/// use rx_rust::{observable::ObservableExt, operators::creating::just::Just};
/// use std::{cell::RefCell, rc::Rc};
///
/// let values = Rc::new(RefCell::new(Vec::new()));
/// let observer_values = values.clone();
/// let _subscription = Just::new(1)
///     .map(|value| value + 1)
///     .subscribe_with_callback(move |value| observer_values.borrow_mut().push(value), |_| {});
/// assert_eq!(*values.borrow(), [2]);
/// ```
pub trait Observable<OR>: ObservableTypes
where
    OR: Observer<Self::Item, Self::Error>,
{
    /// Subscribes `observer`, which receives the events from now on, consuming the observable.
    ///
    /// The returned [`Subscription`] unsubscribes when dropped.
    fn subscribe(self, observer: OR) -> Subscription<Self::Disposal>;
}

/// The operators, as methods on every [`Observable`].
///
/// Each method builds the operator of the same name over `self`; the operator's own documentation
/// in [`operators`](crate::operators) describes its behavior in detail and has an example. See the
/// [module documentation](self) for a pipeline.
pub trait ObservableExt: ObservableTypes + Sized {
    /// Emits a single `bool` indicating whether every item satisfies the provided predicate.
    fn all<F>(self, callback: F) -> All<Self::Item, Self, F>
    where
        F: FnMut(Self::Item) -> bool,
    {
        All::new(self, callback)
    }

    /// Races two observables and mirrors whichever one emits an item or terminates first.
    fn amb_with<OE1>(self, other: OE1) -> Amb<[EitherObservable<Self, OE1>; 2]>
    where
        OE1: ObservableTypes<Item = Self::Item, Error = Self::Error>,
    {
        Amb::new([EitherObservable::Left(self), EitherObservable::Right(other)])
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
    fn buffer<OE1>(self, boundary: OE1) -> Buffer<Self, OE1>
    where
        OE1: ObservableTypes<Item = (), Error = Self::Error>,
    {
        Buffer::new(self, boundary)
    }

    /// Collects items into fixed-size buffers and emits each buffer as soon as it fills up.
    fn buffer_with_count(self, count: NonZeroUsize) -> BufferWithCount<Self> {
        BufferWithCount::new(self, count)
    }

    /// Collects items into a buffer emitted every `time_span`, the first after `delay` (at once for
    /// `None`), on the provided scheduler.
    fn buffer_with_time<S>(
        self,
        time_span: Duration,
        scheduler: S,
        delay: Option<Duration>,
    ) -> BufferWithTime<Self, S> {
        BufferWithTime::new(self, time_span, scheduler, delay)
    }

    /// Collects items into buffers emitted when they reach `count` items or every `time_span`,
    /// whichever comes first; the timer first fires after `delay` (at once for `None`).
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
    fn catch<E1, OE1, F>(self, callback: F) -> Catch<Self::Error, Self, F>
    where
        OE1: ObservableTypes<Item = Self::Item, Error = E1>,
        F: FnOnce(Self::Error) -> OE1,
    {
        Catch::new(self, callback)
    }

    /// Gathers all the items into a collection built with `Default` and `Extend`, and emits it
    /// when the source completes. See [`to_vec`](ObservableExt::to_vec) for the `Vec<T>` case.
    fn collect<C>(self) -> Collect<C, Self::Item, Self>
    where
        C: Default + Extend<Self::Item>,
    {
        Collect::new(self)
    }

    /// Combines the latest values from both observables whenever either produces a new item.
    fn combine_latest<T1, OE2>(self, another_source: OE2) -> CombineLatest<Self, OE2>
    where
        OE2: ObservableTypes<Item = T1, Error = Self::Error>,
    {
        CombineLatest::new(self, another_source)
    }

    /// Flattens an observable-of-observables by concatenating each inner observable sequentially.
    fn concat_all<T1>(self) -> ConcatAll<Self, Self::Item>
    where
        Self::Item: ObservableTypes<Item = T1, Error = Self::Error>,
    {
        ConcatAll::new(self)
    }

    /// Maps each item to an observable and concatenates the resulting inner sequences.
    fn concat_map<T1, OE1, F>(self, callback: F) -> ConcatMap<Self::Item, Self, OE1, F>
    where
        OE1: ObservableTypes<Item = T1, Error = Self::Error>,
        F: FnMut(Self::Item) -> OE1,
    {
        ConcatMap::new(self, callback)
    }

    /// Concatenates the source with another observable, waiting for the first to complete.
    fn concat_with<OE2>(self, source_2: OE2) -> Concat<Self, OE2>
    where
        OE2: ObservableTypes<Item = Self::Item, Error = Self::Error>,
    {
        Concat::new(self, source_2)
    }

    /// Emits `true` if the sequence contains the provided item, `false` otherwise.
    fn contains(self, item: Self::Item) -> Contains<Self::Item, Self> {
        Contains::new(self, item)
    }

    /// Counts the number of items emitted and emits that count as a single value.
    fn count(self) -> Count<Self::Item, Self> {
        Count::new(self)
    }

    /// Emits an item only once `time_span` has passed without the source emitting another one.
    fn debounce<S>(self, time_span: Duration, scheduler: S) -> Debounce<Self, S> {
        Debounce::new(self, time_span, scheduler)
    }

    /// Reports every event of the stream — subscription, values, termination, disposal — to
    /// `callback`, together with `context`.
    fn debug<C, F>(self, context: C, callback: F) -> Debug<Self, C, F>
    where
        F: Fn(C, DebugEvent<'_, Self::Item, Self::Error>),
    {
        Debug::new(self, context, callback)
    }

    /// [`debug`](ObservableExt::debug) that prints every event with `println!`, prefixed by
    /// `label`.
    fn debug_default_print<L>(
        self,
        label: L,
    ) -> Debug<Self, L, DefaultPrintType<L, Self::Item, Self::Error>>
    where
        L: Display,
        Self::Item: std::fmt::Debug,
        Self::Error: std::fmt::Debug,
    {
        Debug::new_default_print(self, label)
    }

    /// Emits a default value if the source completes without emitting any items.
    fn default_if_empty(self, default_value: Self::Item) -> DefaultIfEmpty<Self::Item, Self> {
        DefaultIfEmpty::new(self, default_value)
    }

    /// Offsets the emission of items by the specified duration using the given scheduler.
    fn delay<S>(self, delay: Duration, scheduler: S) -> Delay<Self, S> {
        Delay::new(self, delay, scheduler)
    }

    /// Turns a stream of [`Event`](crate::observer::Event)s back into the events they describe.
    fn dematerialize(self) -> Dematerialize<Self> {
        Dematerialize::new(self)
    }

    /// Filters out duplicate items, keeping only the first occurrence of each value.
    #[allow(clippy::type_complexity)]
    fn distinct(self) -> Distinct<Self, fn(&Self::Item) -> Self::Item>
    where
        Self::Item: Clone,
    {
        Distinct::new(self)
    }

    /// Suppresses consecutive duplicate items, comparing the values directly.
    #[allow(clippy::type_complexity)]
    fn distinct_until_changed(self) -> DistinctUntilChanged<Self, fn(&Self::Item) -> Self::Item>
    where
        Self::Item: Clone,
    {
        DistinctUntilChanged::new(self)
    }

    /// Suppresses consecutive duplicate items using a custom key selector.
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
    fn do_after_termination<F>(self, callback: F) -> DoAfterTermination<Self, F>
    where
        F: FnOnce(Termination<Self::Error>),
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
        F: FnMut(&Self::Item),
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
        F: FnOnce(&Termination<Self::Error>),
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
        F: FnMut(&Self::Item) -> bool,
    {
        Filter::new(self, callback)
    }

    /// Emits only the first item from the source, then completes; nothing if there was none.
    fn first(self) -> First<Self> {
        First::new(self)
    }

    /// Maps each item to an observable and merges the resulting inner sequences concurrently.
    fn flat_map<T1, OE1, F>(self, callback: F) -> FlatMap<Self::Item, Self, OE1, F>
    where
        OE1: ObservableTypes<Item = T1, Error = Self::Error>,
        F: FnMut(Self::Item) -> OE1,
    {
        FlatMap::new(self, callback)
    }

    /// Groups items by key into multiple observable sequences.
    fn group_by<'a, F, K>(self, key_selector: F) -> GroupBy<'a, Self, F, K>
    where
        Self::Mode: ObserverMode,
        F: FnMut(&Self::Item) -> K,
    {
        GroupBy::new(self, key_selector)
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
        F: FnOnce(Self, Emitter<OR, Self::Mode>) -> Subscription<D>,
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
        F: FnOnce(
            Self,
            <Self::Mode as ObserverMode>::BoxedObserver<'a, Self::Item, Self::Error>,
        ) -> Subscription<D>,
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
        F: FnOnce(Emitter<OR, Self::Mode>, Termination<Self::Error>),
    {
        HookOnTermination::new(self, callback)
    }

    /// Like [`hook_on_termination`](ObservableExt::hook_on_termination), but the callback gets the
    /// boxed observer of the source's mode, so the operator subscribes any observer.
    fn hook_on_termination_boxed<'a, F>(self, callback: F) -> HookOnTermination<Self, F, true>
    where
        Self::Mode: ObserverMode,
        F: FnOnce(
            <Self::Mode as ObserverMode>::BoxedObserver<'a, Self::Item, Self::Error>,
            Termination<Self::Error>,
        ),
    {
        HookOnTermination::new_boxed(self, callback)
    }

    /// Ignores all items from the source, only relaying termination events.
    fn ignore_elements(self) -> IgnoreElements<Self> {
        IgnoreElements::new(self)
    }

    /// Erases the observable's concrete type. See [`boxed_observable`] for the flavors; the
    /// lifetimes bound the observer (`'or`), the disposal (`'sub`) and the observable itself
    /// (`'oe`).
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
    fn into_boxed<'or, 'sub, 'oe>(
        self,
    ) -> BoxedObservable<'or, 'sub, 'oe, Self::Item, Self::Error, Self::Mode>
    where
        Self: Observable<
                BoxedObserver<
                    'or,
                    <Self as ObservableTypes>::Item,
                    <Self as ObservableTypes>::Error,
                >,
            > + 'oe,
        Self::Disposal: 'sub,
    {
        BoxedObservable::new(self)
    }

    /// Erases the observable's concrete type, keeping it `Send`.
    ///
    /// Its observer is boxed as a [`SendBoxedObserver`], so it must be `Send` whatever the
    /// source's mode, even for a `Local` source that [`into_boxed`](Self::into_boxed) would let
    /// take an `Rc`:
    ///
    /// ```compile_fail
    /// use rx_rust::{observable::ObservableExt, operators::creating::just::Just};
    /// use std::{cell::RefCell, rc::Rc};
    ///
    /// let values = Rc::new(RefCell::new(Vec::new()));
    /// let _subscription = Just::new(1)
    ///     .into_send_boxed()
    ///     .subscribe_with_callback(move |value| values.borrow_mut().push(value), |_| {});
    /// ```
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
    fn into_send_boxed<'or, 'sub, 'oe>(
        self,
    ) -> SendBoxedObservable<'or, 'sub, 'oe, Self::Item, Self::Error, Self::Mode>
    where
        Self: Observable<
                SendBoxedObserver<
                    'or,
                    <Self as ObservableTypes>::Item,
                    <Self as ObservableTypes>::Error,
                >,
            > + Send
            + 'oe,
        Self::Disposal: Send + 'sub,
    {
        SendBoxedObservable::new(self)
    }

    /// Erases the observable's concrete type except for its observer's, `OR`: only subscribing is
    /// dynamically dispatched, the events are not.
    fn into_boxed_for<'sub, 'oe, OR>(
        self,
    ) -> BoxedObservableFor<'sub, 'oe, Self::Item, Self::Error, Self::Mode, OR>
    where
        OR: Observer<Self::Item, Self::Error>,
        Self: Observable<OR> + 'oe,
        Self::Disposal: 'sub,
    {
        BoxedObservableFor::new(self)
    }

    /// [`into_boxed_for`](Self::into_boxed_for), keeping the observable `Send`.
    fn into_send_boxed_for<'sub, 'oe, OR>(
        self,
    ) -> SendBoxedObservableFor<'sub, 'oe, Self::Item, Self::Error, Self::Mode, OR>
    where
        OR: Observer<Self::Item, Self::Error>,
        Self: Observable<OR> + Send + 'oe,
        Self::Disposal: Send + 'sub,
    {
        SendBoxedObservableFor::new(self)
    }

    /// Erases the observable's concrete type and makes it cloneable.
    fn into_cloneable_boxed<'or, 'sub, 'oe>(
        self,
    ) -> CloneableBoxedObservable<'or, 'sub, 'oe, Self::Item, Self::Error, Self::Mode>
    where
        Self: Observable<
                BoxedObserver<
                    'or,
                    <Self as ObservableTypes>::Item,
                    <Self as ObservableTypes>::Error,
                >,
            > + Clone
            + 'oe,
        Self::Disposal: 'sub,
    {
        CloneableBoxedObservable::new(self)
    }

    /// [`into_cloneable_boxed`](Self::into_cloneable_boxed), keeping the observable `Send` and
    /// `Sync`.
    fn into_send_cloneable_boxed<'or, 'sub, 'oe>(
        self,
    ) -> SendCloneableBoxedObservable<'or, 'sub, 'oe, Self::Item, Self::Error, Self::Mode>
    where
        Self: Observable<
                SendBoxedObserver<
                    'or,
                    <Self as ObservableTypes>::Item,
                    <Self as ObservableTypes>::Error,
                >,
            > + Clone
            + Send
            + Sync
            + 'oe,
        Self::Disposal: Send + 'sub,
    {
        SendCloneableBoxedObservable::new(self)
    }

    /// [`into_boxed_for`](Self::into_boxed_for), making the observable cloneable.
    fn into_cloneable_boxed_for<'sub, 'oe, OR>(
        self,
    ) -> CloneableBoxedObservableFor<'sub, 'oe, Self::Item, Self::Error, Self::Mode, OR>
    where
        OR: Observer<Self::Item, Self::Error>,
        Self: Observable<OR> + Clone + 'oe,
        Self::Disposal: 'sub,
    {
        CloneableBoxedObservableFor::new(self)
    }

    /// [`into_cloneable_boxed_for`](Self::into_cloneable_boxed_for), keeping the observable `Send`
    /// and `Sync`.
    fn into_send_cloneable_boxed_for<'sub, 'oe, OR>(
        self,
    ) -> SendCloneableBoxedObservableFor<'sub, 'oe, Self::Item, Self::Error, Self::Mode, OR>
    where
        OR: Observer<Self::Item, Self::Error>,
        Self: Observable<OR> + Clone + Send + Sync + 'oe,
        Self::Disposal: Send + 'sub,
    {
        SendCloneableBoxedObservableFor::new(self)
    }

    /// Declares the observable [`Shared`](crate::thread_mode::Shared), so that a `Local` source
    /// can be erased into the same type as a `Shared` one, or into a `Send` box.
    fn into_shared(self) -> IntoShared<Self> {
        IntoShared::new(self)
    }

    /// Converts the observable into a future of its first item: `Some(item)`, or `None` when the
    /// source completes without one. The source is stopped as soon as the item is in.
    ///
    /// This is only for a source that cannot fail; a fallible one goes through
    /// [`into_try_future`](Self::into_try_future). An operator that picks another item, such as
    /// `last`, or one that always emits, such as `collect`, goes in front of it.
    fn into_future(self) -> ObservableFuture<Self>
    where
        Self: ObservableTypes<Error = std::convert::Infallible>,
    {
        ObservableFuture::new(self)
    }

    /// Converts the observable into an async stream.
    ///
    /// A `Stream` has no error channel, so this is only for a source that cannot fail; a
    /// fallible one goes through [`into_try_stream`](Self::into_try_stream).
    ///
    /// The items that arrive between two polls are all kept, so a source faster than the
    /// consumer grows the buffer without bound; [`into_stream_with`](Self::into_stream_with)
    /// takes a buffer that bounds it.
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
    /// [`Latest`](crate::operators::others::observable_try_stream::Latest) keeps only the newest
    /// item, [`Bounded`](crate::operators::others::observable_try_stream::Bounded) a fixed number
    /// of them and [`Unbounded`](crate::operators::others::observable_try_stream::Unbounded) —
    /// what [`into_stream`](Self::into_stream) uses — everything; a [`StreamBuffer`] of your own
    /// can fold them instead. Whatever the buffer, the source is never slowed down: a `Stream`
    /// only pulls from the buffer, not from the source.
    ///
    /// # Examples
    /// ```rust
    /// use futures::{FutureExt, StreamExt};
    /// use rx_rust::{
    ///     observable::ObservableExt, observer::Observer,
    ///     operators::others::observable_try_stream::Latest,
    ///     subject::publish_subject::PublishSubject,
    /// };
    /// use std::convert::Infallible;
    ///
    /// let mut subject = PublishSubject::<_, Infallible, rx_rust::thread_mode::Local>::local();
    /// let mut stream = subject.clone().into_stream_with(Latest::new());
    /// assert_eq!(stream.next().now_or_never(), None); // subscribes
    ///
    /// subject.on_next(1);
    /// subject.on_next(2);
    /// subject.on_next(3);
    /// assert_eq!(stream.next().now_or_never(), Some(Some(3)));
    /// assert_eq!(stream.next().now_or_never(), None);
    /// ```
    #[cfg(feature = "futures")]
    fn into_stream_with<B>(self, buffer: B) -> ObservableStream<Self::Item, Self, B>
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
    fn into_try_future(self) -> ObservableTryFuture<Self> {
        ObservableTryFuture::new(self)
    }

    /// Converts the observable into an async stream of `Result`s: each item as `Ok`, and an error
    /// as the last item, `Err`, before the stream ends.
    ///
    /// The items that arrive between two polls are all kept, so a source faster than the
    /// consumer grows the buffer without bound;
    /// [`into_try_stream_with`](Self::into_try_stream_with) takes a buffer that bounds it.
    #[cfg(feature = "futures")]
    fn into_try_stream(self) -> ObservableTryStream<Self::Item, Self::Error, Self> {
        ObservableTryStream::new(self)
    }

    /// Converts the observable into an async stream of `Result`s that keeps the items arriving
    /// between two polls in `buffer`. This is [`into_stream_with`](Self::into_stream_with) for
    /// a source that can fail; see there for the buffers.
    #[cfg(feature = "futures")]
    fn into_try_stream_with<B>(
        self,
        buffer: B,
    ) -> ObservableTryStream<Self::Item, Self::Error, Self, B>
    where
        B: StreamBuffer<Self::Item>,
    {
        ObservableTryStream::with_buffer(self, buffer)
    }

    /// Emits only the last item of the source, on completion; nothing if there was none.
    fn last(self) -> Last<Self> {
        Last::new(self)
    }

    /// Transforms each item by applying a user-supplied mapping function.
    fn map<T1, F>(self, callback: F) -> Map<Self::Item, Self, F>
    where
        F: FnMut(Self::Item) -> T1,
    {
        Map::new(self, callback)
    }

    /// Transforms an error emitted by the source while leaving its items unchanged.
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

    /// Merges an observable-of-observables by interleaving items from inner streams.
    fn merge_all<T1>(self) -> MergeAll<Self, Self::Item>
    where
        Self::Item: ObservableTypes<Item = T1, Error = Self::Error>,
    {
        MergeAll::new(self)
    }

    /// Merges the source with another observable, interleaving both streams concurrently.
    fn merge_with<OE2>(self, source_2: OE2) -> Merge<Self, OE2>
    where
        OE2: ObservableTypes<Item = Self::Item, Error = Self::Error>,
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

    /// Delivers the events downstream from tasks of the provided scheduler.
    fn observe_on<S>(self, scheduler: S) -> ObserveOn<Self, S> {
        ObserveOn::new(self, scheduler)
    }

    /// Multicasts the source using a `PublishSubject`.
    #[allow(clippy::type_complexity)]
    fn publish<'a>(
        self,
    ) -> ConnectableController<Self, PublishSubject<'a, Self::Item, Self::Error, Self::Mode>>
    where
        Self::Mode: ObserverMode,
    {
        self.multicast(PublishSubject::new)
    }

    /// Multicasts the source using an `AsyncSubject`, emitting only the last value.
    #[allow(clippy::type_complexity)]
    fn publish_last<'a>(
        self,
    ) -> ConnectableController<Self, AsyncSubject<'a, Self::Item, Self::Error, Self::Mode>>
    where
        Self::Mode: ObserverMode,
    {
        self.multicast(AsyncSubject::new)
    }

    /// Aggregates the sequence using an initial seed and an accumulator function.
    fn reduce<T0, F>(self, initial_value: T0, callback: F) -> Reduce<T0, Self::Item, Self, F>
    where
        F: FnMut(T0, Self::Item) -> T0,
    {
        Reduce::new(self, initial_value, callback)
    }

    /// Multicasts the source using a `ReplaySubject` that keeps the last `buffer_size` values, or
    /// every value for `None`.
    #[allow(clippy::type_complexity)]
    fn replay<'a>(
        self,
        buffer_size: Option<usize>,
    ) -> ConnectableController<Self, ReplaySubject<'a, Self::Item, Self::Error, Self::Mode>>
    where
        Self::Mode: ObserverMode,
    {
        self.multicast(|| ReplaySubject::new(buffer_size))
    }

    /// Re-subscribes to the source based on the retry strategy returned by the callback.
    fn retry<OE1, F>(self, callback: F) -> Retry<Self, F>
    where
        OE1: ObservableTypes<Item = Self::Item, Error = Self::Error>,
        F: FnMut(Self::Error) -> RetryAction<Self::Error, OE1>,
    {
        Retry::new(self, callback)
    }

    /// Samples the source whenever the sampler observable emits an event.
    fn sample<OE1>(self, sampler: OE1) -> Sample<Self, OE1>
    where
        OE1: ObservableTypes<Item = (), Error = Self::Error>,
    {
        Sample::new(self, sampler)
    }

    /// Accumulates values over time, emitting each intermediate result.
    fn scan<T0, F>(self, initial_value: T0, callback: F) -> Scan<T0, Self::Item, Self, F>
    where
        F: FnMut(T0, Self::Item) -> T0,
    {
        Scan::new(self, initial_value, callback)
    }

    /// Compares two sequences element by element for equality.
    fn sequence_equal<OE2>(self, another_source: OE2) -> SequenceEqual<Self::Item, Self, OE2>
    where
        OE2: ObservableTypes<Item = Self::Item, Error = Self::Error>,
    {
        SequenceEqual::new(self, another_source)
    }

    /// Shares a single subscription to the source using `PublishSubject` semantics.
    #[allow(clippy::type_complexity)]
    fn share<'a>(self) -> RefCount<Self, PublishSubject<'a, Self::Item, Self::Error, Self::Mode>>
    where
        Self::Mode: ObserverMode,
        Self::Item: Clone,
        Self::Error: Clone,
    {
        self.publish().ref_count()
    }

    /// Shares a single subscription using `AsyncSubject` semantics: only the last item, on
    /// completion.
    #[allow(clippy::type_complexity)]
    fn share_last<'a>(self) -> RefCount<Self, AsyncSubject<'a, Self::Item, Self::Error, Self::Mode>>
    where
        Self::Mode: ObserverMode,
        Self::Item: Clone,
        Self::Error: Clone,
    {
        self.publish_last().ref_count()
    }

    /// Shares a single subscription using `ReplaySubject` semantics: later subscribers first get
    /// the last `buffer_size` values, or every value for `None`.
    #[allow(clippy::type_complexity)]
    fn share_replay<'a>(
        self,
        buffer_size: Option<usize>,
    ) -> RefCount<Self, ReplaySubject<'a, Self::Item, Self::Error, Self::Mode>>
    where
        Self::Mode: ObserverMode,
        Self::Item: Clone,
        Self::Error: Clone,
    {
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

    /// Ignores items from the source until the notifier observable emits; a notifier that
    /// completes without emitting completes the result.
    fn skip_until<OE1>(self, start: OE1) -> SkipUntil<Self, OE1>
    where
        OE1: ObservableTypes<Item = (), Error = Self::Error>,
    {
        SkipUntil::new(self, start)
    }

    /// Skips items while the predicate returns `true`, then emits the remaining items.
    fn skip_while<F>(self, callback: F) -> SkipWhile<Self, F>
    where
        F: FnMut(&Self::Item) -> bool,
    {
        SkipWhile::new(self, callback)
    }

    /// Emits the provided values first, then subscribes to the source.
    fn start_with<I>(self, values: I) -> StartWith<Self, I>
    where
        I: IntoIterator<Item = Self::Item>,
    {
        StartWith::new(self, values)
    }

    /// Subscribes to the source on the provided scheduler.
    fn subscribe_on<S>(self, scheduler: S) -> SubscribeOn<Self, S> {
        SubscribeOn::new(self, scheduler)
    }

    /// Subscribes with two closures instead of an [`Observer`].
    ///
    /// `on_next` may return nothing, which keeps the source going, or a [`Flow`], which lets it
    /// end its own stream with [`Flow::Stop`]: the source then stops pushing — a synchronous one
    /// stops iterating — and drops the callbacks without calling `on_termination`.
    fn subscribe_with_callback<FN, FT, R>(
        self,
        on_next: FN,
        on_termination: FT,
    ) -> Subscription<Self::Disposal>
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
    fn switch<T1>(self) -> Switch<Self, Self::Item>
    where
        Self::Item: ObservableTypes<Item = T1, Error = Self::Error>,
    {
        Switch::new(self)
    }

    /// Maps each item to an observable and switches to the latest inner sequence.
    fn switch_map<T1, OE1, F>(self, callback: F) -> SwitchMap<Self::Item, Self, OE1, F>
    where
        OE1: ObservableTypes<Item = T1, Error = Self::Error>,
        F: FnMut(Self::Item) -> OE1,
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

    /// Relays items until the notifier observable emits, then completes; a notifier that
    /// terminates first terminates the result the same way.
    fn take_until<OE1>(self, stop: OE1) -> TakeUntil<Self, OE1>
    where
        OE1: ObservableTypes<Item = (), Error = Self::Error>,
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

    /// Throttles emissions to at most one item per `time_span`, on the clock of `scheduler`.
    ///
    /// Leading-edge: the cooldown is decided by comparing item arrival times, so no timer is
    /// spawned; the scheduler only gives the time.
    fn throttle<S>(self, time_span: Duration, scheduler: S) -> Throttle<Self, S> {
        Throttle::new(self, time_span, scheduler)
    }

    /// Pairs each item with the time elapsed since the previous one, or since the subscription for
    /// the first, on the clock of `scheduler`.
    fn time_interval<S>(self, scheduler: S) -> TimeInterval<Self, S> {
        TimeInterval::new(self, scheduler)
    }

    /// Errors if the next item, or the first since the subscription, does not arrive within
    /// `duration`.
    fn timeout<S>(self, duration: Duration, scheduler: S) -> Timeout<Self, S> {
        Timeout::new(self, duration, scheduler)
    }

    /// Pairs each item with the [`Instant`](std::time::Instant) it arrived at, on the clock of
    /// `scheduler`.
    fn timestamp<S>(self, scheduler: S) -> Timestamp<Self, S> {
        Timestamp::new(self, scheduler)
    }

    /// Gathers all the items into a `Vec` and emits it when the source completes. This is
    /// [`collect`](ObservableExt::collect) specialized to `Vec<T>`, which is the shape that
    /// [`window`](ObservableExt::window) composes with:
    /// `source.window(boundary).concat_map(|window| window.to_vec())`.
    fn to_vec(self) -> Collect<Vec<Self::Item>, Self::Item, Self> {
        Collect::new(self)
    }

    /// Collects items into windows that are opened and closed by another observable.
    /// Completing the boundary stops future window rotation without terminating the source.
    /// An error from the boundary terminates the current window and the outer observable.
    fn window<'a, OE1>(self, boundary: OE1) -> Window<'a, Self, OE1>
    where
        Self::Mode: ObserverMode,
        OE1: ObservableTypes<Item = (), Error = Self::Error>,
    {
        Window::new(self, boundary)
    }

    /// Collects items into windows containing a fixed number of elements.
    fn window_with_count<'a>(self, count: NonZeroUsize) -> WindowWithCount<'a, Self>
    where
        Self::Mode: ObserverMode,
    {
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
        OE2: ObservableTypes<Item = T1, Error = Self::Error>,
    {
        Zip::new(self, another_source)
    }
}

impl<OE: ObservableTypes> ObservableExt for OE {}
