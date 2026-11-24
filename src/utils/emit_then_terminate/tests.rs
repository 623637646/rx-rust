use super::emit_then_terminate;
use crate::{
    observer::{Flow, Observer, Termination},
    thread_mode::mutable::{MutableExt, MutableHelper},
};
use std::cell::RefCell;

type Log = RefCell<Vec<String>>;

/// Logs what it receives, and stops on `stop_on`.
struct Recorder<'a> {
    log: &'a Log,
    stop_on: Option<i32>,
}

impl Observer<Value<'_>, &str> for Recorder<'_> {
    fn on_next(&mut self, value: Value<'_>) -> Flow {
        self.log
            .with_mut(|log| log.push(format!("next {}", value.0)));
        if self.stop_on == Some(value.0) {
            Flow::Break(())
        } else {
            Flow::Continue(())
        }
    }

    fn on_termination(self, termination: Termination<&str>) {
        self.log
            .with_mut(|log| log.push(format!("{termination:?}")));
    }
}

impl Drop for Recorder<'_> {
    fn drop(&mut self) {
        self.log
            .with_mut(|log| log.push("drop observer".to_owned()));
    }
}

/// A value that logs its drop.
struct Value<'a>(i32, &'a Log);

impl Drop for Value<'_> {
    fn drop(&mut self) {
        self.1.with_mut(|log| log.push(format!("drop {}", self.0)));
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
        log.take_value(),
        [
            "next 1",
            "drop 1",
            "next 2",
            "drop 2",
            "Completed",
            "drop observer"
        ]
    );
}

#[test]
fn test_completed_after_iterator_is_dropped() {
    let log = Log::default();
    let observer = Recorder {
        log: &log,
        stop_on: None,
    };
    // Captured by the iterator, which drops it without ever sending it.
    let held = Value(0, &log);
    let values = std::iter::from_fn(move || {
        let _held = &held;
        None
    });

    emit_then_terminate(observer, values, Termination::Completed);
    assert_eq!(log.take_value(), ["drop 0", "Completed", "drop observer"]);
}

#[test]
fn test_completed_without_values() {
    let log = Log::default();
    let observer = Recorder {
        log: &log,
        stop_on: None,
    };

    emit_then_terminate(observer, None, Termination::Completed);
    assert_eq!(log.take_value(), ["Completed", "drop observer"]);
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
    assert_eq!(
        log.take_value(),
        ["next 1", "drop 1", "drop observer", "drop 2"]
    );
}

#[cfg(panic = "unwind")]
#[test]
fn test_panic_on_value() {
    use std::panic::{AssertUnwindSafe, catch_unwind, resume_unwind};

    /// Unwinds out of every value, without running the panic hook, which would print.
    struct Panicking<'a>(#[allow(dead_code)] Recorder<'a>);

    impl Observer<Value<'_>, &str> for Panicking<'_> {
        fn on_next(&mut self, value: Value<'_>) -> Flow {
            drop(value);
            resume_unwind(Box::new(()));
        }

        fn on_termination(self, _: Termination<&str>) {}
    }

    let log = Log::default();
    let observer = Panicking(Recorder {
        log: &log,
        stop_on: None,
    });

    let result = catch_unwind(AssertUnwindSafe(|| {
        emit_then_terminate(
            observer,
            [Value(1, &log), Value(2, &log)],
            Termination::Completed,
        );
    }));
    assert!(result.is_err());
    assert_eq!(log.take_value(), ["drop 1", "drop observer", "drop 2"]);
}

#[test]
fn test_error() {
    let log = Log::default();
    let observer = Recorder {
        log: &log,
        stop_on: None,
    };

    emit_then_terminate(observer, [Value(1, &log)], Termination::Error("error"));
    assert_eq!(
        log.take_value(),
        ["Error(\"error\")", "drop observer", "drop 1"]
    );
}
