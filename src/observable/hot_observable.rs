use super::Observable;
use crate::{
    cancellable::{non_cancellable::NonCancellable, Cancellable},
    observer::Observer,
};
use std::{cell::RefCell, rc::Rc};

// TODO: Implement HotObservable

struct HotObservable<T, E> {
    observers: Rc<RefCell<Vec<Box<dyn Observer<T, E>>>>>,
}

impl<'a, T, E> Observable<'a, T, E> for HotObservable<T, E> {
    fn subscribe<O>(&'a self, observer: O) -> impl Cancellable
    where
        O: Observer<T, E> + 'static,
    {
        self.observers.borrow_mut().push(Box::new(observer));
        NonCancellable {}
    }
}
