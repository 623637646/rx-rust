use crate::{
    observable::Observable,
    observer::{Observer, Termination},
    subscription::Subscription,
};
use educe::Educe;

#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct FromResult<T, E>(Result<T, E>);

impl<T, E> FromResult<T, E> {
    pub fn new(result: Result<T, E>) -> Self {
        Self(result)
    }
}

impl<'or, 'sub, T, E> Observable<'or, 'sub, T, E> for FromResult<T, E> {
    fn subscribe(self, mut observer: impl Observer<T, E> + Send + 'or) -> Subscription<'sub> {
        match self.0 {
            Ok(value) => {
                observer.on_next(value);
                observer.on_termination(Termination::Completed);
            }
            Err(error) => observer.on_termination(Termination::Error(error)),
        }
        Subscription::new_none_disposal()
    }
}
