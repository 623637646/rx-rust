use educe::Educe;
use rx_rust::observer::{Observer, Termination};
use std::sync::{Arc, RwLock};

/// A helper struct for testing observables.
#[derive(Educe)]
#[educe(Debug, Clone)]
pub(crate) struct Checker<T, E> {
    values: Arc<RwLock<Vec<T>>>,
    termination: Arc<RwLock<Option<Termination<E>>>>,
    dropped: Arc<RwLock<bool>>,
}

impl<T, E> Checker<T, E> {
    pub(crate) fn new() -> (Self, CheckerObserver<T, E>) {
        let values = Arc::new(RwLock::new(Vec::new()));
        let termination = Arc::new(RwLock::new(None));
        let dropped = Arc::new(RwLock::new(false));
        (
            Self {
                values: values.clone(),
                termination: termination.clone(),
                dropped: dropped.clone(),
            },
            CheckerObserver {
                values,
                termination,
                dropped,
            },
        )
    }

    pub(crate) fn is_values_matched(&self, expected: &[T]) -> bool
    where
        T: PartialEq,
    {
        let values = self.values.read().unwrap();
        *values == expected
    }

    pub(crate) fn is_active(&self) -> bool {
        let termination = self.termination.read().unwrap();
        let dropped = self.dropped.read().unwrap();
        termination.is_none() && !*dropped
    }

    pub(crate) fn is_dropped(&self) -> bool {
        let termination = self.termination.read().unwrap();
        let dropped = self.dropped.read().unwrap();
        termination.is_none() && *dropped
    }

    pub(crate) fn is_error(&self, expected: E) -> bool
    where
        E: PartialEq,
    {
        let termination = self.termination.read().unwrap();
        matches!(*termination, Some(Termination::Error(ref e)) if *e == expected)
    }

    pub(crate) fn is_completed(&self) -> bool {
        let termination = self.termination.read().unwrap();
        matches!(*termination, Some(Termination::Completed))
    }
}

#[derive(Educe)]
#[educe(Debug)]
pub(crate) struct CheckerObserver<T, E> {
    values: Arc<RwLock<Vec<T>>>,
    termination: Arc<RwLock<Option<Termination<E>>>>,
    dropped: Arc<RwLock<bool>>,
}

impl<T, E> CheckerObserver<T, E> {
    pub(crate) fn into_callbacks(
        self,
    ) -> (
        impl FnMut(T) + Send + use<T, E>,
        impl FnOnce(Termination<E>) + Send + use<T, E>,
    )
    where
        T: Send + Sync,
        E: Send + Sync,
    {
        let values = self.values.clone();
        (
            move |value| {
                let mut values = values.write().unwrap();
                values.push(value);
            },
            |termination| self.on_termination(termination),
        )
    }
}

impl<T, E> Drop for CheckerObserver<T, E> {
    fn drop(&mut self) {
        *self.dropped.write().unwrap() = true;
    }
}

impl<T, E> Observer<T, E> for CheckerObserver<T, E> {
    fn on_next(&mut self, value: T) {
        let mut values = self.values.write().unwrap();
        values.push(value);
    }

    fn on_termination(self, termination: Termination<E>) {
        let mut termination_lock = self.termination.write().unwrap();
        assert!(termination_lock.is_none());
        *termination_lock = Some(termination);
    }
}
