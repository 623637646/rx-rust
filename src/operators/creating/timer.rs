use crate::utils::types::NecessarySend;
use crate::{
    observable::Observable,
    observer::{Observer, Termination},
    scheduler::Scheduler,
    subscription::Subscription,
};
use educe::Educe;
use std::{convert::Infallible, time::Duration};

#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct Timer<T, S> {
    value: T,
    delay: Duration,
    scheduler: S,
}

impl<T, S> Timer<T, S> {
    pub fn new(value: T, delay: Duration, scheduler: S) -> Self {
        Self {
            value,
            delay,
            scheduler,
        }
    }
}

impl<'sub, T, S> Observable<'static, 'sub, T, Infallible> for Timer<T, S>
where
    T: NecessarySend + 'static,
    S: Scheduler,
{
    fn subscribe(
        self,
        mut observer: impl Observer<T, Infallible> + NecessarySend + 'static,
    ) -> Subscription<'sub> {
        let disposal = self.scheduler.schedule(
            || {
                observer.on_next(self.value);
                observer.on_termination(Termination::Completed);
            },
            Some(self.delay),
        );
        Subscription::new_with_disposal(disposal)
    }
}
