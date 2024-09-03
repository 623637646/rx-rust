#[cfg(test)]
use crate::observer::Event;
use crate::observer::{Observer, Terminated};
use std::{cell::RefCell, rc::Rc};

// TODO: Use Arc instead of Rc? checking in multiple threads? for whole project?

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
        values == expected.iter().collect::<Vec<&T>>()
    }

    // TODO: There is no situation to use this function, so remove it.
    // pub(crate) fn is_values_matched(&self, expected: &[T]) -> bool
    // where
    //     T: PartialEq,
    // {
    //     let events = self.events.borrow();
    //     let values: Vec<&T> = events
    //         .iter()
    //         .filter_map(|event| match event {
    //             Event::Next(value) => Some(value),
    //             _ => None,
    //         })
    //         .collect();
    //     values == expected.iter().collect::<Vec<&T>>()
    // }

    // pub(crate) fn is_values_ptr_matched(&self, expected: &[&T]) -> bool {
    //     let events = self.events.borrow();
    //     let values: Vec<&T> = events
    //         .iter()
    //         .filter_map(|event| match event {
    //             Event::Next(value) => Some(value),
    //             _ => None,
    //         })
    //         .collect();
    //     if values.len() != expected.len() {
    //         return false;
    //     }
    //     for (a, b) in values.iter().zip(expected.iter()) {
    //         if !std::ptr::eq(*a, *b) {
    //             return false;
    //         }
    //     }
    //     true
    // }

    pub(crate) fn is_terminated(&self) -> bool {
        let events = self.events.borrow();
        matches!(events.last(), Some(Event::Terminated(_)))
    }

    pub(crate) fn is_terminals_matched(&self, expected: &Terminated<E>) -> bool
    where
        E: PartialEq,
    {
        let events = self.events.borrow();
        if let Some(Event::Terminated(terminated)) = events.last() {
            terminated == expected
        } else {
            false
        }
    }
}

impl<T, E> Observer<T, E> for ObservableChecker<T, E> {
    fn on(&self, event: Event<T, E>) {
        let mut events = self.events.borrow_mut();
        if let Some(Event::Terminated(_)) = events.last() {
            panic!("ObservableCounter is terminated");
        }
        events.push(event);
    }
}

// TODO: ObservableCounter is not atomic, implement struct AtomicObservableCounter {}?
