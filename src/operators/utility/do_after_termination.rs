use crate::utils::types::NecessarySend;
use crate::{
    disposable::subscription::Subscription,
    observable::{Observable, observable_ext::ObservableExt},
    observer::{Observer, Termination},
};
use educe::Educe;

#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct DoAfterTermination<OE, F> {
    source: OE,
    callback: F,
}

impl<OE, F> DoAfterTermination<OE, F> {
    pub fn new<'or, 'sub, T, E>(source: OE, callback: F) -> Self
    where
        OE: Observable<'or, 'sub, T, E>,
        F: FnOnce(Termination<E>),
    {
        Self { source, callback }
    }
}

impl<'or, 'sub, T, E, OE, F> Observable<'or, 'sub, T, E> for DoAfterTermination<OE, F>
where
    T: 'or,
    E: Clone + 'or,
    OE: Observable<'or, 'sub, T, E>,
    F: FnOnce(Termination<E>) + NecessarySend + 'or,
{
    fn subscribe(self, observer: impl Observer<T, E> + NecessarySend + 'or) -> Subscription<'sub> {
        self.source
            .hook_on_termination(move |observer, termination| {
                observer.on_termination(termination.clone());
                (self.callback)(termination);
            })
            .subscribe(observer)
    }
}
