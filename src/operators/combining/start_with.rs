use crate::utils::types::NecessarySend;
use crate::{disposable::subscription::Subscription, observable::Observable, observer::Observer};
use educe::Educe;

#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct StartWith<OE, I> {
    source: OE,
    values: I,
}

impl<OE, I> StartWith<OE, I> {
    pub fn new<'or, 'sub, T, E>(source: OE, values: I) -> Self
    where
        OE: Observable<'or, 'sub, T, E>,
        I: IntoIterator<Item = T>,
    {
        Self { source, values }
    }
}

impl<'or, 'sub, T, E, OE, I> Observable<'or, 'sub, T, E> for StartWith<OE, I>
where
    OE: Observable<'or, 'sub, T, E>,
    I: IntoIterator<Item = T>,
{
    fn subscribe(
        self,
        mut observer: impl Observer<T, E> + NecessarySend + 'or,
    ) -> Subscription<'sub> {
        for value in self.values.into_iter() {
            observer.on_next(value);
        }
        self.source.subscribe(observer)
    }
}
