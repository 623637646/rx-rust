use super::Subject;
use crate::utils::types::NecessarySend;
use crate::{disposable::subscription::Subscription, observable::Observable, observer::Observer};
use educe::Educe;

/// An observable from a subject without the observer behavior of the subject.
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct SubjectObservable<S>(S);

impl<S> SubjectObservable<S> {
    pub fn new(subject: S) -> Self {
        Self(subject)
    }
}

impl<'or, 'sub, T, E, S> Observable<'or, 'sub, T, E> for SubjectObservable<S>
where
    S: Subject<'or, 'sub, T, E>,
{
    fn subscribe(self, observer: impl Observer<T, E> + NecessarySend + 'or) -> Subscription<'sub> {
        self.0.subscribe(observer)
    }
}
