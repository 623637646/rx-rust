use crate::{
    observable::{Observable, Subscription},
    observer::{Observer, Termination},
    utils::types::{MaybeSend, Mutable, MutableHelper, Shared},
};
use educe::Educe;
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
#[derive(Educe)]
#[educe(Debug)]
pub struct ObservableStream<'or, T, OE>
where
    OE: Observable<'or, T, Infallible>,
{
    source: Option<OE>,
    sub: Option<Subscription<OE::D>>,
    context: Shared<Mutable<ObservableStreamContext<T>>>,
}

impl<'or, T, OE> ObservableStream<'or, T, OE>
where
    OE: Observable<'or, T, Infallible>,
{
    pub fn new(source: OE) -> Self
    where
        OE: Observable<'or, T, Infallible>,
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

impl<'or, T, OE> Unpin for ObservableStream<'or, T, OE> where OE: Observable<'or, T, Infallible> {}

impl<'or, T, OE> Stream for ObservableStream<'or, T, OE>
where
    T: MaybeSend + 'or,
    OE: Observable<'or, T, Infallible>,
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

        let waker = cx.waker().clone();
        // The waker this one replaces is handed back, because dropping a `Waker` runs the
        // external code of its vtable, which must not run under the lock.
        let (poll, previous_waker) = self.context.lock_mut(|mut lock| {
            let previous_waker = lock.waker.replace(waker);
            let poll = if let Some(event) = lock.values.pop_front() {
                Poll::Ready(Some(event))
            } else if lock.terminated {
                Poll::Ready(None)
            } else {
                Poll::Pending
            };
            (poll, previous_waker)
        });
        drop(previous_waker); // Drop outside the lock to avoid potential deadlock
        poll
    }
}

struct ObservableStreamObserver<T> {
    context: Shared<Mutable<ObservableStreamContext<T>>>,
}

impl<T> Observer<T, Infallible> for ObservableStreamObserver<T> {
    fn on_next(&mut self, value: T) {
        // The waker is taken under the lock and woken after it is released, because waking runs
        // external code, which must not run under the lock.
        let waker = self.context.lock_mut(|mut lock| {
            lock.values.push_back(value);
            lock.waker.take()
        });
        if let Some(waker) = waker {
            waker.wake();
        }
    }

    fn on_termination(self, _: Termination<Infallible>) {
        // The waker is woken outside the lock, like in `on_next`.
        let waker = self.context.lock_mut(|mut lock| {
            lock.terminated = true;
            lock.waker.take()
        });
        if let Some(waker) = waker {
            waker.wake();
        }
    }
}
