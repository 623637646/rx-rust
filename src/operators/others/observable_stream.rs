use crate::{
    observable::Observable,
    observer::{Event, Observer, Termination},
    subscription::Subscription,
};
use futures::Stream;
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
    task::{Poll, Waker},
};

pub struct ObservableStream<'sub, T, E, OE> {
    source: Option<OE>,
    sub: Option<Subscription<'sub>>,
    context: Arc<Mutex<Context<T, E>>>,
}

impl<'or, 'sub, T, E, OE> ObservableStream<'sub, T, E, OE> {
    pub fn new(source: OE) -> Self
    where
        OE: Observable<'or, 'sub, T, E>,
    {
        Self {
            source: Some(source),
            sub: None,
            context: Arc::new(Mutex::new(Context {
                events: Some(VecDeque::new()),
                waker: None,
            })),
        }
    }
}

impl<'or, 'sub, T, E, OE> Stream for ObservableStream<'sub, T, E, OE>
where
    T: Send + 'or,
    E: Send + 'or,
    OE: Observable<'or, 'sub, T, E> + Unpin,
{
    type Item = Event<T, E>;

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
        if let Some(events) = context.events.as_mut() {
            if let Some(event) = events.pop_front() {
                match event {
                    Event::Next(_) => {}
                    Event::Termination(_) => {
                        context.events = None; // Mark terminated.
                    }
                }
                Poll::Ready(Some(event))
            } else {
                Poll::Pending
            }
        } else {
            Poll::Ready(None)
        }
    }
}

struct Context<T, E> {
    events: Option<VecDeque<Event<T, E>>>, // None means it's terminated
    waker: Option<Waker>,
}

struct ObservableStreamObserver<T, E>(Arc<Mutex<Context<T, E>>>);

impl<T, E> Observer<T, E> for ObservableStreamObserver<T, E> {
    fn on_next(&mut self, value: T) {
        let mut context = self.0.lock().unwrap();
        if let Some(events) = context.events.as_mut() {
            events.push_back(Event::Next(value));
        }
        if let Some(waker) = context.waker.take() {
            waker.wake();
        }
    }

    fn on_termination(self, termination: Termination<E>) {
        let mut context = self.0.lock().unwrap();
        if let Some(events) = context.events.as_mut() {
            events.push_back(Event::Termination(termination));
        }
        if let Some(waker) = context.waker.take() {
            waker.wake();
        }
    }
}
