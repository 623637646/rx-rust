use crate::{
    observable::Observable,
    observer::{Observer, Terminal},
    subscription::Subscription,
};
use educe::Educe;

#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct BufferWithCount<OE> {
    source: OE,
    count: usize,
}

impl<OE> BufferWithCount<OE> {
    pub fn new(source: OE, count: usize) -> Self {
        Self { source, count }
    }
}

impl<'or, 'sub, T, E, OE> Observable<'or, 'sub, Vec<T>, E> for BufferWithCount<OE>
where
    T: Send + 'or,
    OE: Observable<'or, 'sub, T, E>,
{
    fn subscribe(self, observer: impl Observer<Vec<T>, E> + Send + 'or) -> Subscription<'sub> {
        let observer = BufferWithCountObserver {
            observer,
            values: Vec::default(),
            count: self.count,
        };
        self.source.subscribe(observer)
    }
}

pub struct BufferWithCountObserver<T, OR> {
    observer: OR,
    values: Vec<T>,
    count: usize,
}

impl<T, E, OR> Observer<T, E> for BufferWithCountObserver<T, OR>
where
    OR: Observer<Vec<T>, E>,
{
    fn on_next(&mut self, value: T) {
        self.values.push(value);
        if self.values.len() >= self.count {
            self.observer.on_next(std::mem::take(&mut self.values));
        }
    }

    fn on_terminal(mut self, terminal: Terminal<E>) {
        match terminal {
            Terminal::Completed => {
                if !self.values.is_empty() {
                    self.observer.on_next(std::mem::take(&mut self.values));
                }
                self.observer.on_terminal(Terminal::Completed);
            }
            Terminal::Error(error) => {
                self.observer.on_terminal(Terminal::Error(error));
            }
        }
    }
}
