use crate::{
    disposable::subscription::Subscription,
    observable::Observable,
    observer::{Observer, Termination},
    utils::types::{Mutable, MutableHelper, NecessarySend, Shared},
};
use futures::Stream;
use std::{
    collections::VecDeque,
    convert::Infallible,
    task::{Poll, Waker},
};

struct ObservableStreamContext<T> {
    values: VecDeque<T>,
    waker: Option<Waker>,
    terminated: bool,
}

/// Converts an Observable into a `futures::Stream` that can be used with `async/await`.
///
/// # Examples
/// ```rust
/// use futures::StreamExt;
/// use rx_rust::{
///     operators::{
///         creating::from_iter::FromIter,
///         others::observable_stream::ObservableStream,
///     },
/// };
///
/// futures::executor::block_on(async {
///     let source = FromIter::new(vec![1, 2, 3]);
///     let mut stream = ObservableStream::new(source);
///     let values: Vec<_> = (&mut stream).collect().await;
///     assert_eq!(values, vec![1, 2, 3]);
/// });
/// ```
pub struct ObservableStream<'sub, T, OE> {
    source: Option<OE>,
    sub: Option<Subscription<'sub>>,
    context: Shared<Mutable<ObservableStreamContext<T>>>,
}

impl<'or, 'sub, T, OE> ObservableStream<'sub, T, OE> {
    pub fn new(source: OE) -> Self
    where
        OE: Observable<'or, 'sub, T, Infallible>,
    {
        Self {
            source: Some(source),
            sub: None,
            context: Shared::new(Mutable::new(ObservableStreamContext {
                terminated: false,
                waker: None,
                values: VecDeque::new(),
            })),
        }
    }
}

impl<'or, 'sub, T, OE> Stream for ObservableStream<'sub, T, OE>
where
    T: NecessarySend + 'or,
    OE: Observable<'or, 'sub, T, Infallible> + Unpin,
{
    type Item = T;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        if let Some(source) = self.source.take() {
            let observer = ObservableStreamObserver {
                context: self.context.clone(),
            };
            let sub = source.subscribe(observer);
            self.sub = Some(sub);
        }

        self.context.lock_mut(|mut lock| {
            lock.waker = Some(cx.waker().clone());
            if let Some(event) = lock.values.pop_front() {
                Poll::Ready(Some(event))
            } else if lock.terminated {
                Poll::Ready(None)
            } else {
                Poll::Pending
            }
        })
    }
}

struct ObservableStreamObserver<T> {
    context: Shared<Mutable<ObservableStreamContext<T>>>,
}

impl<T> Observer<T, Infallible> for ObservableStreamObserver<T> {
    fn on_next(&mut self, value: T) {
        self.context.lock_mut(|mut lock| {
            lock.values.push_back(value);
            if let Some(waker) = lock.waker.take() {
                drop(lock);
                waker.wake();
            }
        });
    }

    fn on_termination(self, _: Termination<Infallible>) {
        self.context.lock_mut(|mut lock| {
            lock.terminated = true;
            if let Some(waker) = lock.waker.take() {
                drop(lock);
                waker.wake();
            }
        });
    }
}
