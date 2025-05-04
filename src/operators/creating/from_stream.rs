use crate::{
    observable::Observable,
    observer::{Observer, Termination},
    scheduler::Scheduler,
    subscription::Subscription,
};
use educe::Educe;
use futures::Stream;
use std::{
    convert::Infallible,
    sync::{Arc, Mutex},
};

#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct FromStream<SM, S> {
    stream: SM,
    scheduler: S,
}

impl<SM, S> FromStream<SM, S> {
    pub fn new(stream: SM, scheduler: S) -> Self
    where
        SM: Stream + Send + Unpin + 'static,
    {
        Self { stream, scheduler }
    }
}

impl<'sub, T, SM, S> Observable<'static, 'sub, T, Infallible> for FromStream<SM, S>
where
    SM: Stream<Item = T> + Send + Unpin + 'static,
    S: Scheduler,
{
    fn subscribe(
        self,
        observer: impl Observer<T, Infallible> + Send + 'static,
    ) -> Subscription<'sub> {
        let observer = Arc::new(Mutex::new(Some(observer)));
        let disposal = self
            .scheduler
            .schedule_stream(self.stream, move |result| match result {
                Some(value) => {
                    if let Some(observer) = observer.lock().unwrap().as_mut() {
                        observer.on_next(value)
                    }
                }
                None => {
                    if let Some(observer) = observer.lock().unwrap().take() {
                        observer.on_termination(Termination::Completed)
                    }
                }
            });
        Subscription::new_with_disposal(disposal)
    }
}
