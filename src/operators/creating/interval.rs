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
pub struct Interval<S> {
    period: Duration,
    scheduler: S,
    delay: Option<Duration>,
}

impl<S> Interval<S> {
    pub fn new(period: Duration, scheduler: S, delay: Option<Duration>) -> Self {
        Self {
            period,
            scheduler,
            delay,
        }
    }
}

impl<'sub, S> Observable<'static, 'sub, usize, Infallible> for Interval<S>
where
    S: Scheduler,
{
    fn subscribe(
        self,
        mut observer: impl Observer<usize, Infallible> + Send + 'static,
    ) -> Subscription<'sub> {
        let disposal = self.scheduler.schedule_period(
            move |count| {
                observer.on_next(count);
                false
            },
            self.period,
            self.delay,
        );
        Subscription::new_with_disposal(disposal)
    }
}

