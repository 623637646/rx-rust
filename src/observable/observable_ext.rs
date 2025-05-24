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
            flat_map::FlatMap, map::Map, switch_map::SwitchMap,
        },
        utility::{
            delay::Delay, dematerialize::Dematerialize, do_on_next::DoOnNext,
            do_on_termination::DoOnTermination, materialize::Materialize,
        },
    },
    subscription::Subscription,
};
use std::{convert::Infallible, time::Duration};

pub trait ObservableExt<'or, 'sub, T, E, OE>: Sized {
    fn buffer<OE2>(self, boundary: OE2) -> Buffer<Self, OE2>
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

    fn concat<T1>(self) -> Concat<Self, T>
    where
        Self: Observable<'or, 'sub, T, E>,
        T: Observable<'or, 'sub, T1, E>,
    {
        Concat::new(self)
    }

    fn concat_map<T1, OE2, F>(self, callback: F) -> ConcatMap<T, Self, OE2, F>
    where
        Self: Observable<'or, 'sub, T, E>,
        OE2: Observable<'or, 'sub, T1, E>,
        F: FnMut(T) -> OE2,
    {
        ConcatMap::new(self, callback)
    }

    fn delay<S>(self, delay: Duration, scheduler: S) -> Delay<Self, S> {
        Delay::new(self, delay, scheduler)
    }

    fn dematerialize(self) -> Dematerialize<Self> {
        Dematerialize::new(self)
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

    fn flat_map<T1, OE2, F>(self, callback: F) -> FlatMap<T, Self, OE2, F>
    where
        Self: Observable<'or, 'sub, T, E>,
        OE2: Observable<'or, 'sub, T1, E>,
        F: FnMut(T) -> OE2,
    {
        FlatMap::new(self, callback)
    }

    fn hook_on_next<F>(self, callback: F) -> HookOnNext<Self, F>
    where
        Self: Observable<'or, 'sub, T, E>,
        F: for<'a> FnMut(T, Box<dyn FnOnce(T) + 'a>),
    {
        HookOnNext::new(self, callback)
    }

    fn hook_on_termination<F>(self, callback: F) -> HookOnTermination<Self, F>
    where
        Self: Observable<'or, 'sub, T, E>,
        F: for<'a> FnOnce(Termination<E>, Box<dyn FnOnce(Termination<E>) + 'a>),
    {
        HookOnTermination::new(self, callback)
    }

    fn into_stream(self) -> ObservableStream<'sub, T, Self>
    where
        Self: Observable<'or, 'sub, T, Infallible>,
    {
        ObservableStream::new(self)
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

    fn map_value_to_void(self) -> MapValueToVoid<T, Self>
    where
        Self: Observable<'or, 'sub, T, E>,
    {
        MapValueToVoid::new(self)
    }

    fn materialize(self) -> Materialize<Self> {
        Materialize::new(self)
    }

    fn merge<T1>(self) -> Merge<Self, T>
    where
        Self: Observable<'or, 'sub, T, E>,
        T: Observable<'or, 'sub, T1, E>,
    {
        Merge::new(self)
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

    fn switch_map<T1, OE2, F>(self, callback: F) -> SwitchMap<T, Self, OE2, F>
    where
        Self: Observable<'or, 'sub, T, E>,
        OE2: Observable<'or, 'sub, T1, E>,
        F: FnMut(T) -> OE2,
    {
        SwitchMap::new(self, callback)
    }
}

impl<'or, 'sub, T, E, OE> ObservableExt<'or, 'sub, T, E, OE> for OE where
    OE: Observable<'or, 'sub, T, E>
{
}
