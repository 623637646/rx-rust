use super::Observable;
use crate::{
    observer::{Terminal, callback_observer::CallbackObserver},
    operators::{
        others::{
            map_infallible_to_error::MapInfallibleToError, map_value_to_void::MapValueToVoid,
        },
        transforming::{
            buffer::Buffer,
            buffer_with_count::BufferWithCount,
            buffer_with_time::BufferWithTime,
            map::{Map, MapObserver},
        },
        utility::{
            delay::Delay,
            do_on_next::{DoOnNext, DoOnNextObserver},
            do_on_terminal::{DoOnTerminal, DoOnTerminalObserver},
        },
    },
    subscription::Subscription,
};
use std::time::Duration;

pub trait ObservableExt: Sized {
    fn buffer<OE>(self, boundary: OE) -> Buffer<Self, OE> {
        Buffer::new(self, boundary)
    }

    fn buffer_with_count(self, count: usize) -> BufferWithCount<Self> {
        BufferWithCount::new(self, count)
    }

    fn buffer_with_time<S>(self, time_pan: Duration, scheduler: S) -> BufferWithTime<Self, S> {
        BufferWithTime::new(self, time_pan, scheduler)
    }

    fn delay<S>(self, delay: Duration, scheduler: S) -> Delay<Self, S> {
        Delay::new(self, delay, scheduler)
    }

    fn do_on_next<'sub, 'b, T, E, F>(self, callback: F) -> DoOnNext<Self, F>
    where
        F: FnMut(&T),
        Self: Observable<'sub, T, E, DoOnNextObserver<'b, T, E, F>>,
    {
        DoOnNext::new(self, callback)
    }

    fn do_on_terminal<'sub, 'b, T, E, F>(self, callback: F) -> DoOnTerminal<Self, F>
    where
        F: FnOnce(&Terminal<E>),
        Self: Observable<'sub, T, E, DoOnTerminalObserver<'b, T, E, F>>,
    {
        DoOnTerminal::new(self, callback)
    }

    fn map<'sub, 'b, T0, T, E, F>(self, f: F) -> Map<T0, Self, F>
    where
        F: FnMut(T0) -> T,
        Self: Observable<'sub, T0, E, MapObserver<'b, T, E, F>>,
    {
        Map::new(self, f)
    }

    fn map_infallible_to_error(self) -> MapInfallibleToError<Self> {
        MapInfallibleToError::new(self)
    }

    fn map_value_to_void<T>(self) -> MapValueToVoid<T, Self> {
        MapValueToVoid::new(self)
    }

    fn subscribe_with_callback<'sub, 'b, T, E, FN, FT>(
        self,
        on_next: FN,
        on_terminal: FT,
    ) -> Subscription<'sub>
    where
        Self: Observable<'sub, T, E, CallbackObserver<'b, T, E>>,
        FN: FnMut(T) + Send + 'b,
        FT: FnOnce(Terminal<E>) + Send + 'b,
    {
        self.subscribe(CallbackObserver::new(on_next, on_terminal))
    }
}

impl<OE> ObservableExt for OE {}
