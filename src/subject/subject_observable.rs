use super::Subject;
use crate::observable::Subscription;
use crate::utils::types::MaybeSend;
use crate::{observable::Observable, observer::Observer};
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

impl<'or, T, E, S> Observable<'or, T, E> for SubjectObservable<S>
where
    S: Subject<'or, T, E>,
{
    type D = S::D;

    fn subscribe(self, observer: impl Observer<T, E> + MaybeSend + 'or) -> Subscription<Self::D> {
        self.0.subscribe(observer)
    }
}
