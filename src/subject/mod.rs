pub mod async_subject;
pub mod behavior_subject;
pub mod publish_subject;
pub mod replay_subject;
pub mod subject_ext;
pub mod subject_observable;

use crate::{
    observable::Observable,
    observer::{Observer, Termination},
};

/// A Subject is a sort of bridge or proxy that acts both as an observer and as an Observable.
/// See <https://reactivex.io/documentation/subject.html>
pub trait Subject<'or, 'sub, T, E>: Observable<'or, 'sub, T, E> + Observer<T, E> {
    fn terminated(&self) -> Option<Termination<E>>
    where
        E: Clone;
}
