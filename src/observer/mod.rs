pub mod boxed_observer;
pub mod callback_observer;

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
    ///
    /// # Arguments
    ///
    /// * `value` - The next value produced by the operation.
    fn on_next(&mut self, value: T);

    /// Called when the operation has reached its termination state.
    ///
    /// # Arguments
    ///
    /// * `termination` - The termination state of the operation, which can either be `Completed` or `Error`.
    fn on_termination(self, termination: Termination<E>);
}
