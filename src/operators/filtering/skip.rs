use crate::utils::types::NecessarySend;
use crate::{
    disposable::subscription::Subscription,
    observable::Observable,
    observer::{Observer, Termination},
};
use educe::Educe;

/// Suppresses the first N items emitted by an Observable.
/// See <https://reactivex.io/documentation/operators/skip.html>
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct Skip<OE> {
    source: OE,
    count: usize,
}

impl<OE> Skip<OE> {
    pub fn new(source: OE, count: usize) -> Self {
        Self { source, count }
    }
}

impl<'or, 'sub, T, E, OE> Observable<'or, 'sub, T, E> for Skip<OE>
where
    OE: Observable<'or, 'sub, T, E>,
{
    fn subscribe(self, observer: impl Observer<T, E> + NecessarySend + 'or) -> Subscription<'sub> {
        self.source.subscribe(SkipObserver {
            observer,
            count: self.count,
        })
    }
}

struct SkipObserver<OR> {
    observer: OR,
    count: usize,
}

impl<T, E, OR> Observer<T, E> for SkipObserver<OR>
where
    OR: Observer<T, E>,
{
    fn on_next(&mut self, value: T) {
        if self.count > 0 {
            self.count -= 1;
        } else {
            self.observer.on_next(value);
        }
    }

    fn on_termination(self, termination: Termination<E>) {
        self.observer.on_termination(termination);
    }
}
