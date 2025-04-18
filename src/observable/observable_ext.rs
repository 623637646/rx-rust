use super::Observable;
use crate::{
    observer::{Terminal, callback_observer::CallbackObserver},
    operators::{
        transforming::{
            buffer::Buffer,
            buffer_with_count::BufferWithCount,
            map::{Map, MapObserver},
        },
        utility::delay::Delay,
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

    fn delay<S>(self, delay: Duration, scheduler: S) -> Delay<Self, S> {
        Delay::new(self, delay, scheduler)
    }

    fn map<'a, 'b, T0, T, E, F>(self, f: F) -> Map<T0, Self, F>
    where
        F: FnMut(T0) -> T,
        Self: Observable<'a, T0, E, MapObserver<'b, T, E, F>>,
    {
        Map::new(self, f)
    }

    fn subscribe_with_callback<'a, 'b, T, E, FN, FT>(
        self,
        on_next: FN,
        on_terminal: FT,
    ) -> Subscription<'a>
    where
        Self: Observable<'a, T, E, CallbackObserver<'b, T, E>>,
        FN: FnMut(T) + Send + 'b,
        FT: FnOnce(Terminal<E>) + Send + 'b,
    {
        self.subscribe(CallbackObserver::new(on_next, on_terminal))
    }
}

impl<OE> ObservableExt for OE {}
