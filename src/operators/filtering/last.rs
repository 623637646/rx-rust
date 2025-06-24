use crate::{
    observable::{Observable, observable_ext::ObservableExt},
    observer::Observer,
    subscription::Subscription,
};
use educe::Educe;

#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct Last<OE> {
    source: OE,
}

impl<OE> Last<OE> {
    pub fn new(source: OE) -> Self {
        Self { source }
    }
}

impl<'or, 'sub, T, E, OE> Observable<'or, 'sub, T, E> for Last<OE>
where
    T: Send + 'or,
    OE: Observable<'or, 'sub, T, E>,
    'sub: 'or,
{
    fn subscribe(self, observer: impl Observer<T, E> + Send + 'or) -> Subscription<'sub> {
        self.source.take_last(1).subscribe(observer)
    }
}
