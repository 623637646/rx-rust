use crate::{disposable::Disposable, observer::Observer};

pub mod observable_ext;

pub trait Observable<'a, T, E> {
    fn subscribe<O>(&'a self, observer: O) -> impl Disposable
    where
        O: Observer<T, E>;
}
