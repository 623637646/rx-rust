use crate::utils::types::NecessarySend;
use crate::{
    disposable::subscription::Subscription,
    observable::{Observable, observable_ext::ObservableExt},
    observer::Observer,
};
use educe::Educe;

#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct IgnoreElements<OE> {
    source: OE,
}

impl<OE> IgnoreElements<OE> {
    pub fn new(source: OE) -> Self {
        Self { source }
    }
}

impl<'or, 'sub, T, E, OE> Observable<'or, 'sub, T, E> for IgnoreElements<OE>
where
    OE: Observable<'or, 'sub, T, E>,
{
    fn subscribe(self, observer: impl Observer<T, E> + NecessarySend + 'or) -> Subscription<'sub> {
        self.source.filter(|_| false).subscribe(observer)
    }
}
