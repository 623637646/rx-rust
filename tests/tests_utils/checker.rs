use educe::Educe;
use futures::Stream;
use futures::stream::StreamExt;
use rx_rust::{
    observer::{Event, Observer, Termination},
    subscription::disposable::{CallbackDisposal, Disposable},
};
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

    pub(crate) fn from_stream(
        stream: impl Stream<Item = Event<T, E>> + Send + Unpin + 'static,
    ) -> (Self, impl Disposable + Send + 'static)
    where
        T: Clone + Send + Sync + 'static,
        E: Clone + Send + Sync + 'static,
    {
        let values = Arc::new(RwLock::new(Vec::new()));
        let termination = Arc::new(RwLock::new(None));
        let dropped = Arc::new(RwLock::new(false));

        let mut stream = CheckerStream {
            source: stream,
            values: values.clone(),
            termination: termination.clone(),
            dropped: dropped.clone(),
        };

        let handle = tokio::spawn(async move { while stream.next().await.is_some() {} });
        let disposal = CallbackDisposal::new(move || {
            handle.abort();
        });

        (
            Self {
                values,
                termination,
                dropped,
            },
            disposal,
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

struct CheckerStream<T, E, SM> {
    source: SM,
    values: Arc<RwLock<Vec<T>>>,
    termination: Arc<RwLock<Option<Termination<E>>>>,
    dropped: Arc<RwLock<bool>>,
}

impl<T, E, SM> Stream for CheckerStream<T, E, SM>
where
    T: Clone,
    E: Clone,
    SM: Stream<Item = Event<T, E>> + Unpin,
{
    type Item = SM::Item;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        let poll = self.source.poll_next_unpin(cx);
        match &poll {
            std::task::Poll::Ready(event) => {
                if let Some(event) = event {
                    match event {
                        Event::Next(value) => self.values.write().unwrap().push(value.clone()),
                        Event::Termination(termination) => {
                            self.termination
                                .write()
                                .unwrap()
                                .replace(termination.clone());
                        }
                    }
                }
            }
            std::task::Poll::Pending => {}
        }
        poll
    }
}

impl<T, E, SM> Drop for CheckerStream<T, E, SM> {
    fn drop(&mut self) {
        *self.dropped.write().unwrap() = true;
    }
}
