use super::{Observer, Terminal};

/// TODO: doc
/// https://stackoverflow.com/a/56447952/9315497
pub struct BoxedObserver<'a, T, E> {
    handle: Box<dyn FnMut(HandleEvent<T, E>) + Send + 'a>,
}

enum HandleEvent<T, E> {
    Value(T),
    Terminal(Terminal<E>),
}

impl<'a, T, E> BoxedObserver<'a, T, E> {
    pub fn new(observer: impl Observer<T, E> + Send + 'a) -> Self {
        let mut observer = Some(observer);
        BoxedObserver {
            handle: Box::new(move |event| match event {
                HandleEvent::Value(value) => {
                    if let Some(observer) = &mut observer {
                        observer.on_next(value);
                    }
                }
                HandleEvent::Terminal(terminal) => {
                    if let Some(observer) = observer.take() {
                        observer.on_terminal(terminal);
                    }
                }
            }),
        }
    }
}

impl<T, E> Observer<T, E> for BoxedObserver<'_, T, E> {
    fn on_next(&mut self, value: T) {
        (self.handle)(HandleEvent::Value(value));
    }

    fn on_terminal(mut self, terminal: Terminal<E>) {
        (self.handle)(HandleEvent::Terminal(terminal));
    }
}
