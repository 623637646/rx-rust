use crate::observer::event::{Event, Terminated};
use crate::observer::Observer;
use std::sync::{Arc, RwLock};

/// A helper struct for testing observables.
#[derive(Debug, Clone)]
pub(crate) struct CheckingObserver<T, E> {
    events: Arc<RwLock<Vec<Event<T, E>>>>,
    terminated: Arc<RwLock<bool>>,
}

impl<T, E> CheckingObserver<T, E> {
    pub(crate) fn new() -> Self {
        CheckingObserver {
            events: Arc::new(RwLock::new(Vec::new())),
            terminated: Arc::new(RwLock::new(false)),
        }
    }

    pub(crate) fn is_values_matched(&self, expected: &[T]) -> bool
    where
        T: PartialEq,
    {
        let events = self.events.read().unwrap();
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
        let events = self.events.read().unwrap();
        !matches!(events.last(), Some(Event::Terminated(_)))
    }

    pub(crate) fn is_error(&self, expected: E) -> bool
    where
        E: PartialEq,
    {
        let events = self.events.read().unwrap();
        if let Some(Event::Terminated(Terminated::Error(error))) = events.last() {
            error == &expected
        } else {
            false
        }
    }

    pub(crate) fn is_unsubscribed(&self) -> bool {
        let events = self.events.read().unwrap();
        matches!(
            events.last(),
            Some(Event::Terminated(Terminated::Unsubscribed))
        )
    }

    pub(crate) fn is_completed(&self) -> bool {
        let events = self.events.read().unwrap();
        matches!(
            events.last(),
            Some(Event::Terminated(Terminated::Completed))
        )
    }
}

impl<T, E> Observer<T, E> for CheckingObserver<T, E>
where
    T: Sync + Send + 'static,
    E: Sync + Send + 'static,
{
    fn received(&self, event: Event<T, E>) {
        let mut events = self.events.write().unwrap();
        if let Some(Event::Terminated(_)) = events.last() {
            panic!("ObservableCounter is terminated");
        }
        events.push(event);
    }

    fn terminated(&self) -> bool {
        *self.terminated.read().unwrap()
    }

    fn set_terminated(&self, terminated: bool) {
        *self.terminated.write().unwrap() = terminated;
    }
}
