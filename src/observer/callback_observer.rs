use super::{Observer, Terminal};

pub struct CallbackObserver<'cb, T, E> {
    on_next: Box<dyn FnMut(T) + Send + 'cb>,
    on_terminal: Box<dyn FnOnce(Terminal<E>) + Send + 'cb>,
}

impl<'cb, T, E> CallbackObserver<'cb, T, E> {
    pub fn new<FN, FT>(on_next: FN, on_terminal: FT) -> Self
    where
        FN: FnMut(T) + Send + 'cb,
        FT: FnOnce(Terminal<E>) + Send + 'cb,
    {
        Self {
            on_next: Box::new(on_next),
            on_terminal: Box::new(on_terminal),
        }
    }
}

impl<T, E> Observer<T, E> for CallbackObserver<'_, T, E> {
    fn on_next(&mut self, value: T) {
        (self.on_next)(value);
    }

    fn on_terminal(self, terminal: Terminal<E>) {
        (self.on_terminal)(terminal);
    }
}
