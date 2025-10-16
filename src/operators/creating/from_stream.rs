use crate::utils::types::NecessarySend;
use crate::{
    disposable::subscription::Subscription,
    observable::Observable,
    observer::{Observer, Termination},
    scheduler::Scheduler,
};
use educe::Educe;
use futures::Stream;
use std::convert::Infallible;

/// Converts a `Stream` into an Observable.
/// See <https://reactivex.io/documentation/operators/from.html>
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct FromStream<SM, S> {
    stream: SM,
    scheduler: S,
}

impl<SM, S> FromStream<SM, S> {
    pub fn new(stream: SM, scheduler: S) -> Self
    where
        SM: Stream + NecessarySend + Unpin + 'static,
    {
        Self { stream, scheduler }
    }
}

impl<'sub, T, SM, S> Observable<'static, 'sub, T, Infallible> for FromStream<SM, S>
where
    SM: Stream<Item = T> + NecessarySend + Unpin + 'static,
    S: Scheduler,
{
    fn subscribe(
        self,
        observer: impl Observer<T, Infallible> + NecessarySend + 'static,
    ) -> Subscription<'sub> {
        let mut observer = Some(observer);
        let disposal = self
            .scheduler
            .schedule_stream(self.stream, move |result| match result {
                Some(value) => {
                    if let Some(observer) = observer.as_mut() {
                        observer.on_next(value)
                    }
                }
                None => {
                    if let Some(observer) = observer.take() {
                        observer.on_termination(Termination::Completed)
                    }
                }
            });
        Subscription::new_with_disposal(disposal)
    }
}
