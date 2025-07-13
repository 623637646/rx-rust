use crate::utils::types::NecessarySend;
use crate::{
    disposable::subscription::Subscription,
    observable::{Observable, observable_ext::ObservableExt},
    observer::Observer,
};
use educe::Educe;

#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct First<OE> {
    source: OE,
}

impl<OE> First<OE> {
    pub fn new(source: OE) -> Self {
        Self { source }
    }
}

impl<'or, 'sub, T, E, OE> Observable<'or, 'sub, T, E> for First<OE>
where
    OE: Observable<'or, 'sub, T, E>,
    'sub: 'or,
{
    fn subscribe(self, observer: impl Observer<T, E> + NecessarySend + 'or) -> Subscription<'sub> {
        // Or `self.source.take(1).subscribe(observer)`
        self.source.element_at(0).subscribe(observer)
    }
}
