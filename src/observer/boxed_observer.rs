use super::{Observer, Termination};

/// TODO: doc
/// https://stackoverflow.com/a/56447952/9315497
pub struct BoxedObserver<'or, T, E>(Box<dyn FnMut(HandleEvent<T, E>) + Send + 'or>);

enum HandleEvent<T, E> {
    Next(T),
    Termination(Termination<E>),
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
            HandleEvent::Termination(termination) => {
                if let Some(observer) = observer.take() {
                    observer.on_termination(termination);
                }
            }
        }))
    }
}

impl<T, E> Observer<T, E> for BoxedObserver<'_, T, E> {
    fn on_next(&mut self, value: T) {
        self.0(HandleEvent::Next(value));
    }

    fn on_termination(mut self, termination: Termination<E>) {
        self.0(HandleEvent::Termination(termination));
    }
}
