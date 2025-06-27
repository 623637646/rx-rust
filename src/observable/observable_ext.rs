use super::{
    Observable, boxed_observable::BoxedObservable, connectable_observable::ConnectableObservable,
    ref_count_observable::RefCount,
};
use crate::{
    observer::{Termination, boxed_observer::BoxedObserver, callback_observer::CallbackObserver},
    operators::{
        combining::{
            combine_latest::CombineLatest, concat::Concat, concat_all::ConcatAll, merge::Merge,
            merge_all::MergeAll, switch::Switch, zip::Zip,
        },
        conditional_boolean::take_until::TakeUntil,
        filtering::{
            debounce::Debounce, distinct::Distinct, distinct_until_changed::DistinctUntilChanged,
            element_at::ElementAt, filter::Filter, first::First, ignore_elements::IgnoreElements,
            last::Last, sample::Sample, skip::Skip, skip_last::SkipLast, take::Take,
            take_last::TakeLast, throttle::Throttle,
        },
        mathematical_aggregate::reduce::Reduce,
        others::{
            hook_on_next::HookOnNext, hook_on_subscription::HookOnSubscription,
            hook_on_termination::HookOnTermination, map_infallible_to_error::MapInfallibleToError,
            map_infallible_to_value::MapInfallibleToValue, observable_stream::ObservableStream,
        },
        transforming::{
            buffer::Buffer, buffer_with_count::BufferWithCount, buffer_with_time::BufferWithTime,
            buffer_with_time_or_count::BufferWithTimeOrCount, concat_map::ConcatMap,
            flat_map::FlatMap, group_by::GroupBy, map::Map, scan::Scan, switch_map::SwitchMap,
            window::Window, window_with_count::WindowWithCount,
        },
        utility::{
            delay::Delay, dematerialize::Dematerialize, do_on_next::DoOnNext,
            do_on_termination::DoOnTermination, materialize::Materialize,
        },
    },
    subject::{
        async_subject::AsyncSubject, publish_subject::PublishSubject, replay_subject::ReplaySubject,
    },
    subscription::Subscription,
};
use std::{convert::Infallible, num::NonZeroUsize, time::Duration};

pub trait ObservableExt<'or, 'sub, T, E>: Sized {
    fn buffer<OE1>(self, boundary: OE1) -> Buffer<Self, OE1>
    where
        Self: Observable<'or, 'sub, T, E>,
        OE1: Observable<'or, 'sub, (), E>,
    {
        Buffer::new(self, boundary)
    }

    fn buffer_with_count(self, count: NonZeroUsize) -> BufferWithCount<Self> {
        BufferWithCount::new(self, count)
    }

    fn buffer_with_time<S>(
        self,
        time_span: Duration,
        scheduler: S,
        delay: Option<Duration>,
    ) -> BufferWithTime<Self, S> {
        BufferWithTime::new(self, time_span, scheduler, delay)
    }

    fn buffer_with_time_or_count<S>(
        self,
        count: NonZeroUsize,
        time_span: Duration,
        scheduler: S,
        delay: Option<Duration>,
    ) -> BufferWithTimeOrCount<Self, S> {
        BufferWithTimeOrCount::new(self, count, time_span, scheduler, delay)
    }

    fn combine_latest<T1, OE2>(self, another_source: OE2) -> CombineLatest<Self, OE2>
    where
        Self: Observable<'or, 'sub, T, E>,
        OE2: Observable<'or, 'sub, T1, E>,
    {
        CombineLatest::new(self, another_source)
    }

    fn concat_all<T1>(self) -> ConcatAll<Self, T>
    where
        Self: Observable<'or, 'sub, T, E>,
        T: Observable<'or, 'sub, T1, E>,
    {
        ConcatAll::new(self)
    }

    fn concat_map<T1, OE1, F>(self, callback: F) -> ConcatMap<T, Self, OE1, F>
    where
        Self: Observable<'or, 'sub, T, E>,
        OE1: Observable<'or, 'sub, T1, E>,
        F: FnMut(T) -> OE1,
    {
        ConcatMap::new(self, callback)
    }

    fn concat_with<OE2>(self, source_2: OE2) -> Concat<Self, OE2>
    where
        Self: Observable<'or, 'sub, T, E>,
        OE2: Observable<'or, 'sub, T, E>,
    {
        Concat::new(self, source_2)
    }

    fn debounce<S>(self, time_span: Duration, scheduler: S) -> Debounce<Self, S> {
        Debounce::new(self, time_span, scheduler)
    }

    fn delay<S>(self, delay: Duration, scheduler: S) -> Delay<Self, S> {
        Delay::new(self, delay, scheduler)
    }

    fn dematerialize(self) -> Dematerialize<Self> {
        Dematerialize::new(self)
    }

    fn distinct(self) -> Distinct<Self, fn(&T) -> T>
    where
        T: Clone,
        Self: Observable<'or, 'sub, T, E>,
    {
        Distinct::new(self)
    }

    fn distinct_with_key_selector<F, K>(self, key_selector: F) -> Distinct<Self, F>
    where
        Self: Observable<'or, 'sub, T, E>,
        F: FnMut(&T) -> K,
    {
        Distinct::new_with_key_selector(self, key_selector)
    }

    fn distinct_until_changed(self) -> DistinctUntilChanged<Self, fn(&T) -> T>
    where
        T: Clone,
        Self: Observable<'or, 'sub, T, E>,
    {
        DistinctUntilChanged::new(self)
    }

    fn distinct_until_changed_with_key_selector<F, K>(
        self,
        key_selector: F,
    ) -> DistinctUntilChanged<Self, F>
    where
        Self: Observable<'or, 'sub, T, E>,
        F: FnMut(&T) -> K,
    {
        DistinctUntilChanged::new_with_key_selector(self, key_selector)
    }

    fn do_on_next<F>(self, callback: F) -> DoOnNext<Self, F>
    where
        Self: Observable<'or, 'sub, T, E>,
        F: FnMut(&T),
    {
        DoOnNext::new(self, callback)
    }

    fn do_on_termination<F>(self, callback: F) -> DoOnTermination<Self, F>
    where
        Self: Observable<'or, 'sub, T, E>,
        F: FnOnce(&Termination<E>),
    {
        DoOnTermination::new(self, callback)
    }

    fn element_at(self, index: usize) -> ElementAt<Self> {
        ElementAt::new(self, index)
    }

    fn filter<F>(self, callback: F) -> Filter<Self, F>
    where
        Self: Observable<'or, 'sub, T, E>,
        F: FnMut(&T) -> bool,
    {
        Filter::new(self, callback)
    }

    fn first(self) -> First<Self> {
        First::new(self)
    }

    fn flat_map<T1, OE1, F>(self, callback: F) -> FlatMap<T, Self, OE1, F>
    where
        Self: Observable<'or, 'sub, T, E>,
        OE1: Observable<'or, 'sub, T1, E>,
        F: FnMut(T) -> OE1,
    {
        FlatMap::new(self, callback)
    }

    fn group_by<F, K>(self, callback: F) -> GroupBy<Self, F, K>
    where
        Self: Observable<'or, 'sub, T, E>,
        F: FnMut(T) -> K,
    {
        GroupBy::new(self, callback)
    }

    fn hook_on_next<F>(self, callback: F) -> HookOnNext<Self, F>
    where
        Self: Observable<'or, 'sub, T, E>,
        F: for<'a> FnMut(T, Box<dyn FnOnce(T) + 'a>),
    {
        HookOnNext::new(self, callback)
    }

    fn hook_on_subscription<F>(self, callback: F) -> HookOnSubscription<Self, F>
    where
        Self: Observable<'or, 'sub, T, E>,
        F: FnOnce(Self, BoxedObserver<'or, T, E>) -> Subscription<'sub>,
    {
        HookOnSubscription::new(self, callback)
    }

    fn hook_on_termination<F>(self, callback: F) -> HookOnTermination<Self, F>
    where
        Self: Observable<'or, 'sub, T, E>,
        F: for<'a> FnOnce(Termination<E>, Box<dyn FnOnce(Termination<E>) + 'a>),
    {
        HookOnTermination::new(self, callback)
    }

    fn ignore_elements(self) -> IgnoreElements<Self> {
        IgnoreElements::new(self)
    }

    fn into_boxed<'oe>(self) -> BoxedObservable<'or, 'sub, 'oe, T, E>
    where
        T: 'or,
        E: 'or,
        Self: Observable<'or, 'sub, T, E> + Send + 'oe,
    {
        BoxedObservable::new(self)
    }

    fn into_stream(self) -> ObservableStream<'sub, T, Self>
    where
        Self: Observable<'or, 'sub, T, Infallible>,
    {
        ObservableStream::new(self)
    }

    fn last(self) -> Last<Self> {
        Last::new(self)
    }

    fn map<T1, F>(self, callback: F) -> Map<T, Self, F>
    where
        Self: Observable<'or, 'sub, T, E>,
        F: FnMut(T) -> T1,
    {
        Map::new(self, callback)
    }

    fn map_infallible_to_error<E1>(self) -> MapInfallibleToError<E1, Self> {
        MapInfallibleToError::new(self)
    }

    fn map_infallible_to_value<V1>(self) -> MapInfallibleToValue<V1, Self> {
        MapInfallibleToValue::new(self)
    }

    fn materialize(self) -> Materialize<Self> {
        Materialize::new(self)
    }

    fn merge_all<T1>(self) -> MergeAll<Self, T>
    where
        Self: Observable<'or, 'sub, T, E>,
        T: Observable<'or, 'sub, T1, E>,
    {
        MergeAll::new(self)
    }

    fn merge_with<OE2>(self, source_2: OE2) -> Merge<Self, OE2>
    where
        Self: Observable<'or, 'sub, T, E>,
        OE2: Observable<'or, 'sub, T, E>,
    {
        Merge::new(self, source_2)
    }

    fn multicast<S, F>(self, subject_maker: F) -> ConnectableObservable<Self, S>
    where
        F: FnOnce() -> S,
    {
        ConnectableObservable::new(self, subject_maker())
    }

    fn publish(self) -> ConnectableObservable<Self, PublishSubject<'or, T, E>> {
        self.multicast(PublishSubject::default)
    }

    fn publish_last(self) -> ConnectableObservable<Self, AsyncSubject<'or, T, E>> {
        self.multicast(AsyncSubject::default)
    }

    fn reduce<T0, F>(self, initial_value: T0, callback: F) -> Reduce<T0, T, Self, F>
    where
        Self: Observable<'or, 'sub, T, E>,
        F: FnMut(T0, T) -> T0,
    {
        Reduce::new(self, initial_value, callback)
    }

    fn replay(
        self,
        buffer_size: Option<usize>,
    ) -> ConnectableObservable<Self, ReplaySubject<'or, T, E>> {
        self.multicast(|| ReplaySubject::new(buffer_size))
    }

    fn sample<OE1>(self, sampler: OE1) -> Sample<Self, OE1>
    where
        Self: Observable<'or, 'sub, T, E>,
        OE1: Observable<'or, 'sub, (), E>,
    {
        Sample::new(self, sampler)
    }

    fn scan<T0, F>(self, initial_value: T0, callback: F) -> Scan<T0, T, Self, F>
    where
        Self: Observable<'or, 'sub, T, E>,
        F: FnMut(T0, T) -> T0,
    {
        Scan::new(self, initial_value, callback)
    }

    fn share(self) -> RefCount<'sub, Self, PublishSubject<'or, T, E>> {
        self.publish().ref_count()
    }

    fn share_last(self) -> RefCount<'sub, Self, AsyncSubject<'or, T, E>> {
        self.publish_last().ref_count()
    }
    fn share_replay(
        self,
        buffer_size: Option<usize>,
    ) -> RefCount<'sub, Self, ReplaySubject<'or, T, E>> {
        self.replay(buffer_size).ref_count()
    }

    fn skip(self, count: usize) -> Skip<Self> {
        Skip::new(self, count)
    }

    fn skip_last(self, count: usize) -> SkipLast<Self> {
        SkipLast::new(self, count)
    }

    fn start_with<OE0>(self, start: OE0) -> Concat<OE0, Self>
    where
        Self: Observable<'or, 'sub, T, E>,
        OE0: Observable<'or, 'sub, T, E>,
    {
        start.concat_with(self)
    }

    fn subscribe_with_callback<FN, FT>(self, on_next: FN, on_termination: FT) -> Subscription<'sub>
    where
        T: 'or,
        E: 'or,
        Self: Observable<'or, 'sub, T, E>,
        FN: FnMut(T) + Send + 'or,
        FT: FnOnce(Termination<E>) + Send + 'or,
    {
        self.subscribe(CallbackObserver::new(on_next, on_termination))
    }

    fn switch<T1>(self) -> Switch<Self, T>
    where
        Self: Observable<'or, 'sub, T, E>,
        T: Observable<'or, 'sub, T1, E>,
    {
        Switch::new(self)
    }

    fn switch_map<T1, OE1, F>(self, callback: F) -> SwitchMap<T, Self, OE1, F>
    where
        Self: Observable<'or, 'sub, T, E>,
        OE1: Observable<'or, 'sub, T1, E>,
        F: FnMut(T) -> OE1,
    {
        SwitchMap::new(self, callback)
    }

    fn take(self, count: usize) -> Take<Self> {
        Take::new(self, count)
    }

    fn take_last(self, count: usize) -> TakeLast<Self> {
        TakeLast::new(self, count)
    }

    fn take_until<T1, OE1>(self, stop: OE1) -> TakeUntil<T1, Self, OE1>
    where
        Self: Observable<'or, 'sub, T, E>,
        OE1: Observable<'or, 'sub, T1, E>,
    {
        TakeUntil::new(self, stop)
    }

    fn throttle<S>(self, time_span: Duration, scheduler: S) -> Throttle<Self, S> {
        Throttle::new(self, time_span, scheduler)
    }

    fn window<OE1>(self, boundary: OE1) -> Window<Self, OE1>
    where
        Self: Observable<'or, 'sub, T, E>,
        OE1: Observable<'or, 'sub, (), E>,
    {
        Window::new(self, boundary)
    }

    fn window_with_count(self, count: NonZeroUsize) -> WindowWithCount<Self> {
        WindowWithCount::new(self, count)
    }

    fn zip<T1, OE2>(self, another_source: OE2) -> Zip<Self, OE2>
    where
        Self: Observable<'or, 'sub, T, E>,
        OE2: Observable<'or, 'sub, T1, E>,
    {
        Zip::new(self, another_source)
    }
}

impl<'or, 'sub, T, E, OE> ObservableExt<'or, 'sub, T, E> for OE where OE: Observable<'or, 'sub, T, E>
{}
