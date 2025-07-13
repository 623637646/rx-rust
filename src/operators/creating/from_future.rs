use crate::utils::types::NecessarySend;
use crate::{
    disposable::subscription::Subscription,
    observable::Observable,
    observer::{Observer, Termination},
    scheduler::Scheduler,
};
use educe::Educe;
use std::convert::Infallible;

#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct FromFuture<FU, S> {
    future: FU,
    scheduler: S,
}

impl<FU, S> FromFuture<FU, S> {
    pub fn new(future: FU, scheduler: S) -> Self
    where
        FU: Future + NecessarySend + 'static,
    {
        Self { future, scheduler }
    }
}

impl<'sub, T, FU, S> Observable<'static, 'sub, T, Infallible> for FromFuture<FU, S>
where
    FU: Future<Output = T> + NecessarySend + 'static,
    S: Scheduler,
{
    fn subscribe(
        self,
        mut observer: impl Observer<T, Infallible> + NecessarySend + 'static,
    ) -> Subscription<'sub> {
        let disposal = self.scheduler.schedule_future(async {
            let result = self.future.await;
            observer.on_next(result);
            observer.on_termination(Termination::Completed);
        });
        Subscription::new_with_disposal(disposal)
    }
}
