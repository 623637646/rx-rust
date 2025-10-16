use crate::utils::types::{MarkerType, NecessarySend};
use crate::{
    disposable::subscription::Subscription,
    observable::Observable,
    observer::{Observer, Termination},
};
use educe::Educe;
use std::marker::PhantomData;

#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct Count<T, OE> {
    source: OE,
    _marker: MarkerType<T>,
}

impl<T, OE> Count<T, OE> {
    pub fn new<'or, 'sub, E>(source: OE) -> Self
    where
        OE: Observable<'or, 'sub, T, E>,
    {
        Self {
            source,
            _marker: PhantomData,
        }
    }
}

impl<'or, 'sub, T, E, OE> Observable<'or, 'sub, usize, E> for Count<T, OE>
where
    OE: Observable<'or, 'sub, T, E>,
{
    fn subscribe(
        self,
        observer: impl Observer<usize, E> + NecessarySend + 'or,
    ) -> Subscription<'sub> {
        let observer = CountObserver { observer, count: 0 };
        self.source.subscribe(observer)
    }
}

struct CountObserver<OR> {
    observer: OR,
    count: usize,
}

impl<T, E, OR> Observer<T, E> for CountObserver<OR>
where
    OR: Observer<usize, E>,
{
    fn on_next(&mut self, _: T) {
        self.count += 1;
    }

    fn on_termination(mut self, termination: Termination<E>) {
        match termination {
            Termination::Completed => {
                self.observer.on_next(self.count);
            }
            Termination::Error(_) => {}
        }
        self.observer.on_termination(termination)
    }
}
