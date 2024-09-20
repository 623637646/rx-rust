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
        *self.values.read().unwrap() == expected
    }

    pub(crate) fn is_unterminated(&self) -> bool {
        !self.terminal.read().unwrap().is_some()
    }

    pub(crate) fn is_error(&self, expected: E) -> bool
    where
        E: PartialEq,
    {
        let terminal = self.terminal.read().unwrap();
        matches!(terminal.as_ref(), Some(Terminal::Error(e)) if e == &expected)
    }

    pub(crate) fn is_completed(&self) -> bool {
        let terminal = self.terminal.read().unwrap();
        matches!(terminal.as_ref(), Some(Terminal::Completed))
    }
}

impl<T, E> Observer<T, E> for CheckingObserver<T, E>
where
    T: Sync + Send + 'static,
    E: Sync + Send + 'static,
{
    fn on_next(&mut self, value: T) {
        self.values.write().unwrap().push(value);
    }

    fn on_terminal(self: Box<Self>, terminal: Terminal<E>) {
        let mut terminal_guard = self.terminal.write().unwrap();
        assert!(terminal_guard.is_none());
        *terminal_guard = Some(terminal);
    }
}
