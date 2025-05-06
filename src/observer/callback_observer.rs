use super::{Observer, Termination};

pub(crate) struct CallbackObserver<'cb, T, E> {
    on_next: Box<dyn FnMut(T) + Send + 'cb>,
    on_termination: Box<dyn FnOnce(Termination<E>) + Send + 'cb>,
}

impl<'cb, T, E> CallbackObserver<'cb, T, E> {
    pub fn new<FN, FT>(on_next: FN, on_termination: FT) -> Self
    where
        FN: FnMut(T) + Send + 'cb,
        FT: FnOnce(Termination<E>) + Send + 'cb,
    {
        Self {
            on_next: Box::new(on_next),
            on_termination: Box::new(on_termination),
        }
    }
}

impl<T, E> Observer<T, E> for CallbackObserver<'_, T, E> {
    fn on_next(&mut self, value: T) {
        (self.on_next)(value);
    }

    fn on_termination(self, termination: Termination<E>) {
        (self.on_termination)(termination);
    }
}
