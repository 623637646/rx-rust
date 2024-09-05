pub mod hot_observable;
pub mod observable_ext;

use crate::cancellable::Cancellable;
use crate::observer::Observer;

pub trait Observable<'a, T, E> {
    fn subscribe<O>(&'a self, observer: O) -> impl Cancellable
    where
        O: Observer<T, E> + 'static;
}
