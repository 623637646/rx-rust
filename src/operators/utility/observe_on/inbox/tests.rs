use super::InboxHandle;
use crate::{
    observer::{Event, Termination},
    thread_mode::Local,
};

#[test]
fn test_take_after_termination_leaves_no_buffer() {
    for termination in [Termination::Completed, Termination::Error("error")] {
        let inbox = InboxHandle::<Local, _, _>::new();
        inbox.push(Event::Next(111));
        inbox.push(Event::Next(222));
        inbox.push(Event::Termination(termination.clone()));

        let (values, taken_termination) = inbox.take().unwrap();
        assert_eq!(values, [111, 222]);
        assert_eq!(taken_termination, Some(termination));
        inbox.0.with_mut(|slot| {
            assert_eq!(slot.as_ref().unwrap().values.capacity(), 0);
        });
    }
}
