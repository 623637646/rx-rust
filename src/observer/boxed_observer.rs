use super::{Observer, Terminal};
use std::sync::{Arc, Mutex};

/// TODO: doc
/// https://stackoverflow.com/a/56447952/9315497
pub struct BoxedObserver<'a, T, E> {
    on_next: Box<dyn FnMut(T) + Send + 'a>,
    on_terminal: Box<dyn FnOnce(Terminal<E>) + Send + 'a>,
}

impl<'a, T, E> BoxedObserver<'a, T, E> {
    pub fn new(observer: impl Observer<T, E> + Send + 'a) -> Self {
        let observer = Arc::new(Mutex::new(Some(observer)));
        let observer_cloned = observer.clone();
        BoxedObserver {
            on_next: Box::new(move |value| {
                if let Some(observer) = observer.lock().unwrap().as_mut() {
                    observer.on_next(value)
                }
            }),
            on_terminal: Box::new(move |terminal| {
                if let Some(observer) = observer_cloned.lock().unwrap().take() {
                    observer.on_terminal(terminal)
                }
            }),
        }
    }
}

impl<T, E> Observer<T, E> for BoxedObserver<'_, T, E> {
    fn on_next(&mut self, value: T) {
        (self.on_next)(value);
    }

    fn on_terminal(self, terminal: Terminal<E>) {
        (self.on_terminal)(terminal);
    }
}
