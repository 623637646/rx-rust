use super::{Events, Groups};
use crate::observer::{Flow, Observer, Termination};
use std::convert::Infallible;

#[test]
fn test_events_hold_one_without_a_vec() {
    let mut events = Events::new();
    assert!(Observer::<_, Infallible>::on_next(&mut events, 1).is_continue());
    assert!(matches!(events, Events::One(1)));
    let _ = Observer::<_, Infallible>::on_next(&mut events, 2);
    let _ = Observer::<_, Infallible>::on_next(&mut events, 3);
    assert_eq!(events.into_vec(), [1, 2, 3]);
}

/// An observer that records what it receives and stops on `stop_on`.
struct StopOn {
    stop_on: &'static str,
    received: Vec<&'static str>,
}

impl Observer<&'static str, Infallible> for StopOn {
    fn on_next(&mut self, value: &'static str) -> Flow {
        self.received.push(value);
        if value == self.stop_on {
            Flow::Break(())
        } else {
            Flow::Continue(())
        }
    }

    fn on_termination(self, _: Termination<Infallible>) {}
}

/// Groups that emit what each hook was asked, to check the order of the default methods.
struct Log;

impl Groups<Infallible> for Log {
    type Item = ();
    type Event = &'static str;

    fn open(&mut self, observer: &mut impl Observer<&'static str, Infallible>) -> Flow {
        observer.on_next("open")
    }

    fn put(&mut self, _: &mut Option<()>, _: &mut impl Observer<&'static str, Infallible>) -> Flow {
        Flow::Continue(())
    }

    fn close_oldest(&mut self, observer: &mut impl Observer<&'static str, Infallible>) -> Flow {
        observer.on_next("close")
    }

    fn end(
        &mut self,
        _: &Termination<Infallible>,
        _: &mut impl Observer<&'static str, Infallible>,
    ) -> Flow {
        Flow::Continue(())
    }
}

#[test]
fn test_rotate_closes_the_oldest_before_it_opens_the_next() {
    let mut events = Events::new();
    assert!(Log.rotate(&mut events).is_continue());
    assert_eq!(events.into_vec(), ["close", "open"]);
}

#[test]
fn test_rotate_opens_nothing_after_a_stop() {
    let mut observer = StopOn {
        stop_on: "close",
        received: Vec::new(),
    };
    assert!(Log.rotate(&mut observer).is_break());
    assert_eq!(observer.received, ["close"]);
}
