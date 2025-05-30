use crate::{
    observable::{Observable, observable_ext::ObservableExt},
    observer::{Observer, Termination},
    subscription::Subscription,
};
use educe::Educe;

#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct DoOnTermination<OE, F> {
    source: OE,
    callback: F,
}

impl<OE, F> DoOnTermination<OE, F> {
    pub fn new<'or, 'sub, T, E>(source: OE, callback: F) -> Self
    where
        OE: Observable<'or, 'sub, T, E>,
        F: FnOnce(&Termination<E>),
    {
        Self { source, callback }
    }
}

impl<'or, 'sub, T, E, OE, F> Observable<'or, 'sub, T, E> for DoOnTermination<OE, F>
where
    OE: Observable<'or, 'sub, T, E>,
    F: FnOnce(&Termination<E>) + Send + 'or,
{
    fn subscribe(self, observer: impl Observer<T, E> + Send + 'or) -> Subscription<'sub> {
        self.source
            .hook_on_termination(move |termination, original| {
                (self.callback)(&termination);
                original(termination)
            })
            .subscribe(observer)
    }
}
