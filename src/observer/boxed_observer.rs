use super::{Observer, Terminal};

/// TODO: doc
/// https://stackoverflow.com/a/56447952/9315497
pub struct BoxedObserver<'or, T, E>(Box<dyn FnMut(HandleEvent<T, E>) + Send + 'or>);

enum HandleEvent<T, E> {
    Next(T),
    Terminal(Terminal<E>),
}

impl<'or, T, E> BoxedObserver<'or, T, E> {
    pub fn new(observer: impl Observer<T, E> + Send + 'or) -> Self {
        let mut observer = Some(observer);
        Self(Box::new(move |event| match event {
            HandleEvent::Next(value) => {
                if let Some(observer) = &mut observer {
                    observer.on_next(value);
                }
            }
            HandleEvent::Terminal(terminal) => {
                if let Some(observer) = observer.take() {
                    observer.on_terminal(terminal);
                }
            }
        }))
    }
}

impl<T, E> Observer<T, E> for BoxedObserver<'_, T, E> {
    fn on_next(&mut self, value: T) {
        self.0(HandleEvent::Next(value));
    }

    fn on_terminal(mut self, terminal: Terminal<E>) {
        self.0(HandleEvent::Terminal(terminal));
    }
}
