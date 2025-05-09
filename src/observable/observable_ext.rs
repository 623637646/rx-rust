use super::Observable;
use crate::{
    observer::{Termination, callback_observer::CallbackObserver},
    operators::{
        combining::merge_all::MergeAll,
        others::{
            hook_on_next::HookOnNext, hook_on_termination::HookOnTermination,
            map_infallible_to_error::MapInfallibleToError, map_value_to_void::MapValueToVoid,
            observable_stream::ObservableStream,
        },
        transforming::{
            buffer::Buffer, buffer_with_count::BufferWithCount, buffer_with_time::BufferWithTime,
            buffer_with_time_or_count::BufferWithTimeOrCount, map::Map,
        },
        utility::{delay::Delay, do_on_next::DoOnNext, do_on_termination::DoOnTermination},
    },
    subscription::Subscription,
};
use std::{convert::Infallible, time::Duration};

pub trait ObservableExt: Sized {
    fn buffer<OE>(self, boundary: OE) -> Buffer<Self, OE> {
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

    fn delay<S>(self, delay: Duration, scheduler: S) -> Delay<Self, S> {
        Delay::new(self, delay, scheduler)
    }

    fn do_on_next<'or, 'sub, T, E, F>(self, callback: F) -> DoOnNext<Self, F>
    where
        Self: Observable<'or, 'sub, T, E>,
        F: FnMut(&T),
    {
        DoOnNext::new(self, callback)
    }

    fn do_on_termination<'sub, 'or, T, E, F>(self, callback: F) -> DoOnTermination<Self, F>
    where
        Self: Observable<'or, 'sub, T, E>,
        F: FnOnce(&Termination<E>),
    {
        DoOnTermination::new(self, callback)
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

    fn map<'sub, 'or, T0, T, E, F>(self, callback: F) -> Map<T0, Self, F>
    where
        Self: Observable<'or, 'sub, T0, E>,
        F: FnMut(T0) -> T,
    {
        Map::new(self, callback)
    }

    fn map_infallible_to_error(self) -> MapInfallibleToError<Self> {
        MapInfallibleToError::new(self)
    }

    fn map_value_to_void<T>(self) -> MapValueToVoid<T, Self> {
        MapValueToVoid::new(self)
    }

    fn merge_all<'or, 'sub, T, E, OE2>(self) -> MergeAll<Self, OE2>
    where
        Self: Observable<'or, 'sub, OE2, E>,
        OE2: Observable<'or, 'sub, T, E>,
    {
        MergeAll::new(self)
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
}
