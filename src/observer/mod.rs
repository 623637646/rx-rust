/// Represents the terminal state of an operation, which can either be completed successfully or with an error.
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Terminal<E> {
    /// Indicates that the operation has completed successfully.
    Completed,
    /// Indicates that the operation has completed with an error.
    Error(E),
}

/// A trait for observing the progress and terminal state of an operation.
pub trait Observer<T, E> {
    /// Called when the next value in the operation is available.
    ///
    /// # Arguments
    ///
    /// * `value` - The next value produced by the operation.
    fn on_next(&mut self, value: T);

    /// Called when the operation has reached its terminal state.
    ///
    /// # Arguments
    ///
    /// * `terminal` - The terminal state of the operation, which can either be `Completed` or `Error`.
    fn on_terminal(self, terminal: Terminal<E>);
}
