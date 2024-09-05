use crate::observable::Observable;

use super::map::Map;

// TODO: here
trait Cloned<T, E> {
    fn cloned(self) -> impl for<'a> Observable<'a, T, E>;
}

impl<T, E, O> Cloned<T, E> for O
where
    O: for<'a> Observable<'a, T, E>,
    T: Clone,
{
    fn cloned(self) -> impl for<'a> Observable<'a, T, E> {
        Map::new(self, |value| value.clone())
    }
}

// TODO: create Rx, using API like Rx::just, Rx::empty
