use crate::{
    observable::Observable,
    observer::{Observer, Termination},
    subscription::Subscription,
    utils::unsub_after_termination::subscribe_unsub_after_termination,
};
use educe::Educe;
use std::sync::{Arc, Mutex};

#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct Buffer<OE, OE2> {
    source: OE,
    boundary: OE2,
}

impl<OE, OE2> Buffer<OE, OE2> {
    pub fn new<'or, 'sub, T, E>(source: OE, boundary: OE2) -> Self
    where
        OE: Observable<'or, 'sub, T, E>,
        OE2: Observable<'or, 'sub, (), E>,
    {
        Self { source, boundary }
    }
}

impl<'or, 'sub, T, E, OE, OE2> Observable<'or, 'sub, Vec<T>, E> for Buffer<OE, OE2>
where
    T: Send + 'or,
    OE: Observable<'or, 'sub, T, E>,
    OE2: Observable<'or, 'sub, (), E>,
    'sub: 'or,
{
    fn subscribe(self, observer: impl Observer<Vec<T>, E> + Send + 'or) -> Subscription<'sub> {
        subscribe_unsub_after_termination(observer, |observer| {
            let observer = BufferObserver {
                observer: Arc::new(Mutex::new(Some(observer))),
                values: Arc::new(Mutex::new(Vec::default())),
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
    observer: Arc<Mutex<Option<OR>>>,
    values: Arc<Mutex<Vec<T>>>,
}

impl<T, E, OR> Observer<T, E> for BufferObserver<T, OR>
where
    OR: Observer<Vec<T>, E>,
{
    fn on_next(&mut self, value: T) {
        self.values.lock().unwrap().push(value);
    }

    fn on_termination(self, termination: Termination<E>) {
        if let Some(mut observer) = { self.observer.lock().unwrap().take() } {
            match termination {
                Termination::Completed => {
                    let values = std::mem::take(&mut *self.values.lock().unwrap());
                    if !values.is_empty() {
                        observer.on_next(values);
                    }
                    observer.on_termination(Termination::Completed);
                }
                Termination::Error(error) => {
                    observer.on_termination(Termination::Error(error));
                }
            }
        }
    }
}

struct BoundaryObserver<T, OR>(BufferObserver<T, OR>);

impl<T, E, OR> Observer<(), E> for BoundaryObserver<T, OR>
where
    OR: Observer<Vec<T>, E>,
{
    fn on_next(&mut self, _: ()) {
        if let Some(observer) = self.0.observer.lock().unwrap().as_mut() {
            let values = std::mem::take(&mut *self.0.values.lock().unwrap());
            observer.on_next(values);
        }
    }

    fn on_termination(self, termination: Termination<E>) {
        self.0.on_termination(termination);
    }
}
