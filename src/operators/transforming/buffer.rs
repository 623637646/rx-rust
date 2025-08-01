use crate::utils::types::{Mutable, NecessarySend, Shared};
use crate::{
    disposable::subscription::Subscription,
    observable::Observable,
    observer::{Observer, Termination},
    utils::unsub_after_termination::subscribe_unsub_after_termination,
};
use crate::{safe_lock, safe_lock_option, safe_lock_option_observer, safe_lock_vec};
use educe::Educe;

#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct Buffer<OE, OE1> {
    source: OE,
    boundary: OE1,
}

impl<OE, OE1> Buffer<OE, OE1> {
    pub fn new<'or, 'sub, T, E>(source: OE, boundary: OE1) -> Self
    where
        OE: Observable<'or, 'sub, T, E>,
        OE1: Observable<'or, 'sub, (), E>,
    {
        Self { source, boundary }
    }
}

impl<'or, 'sub, T, E, OE, OE1> Observable<'or, 'sub, Vec<T>, E> for Buffer<OE, OE1>
where
    T: NecessarySend + 'or,
    OE: Observable<'or, 'sub, T, E>,
    OE1: Observable<'or, 'sub, (), E>,
    'sub: 'or,
{
    fn subscribe(
        self,
        observer: impl Observer<Vec<T>, E> + NecessarySend + 'or,
    ) -> Subscription<'sub> {
        subscribe_unsub_after_termination(observer, |observer| {
            let observer = Shared::new(Mutable::new(Some(observer)));
            let values = Shared::new(Mutable::new(Vec::default()));
            let subscription_1 = self.boundary.subscribe(BoundaryObserver {
                observer: observer.clone(),
                values: values.clone(),
            });
            let subscription_2 = self.source.subscribe(BufferObserver { observer, values });
            subscription_1 + subscription_2
        })
    }
}

struct BufferObserver<T, OR> {
    observer: Shared<Mutable<Option<OR>>>,
    values: Shared<Mutable<Vec<T>>>,
}

impl<T, E, OR> Observer<T, E> for BufferObserver<T, OR>
where
    OR: Observer<Vec<T>, E>,
{
    fn on_next(&mut self, value: T) {
        safe_lock_vec!(push: self.values, value);
    }

    fn on_termination(self, termination: Termination<E>) {
        if let Some(mut observer) = safe_lock_option!(take: self.observer) {
            match termination {
                Termination::Completed => {
                    let values = safe_lock!(mem_take: self.values);
                    if !values.is_empty() {
                        observer.on_next(values);
                    }
                }
                Termination::Error(_) => {}
            }
            observer.on_termination(termination);
        }
    }
}

struct BoundaryObserver<T, OR> {
    observer: Shared<Mutable<Option<OR>>>,
    values: Shared<Mutable<Vec<T>>>,
}

impl<T, E, OR> Observer<(), E> for BoundaryObserver<T, OR>
where
    OR: Observer<Vec<T>, E>,
{
    fn on_next(&mut self, _: ()) {
        let values = safe_lock!(mem_take: self.values);
        safe_lock_option_observer!(on_next: self.observer, values);
    }

    fn on_termination(self, termination: Termination<E>) {
        if let Some(mut observer) = safe_lock_option!(take: self.observer) {
            match termination {
                Termination::Completed => {
                    let values = safe_lock!(mem_take: self.values);
                    if !values.is_empty() {
                        observer.on_next(values);
                    }
                }
                Termination::Error(_) => {}
            }
            observer.on_termination(termination);
        }
    }
}
