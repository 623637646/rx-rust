use crate::observer::{Observer, Terminal};
use std::sync::{Arc, RwLock};

/// A helper struct for testing observables.
#[derive(Debug, Clone)]
pub(crate) struct CheckingObserver<T, E> {
    values: Arc<RwLock<Vec<T>>>,
    terminal: Arc<RwLock<Option<Terminal<E>>>>,
}

impl<T, E> CheckingObserver<T, E> {
    pub(crate) fn new() -> Self {
        CheckingObserver {
            values: Arc::new(RwLock::new(Vec::new())),
            terminal: Arc::new(RwLock::new(None)),
        }
    }

    pub(crate) fn is_values_matched(&self, expected: &[T]) -> bool
    where
        T: PartialEq,
    {
        let values = self.values.read().unwrap();
        *values == expected
    }

    pub(crate) fn is_unterminated(&self) -> bool {
        let terminal = self.terminal.read().unwrap();
        terminal.is_none()
    }

    pub(crate) fn is_error(&self, expected: E) -> bool
    where
        E: PartialEq,
    {
        let terminal = self.terminal.read().unwrap();
        matches!(*terminal, Some(Terminal::Error(ref e)) if *e == expected)
    }

    pub(crate) fn is_completed(&self) -> bool {
        let terminal = self.terminal.read().unwrap();
        matches!(*terminal, Some(Terminal::Completed))
    }

    pub(crate) fn fn_for_subscribe_on(
        &self,
    ) -> (
        impl FnMut(T) + Send + use<T, E>,
        impl FnOnce(Terminal<E>) + Send + use<T, E>,
    )
    where
        T: Send + Sync + Clone,
        E: Send + Sync + Clone,
    {
        let mut checker_cloned_1 = self.clone();
        let checker_cloned_2 = self.clone();
        (
            move |value| checker_cloned_1.on_next(value),
            |terminal| checker_cloned_2.on_terminal(terminal),
        )
    }
}

impl<T, E> Observer<T, E> for CheckingObserver<T, E> {
    fn on_next(&mut self, value: T) {
        let mut values = self.values.write().unwrap();
        values.push(value);
    }

    fn on_terminal(self, terminal: Terminal<E>) {
        let mut terminal_lock = self.terminal.write().unwrap();
        assert!(terminal_lock.is_none());
        *terminal_lock = Some(terminal);
    }
}
