use crate::{
    disposable::subscription::Subscription,
    observable::Observable,
    observer::{Observer, Termination},
    utils::types::NecessarySend,
};
use educe::Educe;
use std::time::{Duration, Instant};

#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct TimeInterval<OE> {
    source: OE,
}

impl<OE> TimeInterval<OE> {
    pub fn new(source: OE) -> Self {
        Self { source }
    }
}

impl<'or, 'sub, T, E, OE> Observable<'or, 'sub, (T, Duration), E> for TimeInterval<OE>
where
    OE: Observable<'or, 'sub, T, E>,
{
    fn subscribe(
        self,
        observer: impl Observer<(T, Duration), E> + NecessarySend + 'or,
    ) -> Subscription<'sub> {
        let observer = TimeIntervalObserver {
            observer,
            time_stamp: Instant::now(),
        };
        self.source.subscribe(observer)
    }
}

struct TimeIntervalObserver<OR> {
    observer: OR,
    time_stamp: Instant,
}

impl<T, E, OR> Observer<T, E> for TimeIntervalObserver<OR>
where
    OR: Observer<(T, Duration), E>,
{
    fn on_next(&mut self, value: T) {
        let time_span = self.time_stamp.elapsed();
        self.time_stamp = Instant::now();
        self.observer.on_next((value, time_span));
    }

    fn on_termination(self, termination: Termination<E>) {
        self.observer.on_termination(termination);
    }
}
