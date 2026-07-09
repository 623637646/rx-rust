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
pub trait Subject<'or, T, E>: Observable<'or, T, E> + Observer<T, E> {
    fn terminated(&self) -> Option<Termination<E>>
    where
        E: Clone;
}

pub trait SubjectExt<'or, T, E>: Sized {
    // Convert a subject into an observable, erase the observer behavior of the subject.
    fn into_observable(self) -> SubjectObservable<Self> {
        SubjectObservable::new(self)
    }
}

impl<'or, T, E, S> SubjectExt<'or, T, E> for S where S: Subject<'or, T, E> {}
