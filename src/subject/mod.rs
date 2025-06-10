pub mod behavior_subject;
pub mod publish_subject;
pub mod subject_ext;
pub mod subject_observable;

use crate::{
    observable::Observable,
    observer::{Observer, Termination},
};

pub trait Subject<'or, 'sub, T, E>: Observable<'or, 'sub, T, E> + Observer<T, E> {
    fn terminated(&self) -> Option<Termination<E>>
    where
        E: Clone;
}
