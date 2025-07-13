use educe::Educe;
use rx_rust::{
    observer::{Observer, Termination},
    utils::types::{Mutable, MutableHelper, NecessarySend, Shared},
};
use std::sync::atomic::{AtomicBool, Ordering};

/// A helper struct for testing observables.
#[derive(Educe)]
#[educe(Debug, Clone)]
pub(crate) struct Checker<T, E> {
    values: Shared<Mutable<Vec<T>>>,
    termination: Shared<Mutable<Option<Termination<E>>>>,
    dropped: Shared<AtomicBool>,
}

impl<T, E> Checker<T, E> {
    pub(crate) fn new() -> (Self, CheckerObserver<T, E>) {
        let values = Shared::new(Mutable::new(Vec::new()));
        let termination = Shared::new(Mutable::new(None));
        let dropped = Shared::new(AtomicBool::new(false));
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

    pub(crate) fn values(&self) -> Vec<T>
    where
        T: Clone,
    {
        self.values.lock_ref().clone()
    }

    pub(crate) fn is_active(&self) -> bool {
        let termination = self.termination.lock_ref();
        let dropped = self.dropped.load(Ordering::SeqCst);
        termination.is_none() && !dropped
    }

    pub(crate) fn is_dropped(&self) -> bool {
        let termination = self.termination.lock_ref();
        let dropped = self.dropped.load(Ordering::SeqCst);
        termination.is_none() && dropped
    }

    pub(crate) fn is_error(&self, expected: E) -> bool
    where
        E: PartialEq,
    {
        let termination = self.termination.lock_ref();
        matches!(*termination, Some(Termination::Error(ref e)) if *e == expected)
    }

    pub(crate) fn is_completed(&self) -> bool {
        let termination = self.termination.lock_ref();
        matches!(*termination, Some(Termination::Completed))
    }
}

#[derive(Educe)]
#[educe(Debug)]
pub(crate) struct CheckerObserver<T, E> {
    values: Shared<Mutable<Vec<T>>>,
    termination: Shared<Mutable<Option<Termination<E>>>>,
    dropped: Shared<AtomicBool>,
}

impl<T, E> CheckerObserver<T, E> {
    pub(crate) fn into_callbacks(
        self,
    ) -> (
        impl FnMut(T) + NecessarySend + use<T, E>,
        impl FnOnce(Termination<E>) + NecessarySend + use<T, E>,
    )
    where
        T: NecessarySend,
        E: NecessarySend,
    {
        let values = self.values.clone();
        (
            move |value| {
                let mut values = values.lock_mut();
                values.push(value);
            },
            |termination| self.on_termination(termination),
        )
    }
}

impl<T, E> Drop for CheckerObserver<T, E> {
    fn drop(&mut self) {
        self.dropped.store(true, Ordering::SeqCst);
    }
}

impl<T, E> Observer<T, E> for CheckerObserver<T, E> {
    fn on_next(&mut self, value: T) {
        let mut values = self.values.lock_mut();
        values.push(value);
    }

    fn on_termination(self, termination: Termination<E>) {
        let mut termination_lock = self.termination.lock_mut();
        assert!(termination_lock.is_none());
        *termination_lock = Some(termination);
    }
}

#[cfg(feature = "futures")]
use {
    crate::tests_utils::test_runtime::TestRuntime, futures::Stream, futures::stream::StreamExt,
    rx_rust::disposable::subscription::Subscription, std::convert::Infallible,
};

#[cfg(feature = "futures")]
impl<T> Checker<T, Infallible> {
    pub(crate) fn from_stream(
        mut stream: impl Stream<Item = T> + NecessarySend + Unpin + 'static,
        runtime: TestRuntime,
    ) -> (Self, Subscription<'static>)
    where
        T: NecessarySend + 'static,
    {
        let values = Shared::new(Mutable::new(Vec::new()));
        let termination = Shared::new(Mutable::new(None));
        let dropped = Shared::new(AtomicBool::new(false));

        let values_cloned = values.clone();
        let termination_cloned = termination.clone();
        let handle = runtime.spawn(async move {
            while let Some(value) = stream.next().await {
                values_cloned.lock_mut().push(value);
            }
            termination_cloned
                .lock_mut()
                .replace(Termination::Completed);
        });
        let dropped_cloned = dropped.clone();
        (
            Self {
                values,
                termination,
                dropped,
            },
            Subscription::new_with_disposal_callback(move || {
                handle.abort();
                dropped_cloned.store(true, Ordering::SeqCst);
            }),
        )
    }
}
