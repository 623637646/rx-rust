use crate::{
    observable::Observable,
    observer::{Observer, Termination},
    subscription::Subscription,
};
use futures::Stream;
use std::{
    collections::VecDeque,
    convert::Infallible,
    sync::{Arc, Mutex},
    task::{Poll, Waker},
};

pub struct ObservableStream<'sub, T, OE> {
    source: Option<OE>,
    sub: Option<Subscription<'sub>>,
    context: Arc<Mutex<Context<T>>>,
}

impl<'or, 'sub, T, OE> ObservableStream<'sub, T, OE> {
    pub fn new(source: OE) -> Self
    where
        OE: Observable<'or, 'sub, T, Infallible>,
    {
        Self {
            source: Some(source),
            sub: None,
            context: Arc::new(Mutex::new(Context {
                values: VecDeque::new(),
                terminated: false,
                waker: None,
            })),
        }
    }
}

impl<'or, 'sub, T, OE> Stream for ObservableStream<'sub, T, OE>
where
    T: Send + 'or,
    OE: Observable<'or, 'sub, T, Infallible> + Unpin,
{
    type Item = T;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        if let Some(source) = self.source.take() {
            let observer = ObservableStreamObserver(self.context.clone());
            let sub = source.subscribe(observer);
            self.sub = Some(sub);
        }

        let mut context = self.context.lock().unwrap();
        context.waker = Some(cx.waker().clone());
        if let Some(event) = context.values.pop_front() {
            Poll::Ready(Some(event))
        } else if context.terminated {
            Poll::Ready(None)
        } else {
            Poll::Pending
        }
    }
}

struct Context<T> {
    values: VecDeque<T>,
    terminated: bool,
    waker: Option<Waker>,
}

struct ObservableStreamObserver<T>(Arc<Mutex<Context<T>>>);

impl<T> Observer<T, Infallible> for ObservableStreamObserver<T> {
    fn on_next(&mut self, value: T) {
        let mut context = self.0.lock().unwrap();
        context.values.push_back(value);
        if let Some(waker) = context.waker.take() {
            waker.wake();
        }
    }

    fn on_termination(self, _: Termination<Infallible>) {
        let mut context = self.0.lock().unwrap();
        context.terminated = true;
        if let Some(waker) = context.waker.take() {
            waker.wake();
        }
    }
}
