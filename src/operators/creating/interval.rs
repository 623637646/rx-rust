use crate::utils::types::NecessarySend;
use crate::{
    disposable::subscription::Subscription, observable::Observable, observer::Observer,
    scheduler::Scheduler,
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
        mut observer: impl Observer<usize, Infallible> + NecessarySend + 'static,
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
