use super::{Observer, Terminal};

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

pub struct AnonymousObserver<FN, FT> {
    on_next: FN,
    on_terminal: FT,
}

impl<FN, FT> AnonymousObserver<FN, FT> {
    pub fn new(on_next: FN, on_terminal: FT) -> AnonymousObserver<FN, FT> {
        AnonymousObserver {
            on_next,
            on_terminal,
        }
    }
}

impl<T, E, FN, FT> Observer<T, E> for AnonymousObserver<FN, FT>
where
    FN: FnMut(T),
    FT: FnOnce(Terminal<E>),
{
    fn on_next(&mut self, value: T) {
        (self.on_next)(value);
    }

    fn on_terminal(self, terminal: Terminal<E>) {
        (self.on_terminal)(terminal);
    }
}
