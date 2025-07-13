use crate::disposable::shared_disposal::SharedDisposal;
use crate::disposable::subscription::Subscription;
use crate::utils::types::{Mutable, MutableHelper, NecessarySend, Shared};
use crate::{
    observable::Observable,
    observer::{Observer, Termination},
};
use educe::Educe;

#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct Concat<OE1, OE2> {
    source_1: OE1,
    source_2: OE2,
}

impl<OE1, OE2> Concat<OE1, OE2> {
    pub fn new<'or, 'sub, T, E>(source_1: OE1, source_2: OE2) -> Self
    where
        OE1: Observable<'or, 'sub, T, E>,
        OE2: Observable<'or, 'sub, T, E>,
    {
        Self { source_1, source_2 }
    }
}

impl<'or, 'sub, T, E, OE1, OE2> Observable<'or, 'sub, T, E> for Concat<OE1, OE2>
where
    OE1: Observable<'or, 'sub, T, E>,
    OE2: Observable<'or, 'sub, T, E> + NecessarySend + 'or,
    'sub: 'or,
{
    fn subscribe(self, observer: impl Observer<T, E> + NecessarySend + 'or) -> Subscription<'sub> {
        let sub_2 = Shared::new(Mutable::new(None));
        let onserver = ConcatObserver {
            observer,
            source_2: self.source_2,
            sub_2: sub_2.clone(),
        };
        self.source_1.subscribe(onserver) + SharedDisposal::new(sub_2)
    }
}

struct ConcatObserver<'sub, OR, OE2> {
    observer: OR,
    source_2: OE2,
    sub_2: Shared<Mutable<Option<Subscription<'sub>>>>,
}

impl<'or, 'sub, T, E, OR, OE2> Observer<T, E> for ConcatObserver<'sub, OR, OE2>
where
    OR: Observer<T, E> + NecessarySend + 'or,
    OE2: Observable<'or, 'sub, T, E>,
{
    fn on_next(&mut self, value: T) {
        self.observer.on_next(value);
    }

    fn on_termination(self, termination: Termination<E>) {
        match termination {
            Termination::Completed => {
                let sub = self.source_2.subscribe(self.observer);
                self.sub_2.lock_mut().replace(sub);
            }
            Termination::Error(_) => {
                self.observer.on_termination(termination);
            }
        }
    }
}
