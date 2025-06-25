use crate::{
    observable::Observable,
    observer::{Observer, Termination},
    subject::{publish_subject::PublishSubject, subject_observable::SubjectObservable},
    subscription::Subscription,
    utils::unsub_after_termination::subscribe_unsub_after_termination,
};
use educe::Educe;
use std::sync::{Arc, Mutex};

#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct Window<OE, OE1> {
    source: OE,
    boundary: OE1,
}

impl<OE, OE1> Window<OE, OE1> {
    pub fn new<'or, 'sub, T, E>(source: OE, boundary: OE1) -> Self
    where
        OE: Observable<'or, 'sub, T, E>,
        OE1: Observable<'or, 'sub, (), E>,
    {
        Self { source, boundary }
    }
}

impl<'or, 'sub, T, E, OE, OE1>
    Observable<'or, 'sub, SubjectObservable<PublishSubject<'or, T, E>>, E> for Window<OE, OE1>
where
    T: Clone + 'or,
    E: Clone + Send + 'or,
    OE: Observable<'or, 'sub, T, E>,
    OE1: Observable<'or, 'sub, (), E>,
    'sub: 'or,
{
    fn subscribe(
        self,
        observer: impl Observer<SubjectObservable<PublishSubject<'or, T, E>>, E> + Send + 'or,
    ) -> Subscription<'sub> {
        subscribe_unsub_after_termination(observer, |mut observer| {
            let subject = PublishSubject::default();
            observer.on_next(SubjectObservable::new(subject.clone()));

            let subject = Arc::new(Mutex::new(subject));
            let observer = Arc::new(Mutex::new(Some(observer)));
            let window_observer = WindowObserver {
                observer: observer.clone(),
                subject: subject.clone(),
            };
            let boundary_observer = BoundaryObserver { observer, subject };
            let subscription_1 = self.boundary.subscribe(boundary_observer);
            let subscription_2 = self.source.subscribe(window_observer);
            subscription_1 + subscription_2
        })
    }
}

struct WindowObserver<'or, T, E, OR> {
    observer: Arc<Mutex<Option<OR>>>,
    subject: Arc<Mutex<PublishSubject<'or, T, E>>>,
}

impl<'or, T, E, OR> Observer<T, E> for WindowObserver<'or, T, E, OR>
where
    T: Clone,
    E: Clone,
    OR: Observer<SubjectObservable<PublishSubject<'or, T, E>>, E>,
{
    fn on_next(&mut self, value: T) {
        self.subject.lock().unwrap().on_next(value);
    }

    fn on_termination(self, termination: Termination<E>) {
        self.subject
            .lock()
            .unwrap()
            .clone()
            .on_termination(termination.clone());
        if let Some(observer) = { self.observer.lock().unwrap().take() } {
            observer.on_termination(termination);
        }
    }
}

struct BoundaryObserver<'or, T, E, OR> {
    observer: Arc<Mutex<Option<OR>>>,
    subject: Arc<Mutex<PublishSubject<'or, T, E>>>,
}

impl<'or, T, E, OR> Observer<(), E> for BoundaryObserver<'or, T, E, OR>
where
    T: Clone,
    E: Clone,
    OR: Observer<SubjectObservable<PublishSubject<'or, T, E>>, E>,
{
    fn on_next(&mut self, _: ()) {
        let new_subject = PublishSubject::default();
        let old_subject =
            std::mem::replace(&mut *self.subject.lock().unwrap(), new_subject.clone());
        old_subject.on_termination(Termination::Completed);
        if let Some(observer) = self.observer.lock().unwrap().as_mut() {
            observer.on_next(SubjectObservable::new(new_subject));
        }
    }

    fn on_termination(self, termination: Termination<E>) {
        self.subject
            .lock()
            .unwrap()
            .clone()
            .on_termination(termination.clone());
        if let Some(observer) = { self.observer.lock().unwrap().take() } {
            observer.on_termination(termination);
        }
    }
}
