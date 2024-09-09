#[cfg(test)]
use crate::observer::Event;
use crate::observer::{Observer, Terminated};
use std::{cell::RefCell, rc::Rc};

/// TODO: ObservableCounter is not atomic, implement struct AtomicObservableCounter {}?

/// A helper struct for testing observables.
#[derive(Debug, Clone)]
pub(crate) struct CheckingObserver<T, E> {
    events: Rc<RefCell<Vec<Event<T, E>>>>,
}

impl<T, E> CheckingObserver<T, E> {
    pub(crate) fn new() -> Self {
        CheckingObserver {
            events: Rc::new(RefCell::new(Vec::new())),
        }
    }

    pub(crate) fn is_values_matched(&self, expected: &[T]) -> bool
    where
        T: PartialEq,
    {
        let events = self.events.borrow();
        let values: Vec<&T> = events
            .iter()
            .filter_map(|event| match event {
                Event::Next(value) => Some(value),
                _ => None,
            })
            .collect();
        values == expected.iter().collect::<Vec<_>>()
    }

    pub(crate) fn is_unterminated(&self) -> bool {
        let events = self.events.borrow();
        !matches!(events.last(), Some(Event::Terminated(_)))
    }

    pub(crate) fn is_error(&self, expected: &E) -> bool
    where
        E: PartialEq,
    {
        let events = self.events.borrow();
        if let Some(Event::Terminated(Terminated::Error(error))) = events.last() {
            error == expected
        } else {
            false
        }
    }

    pub(crate) fn is_cancelled(&self) -> bool {
        let events = self.events.borrow();
        matches!(
            events.last(),
            Some(Event::Terminated(Terminated::Cancelled))
        )
    }

    pub(crate) fn is_completed(&self) -> bool {
        let events = self.events.borrow();
        matches!(
            events.last(),
            Some(Event::Terminated(Terminated::Completed))
        )
    }
}

impl<T, E> Observer<T, E> for CheckingObserver<T, E> {
    fn on(&self, event: Event<T, E>) {
        let mut events = self.events.borrow_mut();
        if let Some(Event::Terminated(_)) = events.last() {
            panic!("ObservableCounter is terminated");
        }
        events.push(event);
    }
}
