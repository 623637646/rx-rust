use super::{Event, Observer};

/**
An observer that wraps a closure.

Example:
```rust
use rx_rust::observable::Observable;
use rx_rust::observer::anonymous_observer::AnonymousObserver;
use rx_rust::observer::Event;
use rx_rust::operators::just::Just;
use rx_rust::utils::never::Never;
let observable = Just::new(123);
let observer = AnonymousObserver::new(|e: Event<&i32, Never>| {
    println!("{:?}", e);
});
observable.subscribe(observer);
```
*/

pub struct AnonymousObserver<F> {
    on_event: F,
}

impl<F> AnonymousObserver<F> {
    pub fn new(on_event: F) -> AnonymousObserver<F> {
        AnonymousObserver { on_event }
    }
}

impl<T, E, F> Observer<T, E> for AnonymousObserver<F>
where
    F: Fn(Event<T, E>),
{
    fn on(&self, event: Event<T, E>) {
        (self.on_event)(event);
    }
}
