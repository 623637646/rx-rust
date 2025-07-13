use crate::disposable::shared_disposal::SharedDisposal;
use crate::disposable::subscription::Subscription;
use crate::utils::types::{Mutable, MutableHelper, NecessarySend, Shared};
use crate::{
    observable::Observable,
    observer::{Observer, Termination},
};
use educe::Educe;

pub enum RetryAction<E, OE1> {
    Retry(OE1),
    Stop(E),
}

#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct Retry<OE, F> {
    source: OE,
    callback: F,
}

impl<OE, F> Retry<OE, F> {
    pub fn new<'or, 'sub, T, E, OE1>(source: OE, callback: F) -> Self
    where
        OE: Observable<'or, 'sub, T, E>,
        OE1: Observable<'or, 'sub, T, E>,
        F: FnMut(E) -> RetryAction<E, OE1>,
    {
        Self { source, callback }
    }
}

impl<'or, 'sub, T, E, OE, OE1, F> Observable<'or, 'sub, T, E> for Retry<OE, F>
where
    OE: Observable<'or, 'sub, T, E>,
    OE1: Observable<'or, 'sub, T, E>,
    F: FnMut(E) -> RetryAction<E, OE1> + NecessarySend + 'or,
    'sub: 'or,
{
    fn subscribe(self, observer: impl Observer<T, E> + NecessarySend + 'or) -> Subscription<'sub> {
        let sub = Shared::new(Mutable::new(None));
        let onserver = RetryObserver {
            observer,
            callback: self.callback,
            sub: sub.clone(),
        };
        self.source.subscribe(onserver) + SharedDisposal::new(sub)
    }
}

struct RetryObserver<'sub, OR, F> {
    observer: OR,
    callback: F,
    sub: Shared<Mutable<Option<Subscription<'sub>>>>,
}

impl<'or, 'sub, T, E, OR, OE1, F> Observer<T, E> for RetryObserver<'sub, OR, F>
where
    OR: Observer<T, E> + NecessarySend + 'or,
    OE1: Observable<'or, 'sub, T, E>,
    F: FnMut(E) -> RetryAction<E, OE1> + NecessarySend + 'or,
    'sub: 'or,
{
    fn on_next(&mut self, value: T) {
        self.observer.on_next(value);
    }

    fn on_termination(mut self, termination: Termination<E>) {
        match termination {
            Termination::Completed => self.observer.on_termination(Termination::Completed),
            Termination::Error(error) => {
                let action = (self.callback)(error);
                match action {
                    RetryAction::Retry(observable) => {
                        let sub = self.sub.clone();
                        *sub.lock_mut() = Some(observable.subscribe(self));
                    }
                    RetryAction::Stop(error) => {
                        self.observer.on_termination(Termination::Error(error))
                    }
                }
            }
        }
    }
}
