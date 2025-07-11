use crate::utils::types::NecessarySend;
use crate::{
    observable::Observable,
    observer::{Observer, Termination},
    subscription::Subscription,
};
use educe::Educe;
use std::num::NonZeroUsize;

#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct BufferWithCount<OE> {
    source: OE,
    count: NonZeroUsize,
}

impl<OE> BufferWithCount<OE> {
    pub fn new(source: OE, count: NonZeroUsize) -> Self {
        Self { source, count }
    }
}

impl<'or, 'sub, T, E, OE> Observable<'or, 'sub, Vec<T>, E> for BufferWithCount<OE>
where
    T: NecessarySend + 'or,
    OE: Observable<'or, 'sub, T, E>,
{
    fn subscribe(
        self,
        observer: impl Observer<Vec<T>, E> + NecessarySend + 'or,
    ) -> Subscription<'sub> {
        let observer = BufferWithCountObserver {
            observer,
            values: Vec::default(),
            count: self.count,
        };
        self.source.subscribe(observer)
    }
}

struct BufferWithCountObserver<T, OR> {
    observer: OR,
    values: Vec<T>,
    count: NonZeroUsize,
}

impl<T, E, OR> Observer<T, E> for BufferWithCountObserver<T, OR>
where
    OR: Observer<Vec<T>, E>,
{
    fn on_next(&mut self, value: T) {
        self.values.push(value);
        if self.values.len() >= self.count.get() {
            self.observer.on_next(std::mem::take(&mut self.values));
        }
    }

    fn on_termination(mut self, termination: Termination<E>) {
        match termination {
            Termination::Completed => {
                if !self.values.is_empty() {
                    self.observer.on_next(std::mem::take(&mut self.values));
                }
            }
            Termination::Error(_) => {}
        }
        self.observer.on_termination(termination);
    }
}
