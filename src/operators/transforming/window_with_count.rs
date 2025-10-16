use crate::utils::types::NecessarySend;
use crate::{
    disposable::subscription::Subscription,
    observable::Observable,
    observer::{Observer, Termination},
    subject::{publish_subject::PublishSubject, subject_observable::SubjectObservable},
};
use educe::Educe;
use std::{cmp::Ordering, num::NonZeroUsize};

/// Periodically subdivides items from an Observable into Observable windows, each containing a specified number of items.
/// See <https://reactivex.io/documentation/operators/window.html>
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct WindowWithCount<OE> {
    source: OE,
    count: NonZeroUsize,
}

impl<OE> WindowWithCount<OE> {
    pub fn new(source: OE, count: NonZeroUsize) -> Self {
        Self { source, count }
    }
}

impl<'or, 'sub, T, E, OE> Observable<'or, 'sub, SubjectObservable<PublishSubject<'or, T, E>>, E>
    for WindowWithCount<OE>
where
    T: Clone + NecessarySend + 'or,
    E: Clone + NecessarySend + 'or,
    OE: Observable<'or, 'sub, T, E>,
{
    fn subscribe(
        self,
        mut observer: impl Observer<SubjectObservable<PublishSubject<'or, T, E>>, E>
        + NecessarySend
        + 'or,
    ) -> Subscription<'sub> {
        let subject = PublishSubject::default();
        observer.on_next(SubjectObservable::new(subject.clone()));

        let observer = WindowWithCountObserver {
            observer,
            subject,
            count: self.count,
            sent_count: 0,
        };
        self.source.subscribe(observer)
    }
}

struct WindowWithCountObserver<'or, T, E, OR> {
    observer: OR,
    subject: PublishSubject<'or, T, E>,
    count: NonZeroUsize,
    sent_count: usize,
}

impl<'or, T, E, OR> Observer<T, E> for WindowWithCountObserver<'or, T, E, OR>
where
    T: Clone,
    E: Clone,
    OR: Observer<SubjectObservable<PublishSubject<'or, T, E>>, E>,
{
    fn on_next(&mut self, value: T) {
        match (self.sent_count + 1).cmp(&self.count.get()) {
            Ordering::Less => {
                self.subject.on_next(value);
                self.sent_count += 1;
            }
            Ordering::Equal => {
                let new_subject = PublishSubject::default();
                let mut old_subject = std::mem::replace(&mut self.subject, new_subject.clone());
                old_subject.on_next(value);
                old_subject.on_termination(Termination::Completed);
                self.observer.on_next(SubjectObservable::new(new_subject));
                self.sent_count = 0;
            }
            Ordering::Greater => unreachable!(),
        }
    }

    fn on_termination(self, termination: Termination<E>) {
        self.subject.on_termination(termination.clone());
        self.observer.on_termination(termination);
    }
}
