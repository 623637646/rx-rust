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

impl<'or, S> Observable<'or> for SubjectObservable<S>
where
    S: Subject<'or>,
{
    type T = S::T;
    type E = S::E;
    type D = S::D;

    fn subscribe(
        self,
        observer: impl Observer<S::T, S::E> + MaybeSend + 'or,
    ) -> Subscription<Self::D> {
        self.0.subscribe(observer)
    }
}
