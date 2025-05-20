use crate::{
    observable::{Observable, observable_ext::ObservableExt},
    observer::{Observer, Termination},
    subscription::Subscription,
};
use std::convert::Infallible;

impl<'or, 'sub, T, I> Observable<'or, 'sub, T, Infallible> for I
where
    I: IntoIterator<Item = T>,
{
    fn subscribe(
        self,
        mut observer: impl Observer<T, Infallible> + Send + 'or,
    ) -> Subscription<'sub> {
        for value in self.into_iter() {
            observer.on_next(value);
        }
        observer.on_termination(Termination::Completed);
        Subscription::new_none_disposal()
    }
}

impl<I> ObservableExt for I where I: IntoIterator {}
