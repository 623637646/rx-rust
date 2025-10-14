use crate::utils::types::NecessarySend;
use crate::{
    disposable::subscription::Subscription,
    observable::Observable,
    observer::{Observer, Termination},
};
use educe::Educe;

#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct DefaultIfEmpty<T, OE> {
    source: OE,
    default_value: T,
}

impl<T, OE> DefaultIfEmpty<T, OE> {
    pub fn new<'or, 'sub, E>(source: OE, item: T) -> Self
    where
        OE: Observable<'or, 'sub, T, E>,
    {
        Self {
            source,
            default_value: item,
        }
    }
}

impl<'or, 'sub, T, E, OE> Observable<'or, 'sub, T, E> for DefaultIfEmpty<T, OE>
where
    OE: Observable<'or, 'sub, T, E>,
    T: NecessarySend + 'or,
{
    fn subscribe(self, observer: impl Observer<T, E> + NecessarySend + 'or) -> Subscription<'sub> {
        let observer = DefaultIfEmptyObserver {
            observer,
            default_value: Some(self.default_value),
        };
        self.source.subscribe(observer)
    }
}

struct DefaultIfEmptyObserver<T, OR> {
    observer: OR,
    default_value: Option<T>, // None means the source's got at least one value already.
}

impl<T, E, OR> Observer<T, E> for DefaultIfEmptyObserver<T, OR>
where
    OR: Observer<T, E>,
{
    fn on_next(&mut self, value: T) {
        self.default_value = None;
        self.observer.on_next(value);
    }

    fn on_termination(mut self, termination: Termination<E>) {
        match termination {
            Termination::Completed => {
                if let Some(default_value) = self.default_value {
                    self.observer.on_next(default_value);
                }
            }
            Termination::Error(_) => {}
        }
        self.observer.on_termination(termination);
    }
}
