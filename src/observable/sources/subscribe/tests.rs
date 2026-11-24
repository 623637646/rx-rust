use super::{SourceSubscriber, SubscribeEach};
use crate::{
    disposable::dispose_on_drop::DisposeOnDrop,
    observable::{
        ObservableTypes,
        sources::{IterSources, SourceOps, testing::IgnoreObserver},
    },
    operators::creating::just::Just,
    utils::positional::position::Position,
};
use std::ops::ControlFlow;

/// Records the steps it is asked, by position; it stops running once `stop_after` sources
/// are stored, breaks on the position `full_at`, and does not want the position `unwanted`.
#[derive(Default)]
struct Recorder {
    steps: Vec<String>,
    stored: usize,
    stop_after: Option<usize>,
    full_at: Option<usize>,
    unwanted: Option<usize>,
}

impl<K: Position, OE: ObservableTypes> SourceSubscriber<K, OE, &'static str> for Recorder {
    type Observer = IgnoreObserver;

    fn still_running(&self) -> ControlFlow<&'static str> {
        if self.stop_after == Some(self.stored) {
            ControlFlow::Break("stopped")
        } else {
            ControlFlow::Continue(())
        }
    }

    fn observer(&mut self, position: K) -> Option<IgnoreObserver> {
        if self.unwanted == Some(position.position()) {
            return None;
        }
        self.steps.push(format!("observer {}", position.position()));
        Some(IgnoreObserver)
    }

    fn store(&mut self, position: K, _: DisposeOnDrop<OE::Disposal>) -> ControlFlow<&'static str> {
        self.steps.push(format!("store {}", position.position()));
        self.stored += 1;
        if self.full_at == Some(position.position()) {
            ControlFlow::Break("full")
        } else {
            ControlFlow::Continue(())
        }
    }
}

#[test]
fn test_subscribe_each_runs_the_steps_in_order() {
    let mut recorder = Recorder::default();
    let mut cursor = (Just::new(1), Just::new("b")).into_cursor();
    assert!(cursor.subscribe_each(&mut recorder).is_continue());
    assert_eq!(
        recorder.steps,
        ["observer 0", "store 0", "observer 1", "store 1"]
    );
}

#[test]
fn test_subscribe_each_resumes_after_a_break() {
    let mut recorder = Recorder {
        full_at: Some(0),
        ..Recorder::default()
    };
    let mut cursor = IterSources((0..3).map(Just::new)).into_cursor();
    assert_eq!(
        cursor.subscribe_each(&mut recorder),
        ControlFlow::Break("full")
    );
    assert_eq!(recorder.steps, ["observer 0", "store 0"]);
    assert!(cursor.subscribe_each(&mut recorder).is_continue());
    assert_eq!(recorder.stored, 3);
}

#[test]
fn test_subscribe_each_resumes_a_tuple_after_a_break() {
    let mut recorder = Recorder {
        full_at: Some(1),
        ..Recorder::default()
    };
    let mut cursor = (Just::new(1), Just::new("b"), Just::new('c')).into_cursor();
    assert_eq!(
        cursor.subscribe_each(&mut recorder),
        ControlFlow::Break("full")
    );
    assert_eq!(
        recorder.steps,
        ["observer 0", "store 0", "observer 1", "store 1"]
    );
    assert!(cursor.subscribe_each(&mut recorder).is_continue());
    assert!(cursor.subscribe_each(&mut recorder).is_continue());
    assert_eq!(recorder.stored, 3);
}

#[test]
fn test_subscribe_each_skips_an_unwanted_source() {
    let mut recorder = Recorder {
        unwanted: Some(1),
        ..Recorder::default()
    };
    let mut cursor = (Just::new(1), Just::new(2), Just::new(3)).into_cursor();
    assert!(cursor.subscribe_each(&mut recorder).is_continue());
    assert_eq!(
        recorder.steps,
        ["observer 0", "store 0", "observer 2", "store 2"]
    );
}

#[test]
fn test_subscribe_each_drops_a_pulled_source_once_stopped() {
    let mut recorder = Recorder {
        stop_after: Some(1),
        ..Recorder::default()
    };
    let mut pulled = 0;
    let sources = (0..3).map(|value| {
        pulled += 1;
        Just::new(value)
    });
    let mut cursor = IterSources(sources).into_cursor();
    assert_eq!(
        cursor.subscribe_each(&mut recorder),
        ControlFlow::Break("stopped")
    );
    drop(cursor);
    // The second source was pulled, and dropped without being subscribed.
    assert_eq!(pulled, 2);
    assert_eq!(recorder.steps, ["observer 0", "store 0"]);
}

#[test]
fn test_subscribe_each_asks_no_tuple_source_whether_it_still_runs() {
    let mut recorder = Recorder {
        stop_after: Some(1),
        ..Recorder::default()
    };
    let mut cursor = (Just::new(1), Just::new(2)).into_cursor();
    assert!(cursor.subscribe_each(&mut recorder).is_continue());
    assert_eq!(recorder.stored, 2);
}
