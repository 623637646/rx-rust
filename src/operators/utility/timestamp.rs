use crate::{
    disposable::subscription::Subscription,
    observable::Observable,
    observer::{Observer, Termination},
    utils::types::NecessarySend,
};
use educe::Educe;
use std::time::Instant;

/// Attaches a timestamp to each item emitted by an Observable.
/// See <https://reactivex.io/documentation/operators/timestamp.html>
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct Timestamp<OE> {
    source: OE,
}

impl<OE> Timestamp<OE> {
    pub fn new(source: OE) -> Self {
        Self { source }
    }
}

impl<'or, 'sub, T, E, OE> Observable<'or, 'sub, (T, Instant), E> for Timestamp<OE>
where
    OE: Observable<'or, 'sub, T, E>,
{
    fn subscribe(
        self,
        observer: impl Observer<(T, Instant), E> + NecessarySend + 'or,
    ) -> Subscription<'sub> {
        let observer = TimestampObserver { observer };
        self.source.subscribe(observer)
    }
}

struct TimestampObserver<OR> {
    observer: OR,
}

impl<T, E, OR> Observer<T, E> for TimestampObserver<OR>
where
    OR: Observer<(T, Instant), E>,
{
    fn on_next(&mut self, value: T) {
        self.observer.on_next((value, Instant::now()));
    }

    fn on_termination(self, termination: Termination<E>) {
        self.observer.on_termination(termination);
    }
}
