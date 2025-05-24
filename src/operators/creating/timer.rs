use super::just::Just;
use crate::{
    observable::{Observable, observable_ext::ObservableExt},
    observer::Observer,
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
    T: Send + 'static,
    S: Scheduler + Send,
{
    fn subscribe(
        self,
        observer: impl Observer<T, Infallible> + Send + 'static,
    ) -> Subscription<'sub> {
        Just::new(self.value)
            .delay(self.delay, self.scheduler)
            .subscribe(observer)
    }
}
