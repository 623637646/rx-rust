use super::subscribe_with_context_two_steps;
use crate::{
    disposable::{Disposable, callback_disposal::CallbackDisposal, dispose_on_drop::DisposeOnDrop},
    observer::{Termination, callback_observer::CallbackObserver},
    thread_mode::Shared,
};
use std::{
    convert::Infallible,
    sync::{
        Arc, Barrier,
        atomic::{AtomicBool, Ordering},
    },
    thread,
};

/// Raises its flag as it is dropped: what tells an observer that was released from one that
/// was terminated.
struct DropFlag(Arc<AtomicBool>);

impl Drop for DropFlag {
    fn drop(&mut self) {
        self.0.store(true, Ordering::SeqCst);
    }
}

// A step that ends the stream while the subscription is still being assembled keeps the step
// after it from subscribing at all, and what it did subscribe is released at once.
#[test]
fn test_termination_in_the_first_step_skips_the_second_and_releases_the_first() {
    let disposed = Arc::new(AtomicBool::new(false));
    let disposed_first = disposed.clone();
    let completed = Arc::new(AtomicBool::new(false));
    let completed_observer = completed.clone();
    let observer = CallbackObserver::new(
        |_: i32| {},
        move |termination: Termination<Infallible>| {
            assert_eq!(termination, Termination::Completed);
            completed_observer.store(true, Ordering::SeqCst);
        },
    );

    let _subscription = subscribe_with_context_two_steps::<Shared, _, _, _, _, _, _, _, _>(
        observer,
        (),
        |context| {
            let subscription = DisposeOnDrop::new(CallbackDisposal::new(move || {
                disposed_first.store(true, Ordering::SeqCst)
            }));
            context.send_termination(Termination::Completed);
            Some(subscription)
        },
        |_| -> Option<DisposeOnDrop<()>> {
            panic!("a stream that is over must not subscribe the step after it")
        },
    );

    assert!(disposed.load(Ordering::SeqCst));
    assert!(completed.load(Ordering::SeqCst));
}

// `None` ends the assembly: the step after it does not run. With no step left holding a clone
// of the context, the observer is released as the subscription is handed back — without a
// termination, the way a source letting go of it would end the stream.
#[test]
fn test_a_first_step_that_subscribes_nothing_skips_the_second() {
    let released = Arc::new(AtomicBool::new(false));
    let flag = DropFlag(released.clone());
    let observer = CallbackObserver::new(
        move |_: i32| {
            let _ = &flag;
        },
        |_: Termination<Infallible>| panic!("the observer must be released, not terminated"),
    );

    let _subscription = subscribe_with_context_two_steps::<Shared, _, _, _, _, _, _, _, _>(
        observer,
        (),
        |_| -> Option<DisposeOnDrop<()>> { None },
        |_| -> Option<DisposeOnDrop<()>> {
            panic!("the step after one that subscribed nothing must not run")
        },
    );

    assert!(released.load(Ordering::SeqCst));
}

// A stop concurrent with a step, which needs the context's own disposal: the public surface
// only hands one out once the assembly has returned.
#[test]
fn test_disposes_steps_started_before_a_concurrent_stop() {
    let disposed = Arc::new(AtomicBool::new(false));
    let disposed_source = disposed.clone();
    let observer = CallbackObserver::new(
        |_: i32| {},
        |_: Termination<Infallible>| {
            panic!("disposal must not terminate the observer");
        },
    );
    let _subscription = subscribe_with_context_two_steps::<Shared, _, _, _, _, _, _, _, _>(
        observer,
        (),
        |_| Some(DisposeOnDrop::new(())),
        |context| {
            let barrier = Barrier::new(2);
            thread::scope(|scope| {
                let stop = context.disposal();
                let barrier_thread = &barrier;
                scope.spawn(move || {
                    barrier_thread.wait();
                    stop.dispose();
                    barrier_thread.wait();
                });
                barrier.wait();
                barrier.wait();
                Some(DisposeOnDrop::new(CallbackDisposal::new(move || {
                    disposed_source.store(true, Ordering::SeqCst)
                })))
            })
        },
    );
    assert!(disposed.load(Ordering::SeqCst));
}
