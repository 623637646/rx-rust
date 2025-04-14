use super::Observable;
use crate::{
    observer::{Terminal, callback_observer::CallbackObserver},
    operators::{
        transforming::map::{Map, MapObserver},
        utility::delay::Delay,
    },
    subscription::Subscription,
};
use std::time::Duration;

pub trait ObservableExt: Sized {
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
        let observer = CallbackObserver::new(on_next, on_terminal);
        self.subscribe(observer)
    }

    fn map<'a, 'b, T, T2, E, F>(self, f: F) -> Map<Self, F, T>
    where
        F: FnMut(T) -> T2,
        Self: Observable<'a, T, E, MapObserver<'b, T2, E, F>>,
    {
        Map::new(self, f)
    }

    fn delay<S>(self, delay: Duration, scheduler: S) -> Delay<Self, S> {
        Delay::new(self, delay, scheduler)
    }
}

impl<OE> ObservableExt for OE {}
