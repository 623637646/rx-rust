use crate::disposable::shared_disposal::SharedDisposal;
use crate::disposable::subscription::Subscription;
use crate::utils::safe_lock::SafeLockOption;
use crate::utils::types::{Mutable, NecessarySend, Shared};
use crate::{
    observable::Observable,
    observer::{Observer, Termination},
    utils::types::MarkerType,
};
use educe::Educe;
use std::marker::PhantomData;

#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct CatchError<E0, OE, F> {
    source: OE,
    callback: F,
    _marker: MarkerType<E0>,
}

impl<E0, OE, F> CatchError<E0, OE, F> {
    pub fn new<'or, 'sub, T, E, OE1>(source: OE, callback: F) -> Self
    where
        OE: Observable<'or, 'sub, T, E0>,
        OE1: Observable<'or, 'sub, T, E>,
        F: FnOnce(E0) -> OE1,
    {
        Self {
            source,
            callback,
            _marker: PhantomData,
        }
    }
}

impl<'or, 'sub, T, E0, E, OE, OE1, F> Observable<'or, 'sub, T, E> for CatchError<E0, OE, F>
where
    E: 'or,
    OE: Observable<'or, 'sub, T, E0>,
    OE1: Observable<'or, 'sub, T, E>,
    F: FnOnce(E0) -> OE1 + NecessarySend + 'or,
    'sub: 'or,
{
    fn subscribe(self, observer: impl Observer<T, E> + NecessarySend + 'or) -> Subscription<'sub> {
        let sub = Shared::new(Mutable::new(None));
        let onserver = CatchErrorObserver {
            observer,
            callback: self.callback,
            sub: sub.clone(),
            _marker: PhantomData,
        };
        self.source.subscribe(onserver) + SharedDisposal::new(sub)
    }
}

struct CatchErrorObserver<'sub, E, OR, F> {
    observer: OR,
    callback: F,
    sub: Shared<Mutable<Option<Subscription<'sub>>>>,
    _marker: MarkerType<E>,
}

impl<'or, 'sub, T, E0, E, OR, OE1, F> Observer<T, E0> for CatchErrorObserver<'sub, E, OR, F>
where
    OR: Observer<T, E> + NecessarySend + 'or,
    OE1: Observable<'or, 'sub, T, E>,
    F: FnOnce(E0) -> OE1,
{
    fn on_next(&mut self, value: T) {
        self.observer.on_next(value);
    }

    fn on_termination(self, termination: Termination<E0>) {
        match termination {
            Termination::Completed => self.observer.on_termination(Termination::Completed),
            Termination::Error(error) => {
                let observable = (self.callback)(error);
                let sub = observable.subscribe(self.observer);
                self.sub.safe_lock_replace(sub);
            }
        }
    }
}
