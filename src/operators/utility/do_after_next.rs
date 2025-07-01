use crate::{
    observable::{Observable, observable_ext::ObservableExt},
    observer::Observer,
    subscription::Subscription,
};
use educe::Educe;

#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct DoAfterNext<OE, F> {
    source: OE,
    callback: F,
}

impl<OE, F> DoAfterNext<OE, F> {
    pub fn new<'or, 'sub, T, E>(source: OE, callback: F) -> Self
    where
        OE: Observable<'or, 'sub, T, E>,
        F: FnMut(T),
    {
        Self { source, callback }
    }
}

impl<'or, 'sub, T, E, OE, F> Observable<'or, 'sub, T, E> for DoAfterNext<OE, F>
where
    T: Clone,
    OE: Observable<'or, 'sub, T, E>,
    F: FnMut(T) + Send + 'or,
{
    fn subscribe(mut self, observer: impl Observer<T, E> + Send + 'or) -> Subscription<'sub> {
        self.source
            .hook_on_next(move |observer, value| {
                observer.on_next(value.clone());
                (self.callback)(value);
            })
            .subscribe(observer)
    }
}
