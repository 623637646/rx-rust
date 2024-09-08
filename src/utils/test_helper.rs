#[cfg(test)]
use crate::observer::Event;
use crate::observer::{Observer, Terminated};
use std::{cell::RefCell, rc::Rc};

/// TODO:
/// ObservableCounter is not atomic, implement struct AtomicObservableCounter {}?
/// Use Arc instead of Rc? checking in multiple threads? for whole project?

/// A helper struct for testing observables.
#[derive(Debug, Clone)]
pub(crate) struct ObservableChecker<T, E> {
    events: Rc<RefCell<Vec<Event<T, E>>>>,
}

impl<T, E> ObservableChecker<T, E> {
    pub(crate) fn new() -> Self {
        ObservableChecker {
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

impl<T, E> Observer<T, E> for ObservableChecker<T, E> {
    fn on(&mut self, event: Event<T, E>) {
        let mut events = self.events.borrow_mut(); // TODO: remove borrow_mut()?
        if let Some(Event::Terminated(_)) = events.last() {
            panic!("ObservableCounter is terminated");
        }
        match event {
            Event::Next(value) => events.push(Event::Next(value)),
            Event::Terminated(terminated) => match terminated {
                Terminated::Error(error) => {
                    events.push(Event::Terminated(Terminated::Error(error)))
                }
                Terminated::Cancelled => events.push(Event::Terminated(Terminated::Cancelled)),
                Terminated::Completed => events.push(Event::Terminated(Terminated::Completed)),
            },
        }
    }
}

// TODO:
// #[derive(Debug, PartialEq)]
// struct NonCloneableStruct {
//     value: i32,
// }

// impl NonCloneableStruct {
//     fn new() -> Self {
//         NonCloneableStruct { value: 33 }
//     }
// }
