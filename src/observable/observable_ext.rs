use super::Observable;
use crate::{
    observer::{Termination, callback_observer::CallbackObserver},
    operators::{
        combining::{merge::Merge, switch::Switch},
        mathematical_aggregate::concat::Concat,
        others::{
            hook_on_next::HookOnNext, hook_on_termination::HookOnTermination,
            map_infallible_to_error::MapInfallibleToError, map_value_to_void::MapValueToVoid,
            observable_stream::ObservableStream,
        },
        transforming::{
            buffer::Buffer, buffer_with_count::BufferWithCount, buffer_with_time::BufferWithTime,
            buffer_with_time_or_count::BufferWithTimeOrCount, concat_map::ConcatMap,
            flat_map::FlatMap, group_by::GroupBy, map::Map, switch_map::SwitchMap,
        },
        utility::{
            delay::Delay, dematerialize::Dematerialize, do_on_next::DoOnNext,
            do_on_termination::DoOnTermination, materialize::Materialize,
        },
    },
    subscription::Subscription,
};
use std::{convert::Infallible, time::Duration};

pub trait ObservableExt: Sized {
    fn buffer<'or, 'sub, T, E, OE2>(self, boundary: OE2) -> Buffer<Self, OE2>
    where
        Self: Observable<'or, 'sub, T, E>,
        OE2: Observable<'or, 'sub, (), E>,
    {
        Buffer::new(self, boundary)
    }

    fn buffer_with_count(self, count: usize) -> BufferWithCount<Self> {
        BufferWithCount::new(self, count)
    }

    fn buffer_with_time<S>(
        self,
        time_pan: Duration,
        scheduler: S,
        delay: Option<Duration>,
    ) -> BufferWithTime<Self, S> {
        BufferWithTime::new(self, time_pan, scheduler, delay)
    }

    fn buffer_with_time_or_count<S>(
        self,
        count: usize,
        time_pan: Duration,
        scheduler: S,
        delay: Option<Duration>,
    ) -> BufferWithTimeOrCount<Self, S> {
        BufferWithTimeOrCount::new(self, count, time_pan, scheduler, delay)
    }

    fn concat<'or, 'sub, T, E, OE2>(self) -> Concat<Self, OE2>
    where
        Self: Observable<'or, 'sub, OE2, E>,
        OE2: Observable<'or, 'sub, T, E>,
    {
        Concat::new(self)
    }

    fn concat_map<'or, 'sub, T0, T, E, OE2, F>(self, callback: F) -> ConcatMap<T0, Self, OE2, F>
    where
        Self: Observable<'or, 'sub, T0, E>,
        OE2: Observable<'or, 'sub, T, E>,
        F: FnMut(T0) -> OE2,
    {
        ConcatMap::new(self, callback)
    }

    fn delay<S>(self, delay: Duration, scheduler: S) -> Delay<Self, S> {
        Delay::new(self, delay, scheduler)
    }

    fn dematerialize(self) -> Dematerialize<Self> {
        Dematerialize::new(self)
    }

    fn do_on_next<'or, 'sub, T, E, F>(self, callback: F) -> DoOnNext<Self, F>
    where
        Self: Observable<'or, 'sub, T, E>,
        F: FnMut(&T),
    {
        DoOnNext::new(self, callback)
    }

    fn do_on_termination<'or, 'sub, T, E, F>(self, callback: F) -> DoOnTermination<Self, F>
    where
        Self: Observable<'or, 'sub, T, E>,
        F: FnOnce(&Termination<E>),
    {
        DoOnTermination::new(self, callback)
    }

    fn flat_map<'or, 'sub, T0, T, E, OE2, F>(self, callback: F) -> FlatMap<T0, Self, OE2, F>
    where
        Self: Observable<'or, 'sub, T0, E>,
        OE2: Observable<'or, 'sub, T, E>,
        F: FnMut(T0) -> OE2,
    {
        FlatMap::new(self, callback)
    }

    fn group_by<'or, 'sub, T, E, F, K>(self, callback: F) -> GroupBy<Self, F, K>
    where
        Self: Observable<'or, 'sub, T, E>,
        F: FnMut(T) -> K,
    {
        GroupBy::new(self, callback)
    }

    fn hook_on_next<'or, 'sub, T, E, F>(self, callback: F) -> HookOnNext<Self, F>
    where
        Self: Observable<'or, 'sub, T, E>,
        F: for<'a> FnMut(T, Box<dyn FnOnce(T) + 'a>),
    {
        HookOnNext::new(self, callback)
    }

    fn hook_on_termination<'or, 'sub, T, E, F>(self, callback: F) -> HookOnTermination<Self, F>
    where
        Self: Observable<'or, 'sub, T, E>,
        F: for<'a> FnOnce(Termination<E>, Box<dyn FnOnce(Termination<E>) + 'a>),
    {
        HookOnTermination::new(self, callback)
    }

    fn into_stream<'or, 'sub, T>(self) -> ObservableStream<'sub, T, Self>
    where
        Self: Observable<'or, 'sub, T, Infallible>,
    {
        ObservableStream::new(self)
    }

    fn map<'or, 'sub, T0, T, E, F>(self, callback: F) -> Map<T0, Self, F>
    where
        Self: Observable<'or, 'sub, T0, E>,
        F: FnMut(T0) -> T,
    {
        Map::new(self, callback)
    }

    fn map_infallible_to_error(self) -> MapInfallibleToError<Self> {
        MapInfallibleToError::new(self)
    }

    fn map_value_to_void<'or, 'sub, T, E>(self) -> MapValueToVoid<T, Self>
    where
        Self: Observable<'or, 'sub, T, E>,
    {
        MapValueToVoid::new(self)
    }

    fn materialize(self) -> Materialize<Self> {
        Materialize::new(self)
    }

    fn merge<'or, 'sub, T, E, OE2>(self) -> Merge<Self, OE2>
    where
        Self: Observable<'or, 'sub, OE2, E>,
        OE2: Observable<'or, 'sub, T, E>,
    {
        Merge::new(self)
    }

    fn subscribe_with_callback<'or, 'sub, T, E, FN, FT>(
        self,
        on_next: FN,
        on_termination: FT,
    ) -> Subscription<'sub>
    where
        T: 'or,
        E: 'or,
        Self: Observable<'or, 'sub, T, E>,
        FN: FnMut(T) + Send + 'or,
        FT: FnOnce(Termination<E>) + Send + 'or,
    {
        self.subscribe(CallbackObserver::new(on_next, on_termination))
    }

    fn switch<'or, 'sub, T, E, OE2>(self) -> Switch<Self, OE2>
    where
        Self: Observable<'or, 'sub, OE2, E>,
        OE2: Observable<'or, 'sub, T, E>,
    {
        Switch::new(self)
    }

    fn switch_map<'or, 'sub, T0, T, E, OE2, F>(self, callback: F) -> SwitchMap<T0, Self, OE2, F>
    where
        Self: Observable<'or, 'sub, T0, E>,
        OE2: Observable<'or, 'sub, T, E>,
        F: FnMut(T0) -> OE2,
    {
        SwitchMap::new(self, callback)
    }
}
