use super::{Event, Observer};
use std::sync::RwLock;

/**
An observer that wraps a closure.

# Example
```rust
use rx_rust::observable::Observable;
use rx_rust::observer::anonymous_observer::AnonymousObserver;
use rx_rust::observer::event::Event;
use rx_rust::operators::just::Just;
use std::convert::Infallible;
let observable = Just::new(123);
let observer = AnonymousObserver::new(|e: Event<i32, Infallible>| {
    println!("{:?}", e);
});
observable.subscribe(observer);
```
*/

pub struct AnonymousObserver<F> {
    received_event: F,
    terminated: RwLock<bool>,
}

impl<F> AnonymousObserver<F> {
    pub fn new(on_event: F) -> AnonymousObserver<F> {
        AnonymousObserver {
            received_event: on_event,
            terminated: RwLock::new(false),
        }
    }
}

impl<T, E, F> Observer<T, E> for AnonymousObserver<F>
where
    F: Fn(Event<T, E>) + Sync + Send + 'static,
{
    fn on(&self, event: Event<T, E>) {
        (self.received_event)(event);
    }

    fn terminated(&self) -> bool {
        *self.terminated.read().unwrap()
    }

    fn set_terminated(&self, terminated: bool) {
        *self.terminated.write().unwrap() = terminated;
    }
}
