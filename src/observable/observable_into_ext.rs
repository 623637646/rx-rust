use super::Observable;

pub trait ObservableIntoExt<T, E> {
    fn into_observable(self) -> impl for<'b> Observable<'b, T, E>;
}

impl<T, E, O> ObservableIntoExt<T, E> for O
where
    O: for<'a> Observable<'a, T, E>,
{
    fn into_observable(self) -> impl for<'b> Observable<'b, T, E> {
        self
    }
}
