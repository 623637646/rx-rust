use super::{Subject, subject_observable::SubjectObservable};

pub trait SubjectExt<'or, 'sub, T, E>: Sized {
    // Convert a subject into an observable, erase the observer behavior of the subject.
    fn into_observable(self) -> SubjectObservable<Self> {
        SubjectObservable::new(self)
    }
}

impl<'or, 'sub, T, E, S> SubjectExt<'or, 'sub, T, E> for S where S: Subject<'or, 'sub, T, E> {}
