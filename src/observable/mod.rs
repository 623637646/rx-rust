use crate::{disposable::Disposable, observer::Observer};

pub mod observable_ext;

pub trait Observable<'a, T, E> {
    fn subscribe<O>(&'a self, observer: O) -> impl Disposable
    where
        O: Observer<T, E>;
}

// TODO: here
// trait Cloned<'a, T, E> {
//     fn cloned(&self) -> impl Observable<T, E>;
// }

// impl<'a, T, E, O> Cloned<'a, T, E> for O
// where
//     O: Observable<'a, T, E>,
// {
//     fn cloned(&self) -> impl Observable<T, E> {
//         Create::new(|observer: &dyn Observer<T, E>| || {})
//     }
// }

// TODO: create Rx, using API like Rx::just, Rx::empty
