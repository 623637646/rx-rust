use crate::{
    observable::Observable,
    observer::{Observer, Termination},
    subscription::Subscription,
    utils::instant_lock::{InstantMutLock, InstantRefLock},
};
use futures::Stream;
use std::{
    collections::VecDeque,
    convert::Infallible,
    sync::{Arc, Mutex, RwLock},
    task::{Poll, Waker},
};

pub struct ObservableStream<'sub, T, OE> {
    source: Option<OE>,
    sub: Option<Subscription<'sub>>,
    values: Arc<Mutex<VecDeque<T>>>,
    terminated: Arc<RwLock<bool>>,
    waker: Arc<Mutex<Option<Waker>>>,
}

impl<'or, 'sub, T, OE> ObservableStream<'sub, T, OE> {
    pub fn new(source: OE) -> Self
    where
        OE: Observable<'or, 'sub, T, Infallible>,
    {
        Self {
            source: Some(source),
            sub: None,
            values: Arc::new(Mutex::new(VecDeque::new())),
            terminated: Arc::new(RwLock::new(false)),
            waker: Arc::new(Mutex::new(None)),
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
            let observer = ObservableStreamObserver {
                values: self.values.clone(),
                terminated: self.terminated.clone(),
                waker: self.waker.clone(),
            };
            let sub = source.subscribe(observer);
            self.sub = Some(sub);
        }

        self.waker.lock_mut(|v| *v = Some(cx.waker().clone()));
        if let Some(event) = self.values.lock_mut(VecDeque::pop_front) {
            Poll::Ready(Some(event))
        } else if self.terminated.lock_ref(|v| *v) {
            Poll::Ready(None)
        } else {
            Poll::Pending
        }
    }
}

struct ObservableStreamObserver<T> {
    values: Arc<Mutex<VecDeque<T>>>,
    terminated: Arc<RwLock<bool>>,
    waker: Arc<Mutex<Option<Waker>>>,
}

impl<T> Observer<T, Infallible> for ObservableStreamObserver<T> {
    fn on_next(&mut self, value: T) {
        self.values.lock_mut(|v| v.push_back(value));
        if let Some(waker) = self.waker.lock_mut(Option::take) {
            waker.wake();
        }
    }

    fn on_termination(self, _: Termination<Infallible>) {
        self.terminated.lock_mut(|v| *v = true);
        if let Some(waker) = self.waker.lock_mut(Option::take) {
            waker.wake();
        }
    }
}
