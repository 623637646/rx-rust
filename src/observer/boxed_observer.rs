use super::{Event, Observer, Termination};
use crate::utils::types::NecessarySend;

cfg_if::cfg_if! {
    if #[cfg(feature = "single-threaded")] {
        /// Type-erased observer for single-threaded builds to handle this problem https://stackoverflow.com/q/46620790/9315497
        pub struct BoxedObserver<'or, T, E>(Box<dyn FnMut(Event<T, E>) + 'or>);
    } else {
        /// Type-erased observer for multi-threaded builds to handle this problem https://stackoverflow.com/q/46620790/9315497
        pub struct BoxedObserver<'or, T, E>(Box<dyn FnMut(Event<T, E>) + Send + 'or>);
    }
}

impl<'or, T, E> BoxedObserver<'or, T, E> {
    pub fn new(observer: impl Observer<T, E> + NecessarySend + 'or) -> Self {
        let mut observer = Some(observer);
        Self(Box::new(move |event| match event {
            Event::Next(value) => {
                if let Some(observer) = &mut observer {
                    observer.on_next(value);
                }
            }
            Event::Termination(termination) => {
                if let Some(observer) = observer.take() {
                    observer.on_termination(termination);
                }
            }
        }))
    }
}

impl<T, E> Observer<T, E> for BoxedObserver<'_, T, E> {
    fn on_next(&mut self, value: T) {
        self.0(Event::Next(value));
    }

    fn on_termination(mut self, termination: Termination<E>) {
        self.0(Event::Termination(termination));
    }
}
