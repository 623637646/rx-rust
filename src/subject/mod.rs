pub mod async_subject;
pub mod behavior_subject;
pub mod publish_subject;
pub mod replay_subject;
pub mod subject_observable;

use crate::{
    observable::Observable,
    observer::{Observer, Termination},
    subject::subject_observable::SubjectObservable,
};

/// A Subject is a sort of bridge or proxy that acts both as an observer and as an Observable.
/// See <https://reactivex.io/documentation/subject.html>
pub trait Subject<'or>:
    Observable<'or> + Observer<<Self as Observable<'or>>::T, <Self as Observable<'or>>::E>
{
    fn terminated(&self) -> Option<Termination<Self::E>>
    where
        Self::E: Clone;
}

pub trait SubjectExt<'or>: Sized {
    // Convert a subject into an observable, erase the observer behavior of the subject.
    fn into_observable(self) -> SubjectObservable<Self> {
        SubjectObservable::new(self)
    }
}

impl<'or, S> SubjectExt<'or> for S where S: Subject<'or> {}
