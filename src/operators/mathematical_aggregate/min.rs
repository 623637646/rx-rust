use crate::utils::types::NecessarySend;
use crate::{
    disposable::subscription::Subscription,
    observable::Observable,
    observer::{Observer, Termination},
};
use educe::Educe;

#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct Min<OE> {
    source: OE,
}

impl<OE> Min<OE> {
    pub fn new<'or, 'sub, T, E>(source: OE) -> Self
    where
        OE: Observable<'or, 'sub, T, E>,
    {
        Self { source }
    }
}

impl<'or, 'sub, T, E, OE> Observable<'or, 'sub, T, E> for Min<OE>
where
    T: PartialOrd + NecessarySend + 'or,
    OE: Observable<'or, 'sub, T, E>,
{
    fn subscribe(self, observer: impl Observer<T, E> + NecessarySend + 'or) -> Subscription<'sub> {
        let observer = MinObserver {
            observer,
            min: None,
        };
        self.source.subscribe(observer)
    }
}

struct MinObserver<T, OR> {
    observer: OR,
    min: Option<T>,
}

impl<T, E, OR> Observer<T, E> for MinObserver<T, OR>
where
    T: PartialOrd,
    OR: Observer<T, E>,
{
    fn on_next(&mut self, value: T) {
        if let Some(min) = &mut self.min {
            if value < *min {
                *min = value;
            }
        } else {
            self.min = Some(value);
        }
    }

    fn on_termination(mut self, termination: Termination<E>) {
        match termination {
            Termination::Completed => {
                if let Some(min) = self.min {
                    self.observer.on_next(min);
                }
            }
            Termination::Error(_) => {}
        }
        self.observer.on_termination(termination)
    }
}
