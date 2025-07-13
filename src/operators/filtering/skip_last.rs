use crate::utils::types::NecessarySend;
use crate::{
    disposable::subscription::Subscription,
    observable::Observable,
    observer::{Observer, Termination},
};
use educe::Educe;
use std::collections::VecDeque;

#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct SkipLast<OE> {
    source: OE,
    count: usize,
}

impl<OE> SkipLast<OE> {
    pub fn new(source: OE, count: usize) -> Self {
        Self { source, count }
    }
}

impl<'or, 'sub, T, E, OE> Observable<'or, 'sub, T, E> for SkipLast<OE>
where
    T: NecessarySend + 'or,
    OE: Observable<'or, 'sub, T, E>,
{
    fn subscribe(self, observer: impl Observer<T, E> + NecessarySend + 'or) -> Subscription<'sub> {
        self.source.subscribe(SkipLastObserver {
            observer,
            count: self.count,
            buffer: VecDeque::new(),
        })
    }
}

struct SkipLastObserver<T, OR> {
    observer: OR,
    count: usize,
    buffer: VecDeque<T>,
}

impl<T, E, OR> Observer<T, E> for SkipLastObserver<T, OR>
where
    OR: Observer<T, E>,
{
    fn on_next(&mut self, value: T) {
        self.buffer.push_back(value);
        if self.buffer.len() > self.count {
            self.observer.on_next(self.buffer.pop_front().unwrap());
        }
    }

    fn on_termination(self, termination: Termination<E>) {
        self.observer.on_termination(termination);
    }
}
