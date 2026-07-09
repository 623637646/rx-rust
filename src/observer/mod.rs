pub mod boxed_observer;
pub mod callback_observer;

use crate::{observer::boxed_observer::BoxedObserver, utils::types::MaybeSend};
use educe::Educe;

/// Represents the termination state of an operation, which can either be completed successfully or with an error.
#[derive(Educe)]
#[educe(Debug, Clone, PartialEq, Eq)]
pub enum Termination<E> {
    /// Indicates that the operation has completed successfully.
    Completed,
    /// Indicates that the operation has completed with an error.
    Error(E),
}

/// A trait for observing the progress and termination state of an operation.
pub trait Observer<T, E> {
    /// Called when the next value in the operation is available.
    fn on_next(&mut self, value: T);

    /// Called when the operation has reached its termination state.
    fn on_termination(self, termination: Termination<E>);
}

#[derive(Educe)]
#[educe(Debug, Clone, PartialEq, Eq)]
pub enum Event<T, E> {
    Next(T),
    Termination(Termination<E>),
}

pub trait BoxedObserverExt<T, E>: Observer<T, E> + Sized {
    fn into_boxed<'or>(self) -> BoxedObserver<'or, T, E>
    where
        Self: MaybeSend + 'or,
    {
        BoxedObserver::new(self)
    }
}

impl<T, E, OR> BoxedObserverExt<T, E> for OR where OR: Observer<T, E> {}
