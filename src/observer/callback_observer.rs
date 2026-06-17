use super::{Observer, Termination};
use educe::Educe;

#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct CallbackObserver<FN, FT> {
    on_next: FN,
    on_termination: FT,
}

impl<FN, FT> CallbackObserver<FN, FT> {
    pub fn new<T, E>(on_next: FN, on_termination: FT) -> Self
    where
        FN: FnMut(T),
        FT: FnOnce(Termination<E>),
    {
        Self {
            on_next,
            on_termination,
        }
    }
}

impl<T, E, FN, FT> Observer<T, E> for CallbackObserver<FN, FT>
where
    FN: FnMut(T),
    FT: FnOnce(Termination<E>),
{
    fn on_next(&mut self, value: T) {
        (self.on_next)(value);
    }

    fn on_termination(self, termination: Termination<E>) {
        (self.on_termination)(termination);
    }
}
