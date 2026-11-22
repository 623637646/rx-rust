//! Delivery of values followed by completion, for a synchronous source or an operator that emits
//! values as it completes.

use crate::observer::{Observer, Termination};

/// Ends the stream of `observer` with `termination`, sending `values` first if it is a completion.
///
/// The values end the stream, so a downstream that stops on one of them is not completed on top of
/// that: it has already ended itself, and the values after it are dropped. An error drops the
/// values unsent, once it is delivered, where they would otherwise be dropped with the operator's
/// observer.
pub(crate) fn emit_then_terminate<T, E>(
    mut observer: impl Observer<T, E>,
    values: impl IntoIterator<Item = T>,
    termination: Termination<E>,
) {
    match termination {
        Termination::Completed => {
            let flow = values
                .into_iter()
                .try_for_each(|value| observer.on_next(value));
            if flow.is_continue() {
                observer.on_termination(termination);
            }
        }
        Termination::Error(_) => {
            observer.on_termination(termination);
            drop(values);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::emit_then_terminate;
    use crate::observer::{Flow, Observer, Termination};
    use std::cell::RefCell;

    type Log = RefCell<Vec<String>>;

    /// Logs what it receives, and stops on `stop_on`.
    struct Recorder<'a> {
        log: &'a Log,
        stop_on: Option<i32>,
    }

    impl Observer<Value<'_>, &str> for Recorder<'_> {
        fn on_next(&mut self, value: Value<'_>) -> Flow {
            self.log.borrow_mut().push(format!("next {}", value.0));
            if self.stop_on == Some(value.0) {
                Flow::Break(())
            } else {
                Flow::Continue(())
            }
        }

        fn on_termination(self, termination: Termination<&str>) {
            self.log.borrow_mut().push(format!("{termination:?}"));
        }
    }

    /// A value that logs its drop.
    struct Value<'a>(i32, &'a Log);

    impl Drop for Value<'_> {
        fn drop(&mut self) {
            self.1.borrow_mut().push(format!("drop {}", self.0));
        }
    }

    #[test]
    fn test_completed() {
        let log = Log::default();
        let observer = Recorder {
            log: &log,
            stop_on: None,
        };

        emit_then_terminate(
            observer,
            [Value(1, &log), Value(2, &log)],
            Termination::Completed,
        );
        assert_eq!(
            log.take(),
            ["next 1", "drop 1", "next 2", "drop 2", "Completed"]
        );
    }

    #[test]
    fn test_completed_without_values() {
        let log = Log::default();
        let observer = Recorder {
            log: &log,
            stop_on: None,
        };

        emit_then_terminate(observer, None, Termination::Completed);
        assert_eq!(log.take(), ["Completed"]);
    }

    #[test]
    fn test_stop_on_value() {
        let log = Log::default();
        let observer = Recorder {
            log: &log,
            stop_on: Some(1),
        };

        emit_then_terminate(
            observer,
            [Value(1, &log), Value(2, &log)],
            Termination::Completed,
        );
        assert_eq!(log.take(), ["next 1", "drop 1", "drop 2"]);
    }

    #[test]
    fn test_error() {
        let log = Log::default();
        let observer = Recorder {
            log: &log,
            stop_on: None,
        };

        emit_then_terminate(observer, [Value(1, &log)], Termination::Error("error"));
        assert_eq!(log.take(), ["Error(\"error\")", "drop 1"]);
    }
}
