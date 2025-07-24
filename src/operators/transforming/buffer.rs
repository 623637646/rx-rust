use crate::utils::safe_lock::{SafeLock, SafeLockOption, SafeLockOptionObserver, SafeLockVec};
use crate::utils::types::{Mutable, NecessarySend, Shared};
use crate::{
    disposable::subscription::Subscription,
    observable::Observable,
    observer::{Observer, Termination},
    utils::unsub_after_termination::subscribe_unsub_after_termination,
};
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
            let observer = BufferObserver {
                observer: Shared::new(Mutable::new(Some(observer))),
                values: Shared::new(Mutable::new(Vec::default())),
            };
            let boundary = BoundaryObserver(observer.clone());
            let subscription_1 = self.boundary.subscribe(boundary);
            let subscription_2 = self.source.subscribe(observer);
            subscription_1 + subscription_2
        })
    }
}

#[derive(Educe)]
#[educe(Clone)]
struct BufferObserver<T, OR> {
    observer: Shared<Mutable<Option<OR>>>,
    values: Shared<Mutable<Vec<T>>>,
}

impl<T, E, OR> Observer<T, E> for BufferObserver<T, OR>
where
    OR: Observer<Vec<T>, E>,
{
    fn on_next(&mut self, value: T) {
        self.values.safe_lock_push(value);
    }

    fn on_termination(self, termination: Termination<E>) {
        if let Some(mut observer) = self.observer.safe_lock_take() {
            match termination {
                Termination::Completed => {
                    let values = self.values.safe_lock_mem_take();
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

struct BoundaryObserver<T, OR>(BufferObserver<T, OR>);

impl<T, E, OR> Observer<(), E> for BoundaryObserver<T, OR>
where
    OR: Observer<Vec<T>, E>,
{
    fn on_next(&mut self, _: ()) {
        self.0
            .observer
            .safe_lock_on_next_with_builder(|| Some(self.0.values.safe_lock_mem_take()));
    }

    fn on_termination(self, termination: Termination<E>) {
        self.0.on_termination(termination);
    }
}
