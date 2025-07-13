use crate::utils::types::NecessarySend;
use crate::{
    disposable::subscription::Subscription,
    observable::Observable,
    observer::{Observer, Termination},
};
use educe::Educe;
use std::convert::Infallible;

#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct FromIter<I>(I);

impl<I> FromIter<I> {
    pub fn new(into_iterator: I) -> Self
    where
        I: IntoIterator,
    {
        Self(into_iterator)
    }
}

impl<'or, 'sub, T, I> Observable<'or, 'sub, T, Infallible> for FromIter<I>
where
    I: IntoIterator<Item = T>,
{
    fn subscribe(
        self,
        mut observer: impl Observer<T, Infallible> + NecessarySend + 'or,
    ) -> Subscription<'sub> {
        for value in self.0.into_iter() {
            observer.on_next(value);
        }
        observer.on_termination(Termination::Completed);
        Subscription::new_none_disposal()
    }
}
