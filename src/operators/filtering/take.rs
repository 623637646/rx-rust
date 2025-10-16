use crate::utils::types::NecessarySend;
use crate::{
    disposable::subscription::Subscription,
    observable::Observable,
    observer::{Observer, Termination},
    utils::unsub_after_termination::subscribe_unsub_after_termination,
};
use educe::Educe;

/// Emits only the first N items emitted by an Observable.
/// See <https://reactivex.io/documentation/operators/take.html>
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct Take<OE> {
    source: OE,
    count: usize,
}

impl<OE> Take<OE> {
    pub fn new(source: OE, count: usize) -> Self {
        Self { source, count }
    }
}

impl<'or, 'sub, T, E, OE> Observable<'or, 'sub, T, E> for Take<OE>
where
    OE: Observable<'or, 'sub, T, E>,
    'sub: 'or,
{
    fn subscribe(self, observer: impl Observer<T, E> + NecessarySend + 'or) -> Subscription<'sub> {
        if self.count == 0 {
            observer.on_termination(Termination::Completed);
            Subscription::default()
        } else {
            subscribe_unsub_after_termination(observer, |observer| {
                self.source.subscribe(TakeObserver {
                    observer: Some(observer),
                    count: self.count,
                })
            })
        }
    }
}

struct TakeObserver<OR> {
    observer: Option<OR>,
    count: usize,
}

impl<T, E, OR> Observer<T, E> for TakeObserver<OR>
where
    OR: Observer<T, E>,
{
    fn on_next(&mut self, value: T) {
        if let Some(observer) = &mut self.observer {
            observer.on_next(value);
            self.count -= 1;
            if self.count == 0 {
                self.observer
                    .take()
                    .unwrap()
                    .on_termination(Termination::Completed);
            }
        }
    }

    fn on_termination(mut self, termination: Termination<E>) {
        if let Some(observer) = self.observer.take() {
            observer.on_termination(termination);
        }
    }
}
