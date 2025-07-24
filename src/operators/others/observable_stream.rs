use crate::{
    disposable::subscription::Subscription,
    observable::Observable,
    observer::{Observer, Termination},
    utils::{
        safe_lock::{SafeLock, SafeLockOption, SafeLockVecDeque},
        types::{Mutable, NecessarySend, Shared},
    },
};
use futures::Stream;
use std::{
    collections::VecDeque,
    convert::Infallible,
    sync::atomic::{AtomicBool, Ordering},
    task::{Poll, Waker},
};

pub struct ObservableStream<'sub, T, OE> {
    source: Option<OE>,
    sub: Option<Subscription<'sub>>,
    values: Shared<Mutable<VecDeque<T>>>,
    terminated: Shared<AtomicBool>,
    waker: Shared<Mutable<Option<Waker>>>,
}

impl<'or, 'sub, T, OE> ObservableStream<'sub, T, OE> {
    pub fn new(source: OE) -> Self
    where
        OE: Observable<'or, 'sub, T, Infallible>,
    {
        Self {
            source: Some(source),
            sub: None,
            values: Shared::new(Mutable::new(VecDeque::new())),
            terminated: Shared::new(AtomicBool::new(false)),
            waker: Shared::new(Mutable::new(None)),
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
                values: self.values.clone(),
                terminated: self.terminated.clone(),
                waker: self.waker.clone(),
            };
            let sub = source.subscribe(observer);
            self.sub = Some(sub);
        }

        self.waker.safe_lock_set(Some(cx.waker().clone()));
        if let Some(event) = self.values.safe_lock_pop_front() {
            Poll::Ready(Some(event))
        } else if self.terminated.load(Ordering::SeqCst) {
            Poll::Ready(None)
        } else {
            Poll::Pending
        }
    }
}

struct ObservableStreamObserver<T> {
    values: Shared<Mutable<VecDeque<T>>>,
    terminated: Shared<AtomicBool>,
    waker: Shared<Mutable<Option<Waker>>>,
}

impl<T> Observer<T, Infallible> for ObservableStreamObserver<T> {
    fn on_next(&mut self, value: T) {
        self.values.safe_lock_push_back(value);
        if let Some(waker) = self.waker.safe_lock_take() {
            waker.wake();
        }
    }

    fn on_termination(self, _: Termination<Infallible>) {
        self.terminated.store(true, Ordering::SeqCst);
        if let Some(waker) = self.waker.safe_lock_take() {
            waker.wake();
        }
    }
}
