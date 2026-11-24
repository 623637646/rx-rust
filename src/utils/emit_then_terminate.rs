//! Delivery of values followed by completion, for a synchronous source or an operator that emits
//! values as it completes.

use crate::observer::{Observer, Termination};

/// Ends the stream of `observer` with `termination`, sending `values` first if it is a completion.
///
/// The values end the stream, so a downstream that stops on one of them is not completed on top of
/// that: it has already ended itself, and is dropped, then the values after it. A completion
/// follows the drop of the iterator the values came from. An error drops the values unsent, once it
/// is delivered, where they would otherwise be dropped with the operator's observer.
pub(crate) fn emit_then_terminate<T, E>(
    observer: impl Observer<T, E>,
    values: impl IntoIterator<Item = T>,
    termination: Termination<E>,
) {
    // The observer is declared last, so that a stop drops it before the values left, an unwind
    // included (decision 0011).
    let (mut values, mut observer) = (values.into_iter(), observer);
    if let Termination::Completed = termination {
        if values
            .try_for_each(|value| observer.on_next(value))
            .is_break()
        {
            return;
        }
        // No value is left, but the iterator may hold more than values (a file, for one): gone
        // before whatever the completion starts.
        drop(values);
    }
    observer.on_termination(termination);
}

#[cfg(test)]
mod tests;
