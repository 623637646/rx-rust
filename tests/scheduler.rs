mod tests_utils;

use crate::main_loop::MainLoop;
use crate::tests_utils::DURATION_10_MS;
use crate::tests_utils::DURATION_30_MS;
use crate::tests_utils::DURATION_100_MS;
use crate::tests_utils::drop_probe::DropProbe;
use crate::tests_utils::test_scheduler::block_on;
use futures::StreamExt;
use rx_rust::disposable::Disposable;
use rx_rust::observable::ObservableExt;
use rx_rust::observer::{Observer, Termination};
use rx_rust::operators::creating::just::Just;
use rx_rust::scheduler::virtual_time::VirtualTime;
use rx_rust::scheduler::{Scheduler, SchedulerExt, Task, TaskState, drive};
use rx_rust::subject::publish_subject::PublishSubject;
use rx_rust::thread_mode::mutable::{MutableBoolHelper, MutableExt, MutableHelper};
use std::convert::Infallible;
use std::pin::pin;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex, mpsc};
use std::task::{Context, Poll, Waker};
use std::thread;
use std::time::{Duration, Instant};

const RECURSION_EXECUTION_TIMES: usize = 200;

#[test]
fn test_schedule_without_delay() {
    block_on(|scheduler| async move {
        let (tx, rx) = futures::channel::oneshot::channel();
        let task = || {
            tx.send(()).unwrap();
        };
        let start_time = Instant::now();
        let disposal = scheduler.schedule(task, None);
        assert!(rx.await.is_ok());
        let elapsed_time = start_time.elapsed();
        assert!(elapsed_time < DURATION_30_MS);
        disposal.dispose();
    });
}

#[test]
fn test_schedule_with_delay() {
    block_on(|scheduler| async move {
        let (tx, rx) = futures::channel::oneshot::channel();
        let task = || {
            tx.send(()).unwrap();
        };
        let start_time = Instant::now();
        let disposal = scheduler.schedule(task, Some(DURATION_100_MS));
        assert!(rx.await.is_ok());
        let elapsed_time = start_time.elapsed();
        assert!(elapsed_time >= DURATION_100_MS);
        assert!(elapsed_time < DURATION_100_MS + DURATION_30_MS);
        disposal.dispose();
    });
}

#[test]
fn test_schedule_with_abort() {
    block_on(|scheduler| async move {
        let (tx, rx) = futures::channel::oneshot::channel();
        let task = || {
            tx.send(()).unwrap();
        };
        let start_time = Instant::now();
        let disposal = scheduler.schedule(task, Some(DURATION_100_MS));
        disposal.dispose();
        assert!(rx.await.is_err());
        let elapsed_time = start_time.elapsed();
        assert!(elapsed_time < DURATION_30_MS);
    });
}

// A task disposed while it sleeps between two steps is dropped at once, with its context, rather
// than when its sleep would have ended: every runtime wakes a task it aborts.
#[test]
fn test_schedule_with_abort_while_sleeping() {
    block_on(|scheduler| async move {
        let (tx, mut rx) = futures::channel::mpsc::unbounded();
        let task = Task::new(tx, |tx, _, _, _| {
            let _ = tx.unbounded_send(());
            Poll::Ready(TaskState::SleepUntil(
                Instant::now() + Duration::from_secs(1),
            ))
        });
        let disposal = scheduler.run_task(task, None);
        // The first step has run: the task now sleeps.
        assert!(rx.next().await.is_some());
        let start_time = Instant::now();
        disposal.dispose();
        // Dropping the task drops `tx`, which closes the channel.
        assert!(rx.next().await.is_none());
        assert!(start_time.elapsed() < DURATION_100_MS);
    });
}

#[test]
fn test_schedule_with_late_abort() {
    block_on(|scheduler| async move {
        let (tx, rx) = futures::channel::oneshot::channel();
        let task = || {
            tx.send(()).unwrap();
        };
        let disposal = scheduler.schedule(task, None);
        scheduler.sleep(DURATION_10_MS).await;
        disposal.dispose();
        assert!(rx.await.is_ok());
    });
}

#[test]
fn test_schedule_recursively_without_delay() {
    block_on(|scheduler| async move {
        let (tx, mut rx) = futures::channel::mpsc::unbounded();
        let mut tx = Some(tx);
        let first = Instant::now();
        let start_instant = Instant::now();
        let disposal = scheduler.schedule_recursively(
            move |index, _| {
                if index == RECURSION_EXECUTION_TIMES {
                    tx.take().unwrap();
                    TaskState::Finished
                } else {
                    tx.as_ref().unwrap().unbounded_send(Instant::now()).unwrap();
                    TaskState::SleepUntil(first + DURATION_10_MS * (index as u32 + 1))
                }
            },
            None,
        );
        let mut count = 0;
        while let Some(call_instant) = rx.next().await {
            let duration = call_instant - start_instant;
            let diff = duration - (count as u32 * DURATION_10_MS);
            assert!(diff < DURATION_30_MS, "diff: {diff:?}, count: {count}");
            count += 1;
        }
        assert_eq!(count, RECURSION_EXECUTION_TIMES);
        disposal.dispose();
    });
}

#[test]
fn test_schedule_recursively_with_delay() {
    block_on(|scheduler| async move {
        let (tx, mut rx) = futures::channel::mpsc::unbounded();
        let mut tx = Some(tx);
        let first = Instant::now() + DURATION_10_MS;
        let start_instant = Instant::now();
        let disposal = scheduler.schedule_recursively(
            move |index, _| {
                if index == RECURSION_EXECUTION_TIMES {
                    tx.take().unwrap();
                    TaskState::Finished
                } else {
                    tx.as_ref().unwrap().unbounded_send(Instant::now()).unwrap();
                    TaskState::SleepUntil(first + DURATION_10_MS * (index as u32 + 1))
                }
            },
            Some(DURATION_10_MS),
        );
        let mut count = 0;
        while let Some(call_instant) = rx.next().await {
            let duration = call_instant - start_instant;
            let diff = duration - (count as u32 * DURATION_10_MS);
            assert!(diff < DURATION_30_MS, "diff: {diff:?}, count: {count}");
            count += 1;
        }
        assert_eq!(count, RECURSION_EXECUTION_TIMES);
        disposal.dispose();
    });
}

// For panic `overflow when subtracting durations` in `let delay = delay - diff`.
#[test]
fn test_schedule_recursively_small_delay() {
    block_on(|scheduler| async move {
        let (tx, mut rx) = futures::channel::mpsc::unbounded();
        let mut tx = Some(tx);
        let small = DURATION_10_MS;
        let first = Instant::now() + small;
        let disposal = scheduler.schedule_recursively(
            move |index, _| {
                if index == RECURSION_EXECUTION_TIMES {
                    tx.take().unwrap();
                    TaskState::Finished
                } else {
                    tx.as_ref().unwrap().unbounded_send(()).unwrap();
                    TaskState::SleepUntil(first + small * (index as u32 + 1))
                }
            },
            Some(small),
        );
        let mut count = 0;
        while (rx.next().await).is_some() {
            count += 1;
        }
        assert_eq!(count, RECURSION_EXECUTION_TIMES);
        disposal.dispose();
    });
}

// `Yield` must yield between iterations: otherwise the first
// receive below would starve on a single-threaded pool, and disposal could
// never take effect (abort/cancel only happens at await points).
#[test]
fn test_schedule_recursively_continue_immediately_yields_and_disposes() {
    block_on(|scheduler| async move {
        let (tx, mut rx) = futures::channel::mpsc::unbounded();
        let disposal = scheduler.schedule_recursively(
            move |index, _| {
                // The receiver may be gone after disposal races; ignore errors.
                let _ = tx.unbounded_send(index);
                TaskState::Yield
            },
            None,
        );
        // Requires the recursive loop to yield to this task.
        assert!(rx.next().await.is_some());
        disposal.dispose();
        // After disposal the task is dropped, dropping `tx` and closing the
        // channel. If disposal didn't stop the loop, this would hang forever.
        while rx.next().await.is_some() {}
    });
}

// Same guarantee for `SleepUntil` with an instant that has already passed.
#[test]
fn test_schedule_recursively_continue_at_past_instant_yields_and_disposes() {
    block_on(|scheduler| async move {
        let (tx, mut rx) = futures::channel::mpsc::unbounded();
        let past = Instant::now();
        let disposal = scheduler.schedule_recursively(
            move |index, _| {
                let _ = tx.unbounded_send(index);
                // Always in the past by the time it is evaluated.
                TaskState::SleepUntil(past)
            },
            None,
        );
        assert!(rx.next().await.is_some());
        disposal.dispose();
        while rx.next().await.is_some() {}
    });
}

#[test]
fn test_schedule_periodically_without_delay() {
    block_on(|scheduler| async move {
        let (tx, mut rx) = futures::channel::mpsc::unbounded();
        let mut tx = Some(tx);
        let start_instant = Instant::now();
        let disposal = scheduler.schedule_periodically(
            move |index| {
                if index == RECURSION_EXECUTION_TIMES {
                    tx.take().unwrap();
                    false
                } else {
                    tx.as_ref().unwrap().unbounded_send(Instant::now()).unwrap();
                    true
                }
            },
            DURATION_10_MS,
            None,
        );
        let mut count = 0;
        while let Some(call_instant) = rx.next().await {
            let duration = call_instant - start_instant;
            let diff = duration - (count as u32 * DURATION_10_MS);
            assert!(diff < DURATION_30_MS, "diff: {diff:?}, count: {count}");
            count += 1;
        }
        assert_eq!(count, RECURSION_EXECUTION_TIMES);
        disposal.dispose();
    });
}

#[test]
fn test_schedule_periodically_with_delay() {
    block_on(|scheduler| async move {
        let (tx, mut rx) = futures::channel::mpsc::unbounded();
        let mut tx = Some(tx);
        let start_instant = Instant::now();
        let disposal = scheduler.schedule_periodically(
            move |index| {
                if index == RECURSION_EXECUTION_TIMES {
                    tx.take().unwrap();
                    false
                } else {
                    tx.as_ref().unwrap().unbounded_send(Instant::now()).unwrap();
                    true
                }
            },
            DURATION_10_MS,
            Some(DURATION_10_MS),
        );
        let mut count = 0;
        while let Some(call_instant) = rx.next().await {
            let duration = call_instant - start_instant;
            let diff = duration - (count as u32 * DURATION_10_MS);
            assert!(diff < DURATION_30_MS, "diff: {diff:?}, count: {count}");
            count += 1;
        }
        assert_eq!(count, RECURSION_EXECUTION_TIMES);
        disposal.dispose();
    });
}

/// The rate is fixed: the calls a slow call made late run back to back once it returns, none is
/// skipped, and the calls after them are back on the schedule set when the task was scheduled.
#[test]
fn test_schedule_periodically_catches_up_after_an_overrun() {
    block_on(|scheduler| async move {
        // Three and a half periods: the calls due at 10, 20 and 30 ms are all late once it ends.
        let overrun = Duration::from_millis(35);
        let (tx, mut rx) = futures::channel::mpsc::unbounded();
        let mut tx = Some(tx);
        let start_instant = Instant::now();
        let disposal = scheduler.schedule_periodically(
            move |index| {
                if index == 6 {
                    tx.take().unwrap();
                    return false;
                }
                tx.as_ref().unwrap().unbounded_send(Instant::now()).unwrap();
                if index == 0 {
                    thread::sleep(overrun);
                }
                true
            },
            DURATION_10_MS,
            None,
        );
        let mut calls = Vec::new();
        while let Some(call_instant) = rx.next().await {
            calls.push(call_instant - start_instant);
        }
        assert_eq!(calls.len(), 6);
        for (index, call) in calls.iter().enumerate() {
            // Never early: every call keeps its slot on the schedule.
            assert!(*call >= index as u32 * DURATION_10_MS, "calls: {calls:?}");
        }
        // The late calls run as soon as the slow one returns, not a period apart.
        assert!(calls[1] >= overrun, "calls: {calls:?}");
        assert!(calls[3] - calls[1] < DURATION_10_MS, "calls: {calls:?}");
        // The next one is due at 40 ms, as if no call had been late.
        assert!(
            calls[4] < 4 * DURATION_10_MS + DURATION_30_MS,
            "calls: {calls:?}"
        );
        disposal.dispose();
    });
}

#[test]
#[should_panic(expected = "period must be non-zero")]
fn test_schedule_periodically_rejects_zero_period() {
    block_on(|scheduler| async move {
        let _disposal = scheduler.schedule_periodically(|_| false, Duration::ZERO, None);
    });
}

/// A delay too long for an `Instant` to represent never ends, whatever the runtime's timer makes of
/// it: `drive` never calls `sleep` with it (tokio's would wake after some thirty years), nor steps
/// the task.
#[test]
fn test_drive_delay_too_long_never_ends() {
    let time = VirtualTime::new();
    let slept = Arc::new(AtomicBool::new(false));
    let ran = Arc::new(AtomicBool::new(false));
    let slept_sleep = slept.clone();
    let ran_task = ran.clone();
    let task = Task::once(move || ran_task.write(true), |task| task());

    let mut future = pin!(drive(
        task,
        Some(Duration::MAX),
        time.scheduler(),
        move |_| {
            slept_sleep.write(true);
            std::future::ready(())
        },
    ));
    let mut cx = Context::from_waker(Waker::noop());
    assert!(future.as_mut().poll(&mut cx).is_pending());
    assert!(future.as_mut().poll(&mut cx).is_pending());
    assert!(!slept.read());
    assert!(!ran.read());
}

#[test]
fn test_schedule_stream() {
    block_on(|scheduler| async move {
        let (tx, mut rx) = futures::channel::mpsc::unbounded();
        let _disposal = scheduler.schedule_stream(futures::stream::iter([1, 2, 3]), move |item| {
            tx.unbounded_send(item).unwrap();
            true
        });
        let mut results = Vec::new();
        while let Some(result) = rx.next().await {
            results.push(result);
        }
        // Each element arrives, and the end of the stream is announced with a `None`.
        assert_eq!(results, [Some(1), Some(2), Some(3), None]);
    });
}

#[test]
fn test_schedule_stream_stops_on_false() {
    block_on(|scheduler| async move {
        let (tx, mut rx) = futures::channel::mpsc::unbounded();
        let stream = futures::stream::iter(1..);
        let _disposal = scheduler.schedule_stream(stream, move |item| {
            let item = item.unwrap();
            tx.unbounded_send(item).unwrap();
            item < 2
        });
        // A `false` answer ends the task, which drops `tx` and closes the channel. If the loop
        // kept polling the infinite stream, this would hang forever.
        let mut results = Vec::new();
        while let Some(result) = rx.next().await {
            results.push(result);
        }
        // The final `None` is not delivered after a stop: it would `unwrap` above.
        assert_eq!(results, [1, 2]);
    });
}

#[test]
fn test_tokio_schedules_outside_runtime_context() {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("Failed building the Runtime");
    let scheduler =
        rx_rust::scheduler::runtime::tokio::TokioScheduler::from_handle(runtime.handle().clone());
    let (tx, rx) = futures::channel::oneshot::channel();

    // Not on a thread of the runtime, nor inside it: the scheduler holds its handle.
    let disposal = scheduler.schedule(
        move || tx.send(()).expect("receiver should remain alive"),
        Some(DURATION_10_MS),
    );

    assert!(runtime.block_on(rx).is_ok());
    disposal.dispose();
}

#[test]
fn test_tokio_local_ambient_runs_on_the_current_local_set() {
    use rx_rust::scheduler::runtime::tokio::TokioLocalScheduler;
    use std::{cell::Cell, rc::Rc};

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("Failed building the Runtime");
    let local_set = tokio::task::LocalSet::new();
    // Not `Send`: only a single-threaded scheduler accepts it.
    let ran = Rc::new(Cell::new(false));
    let ran_in_task = ran.clone();
    local_set.block_on(&runtime, async {
        let (tx, rx) = futures::channel::oneshot::channel();
        let _disposal = TokioLocalScheduler::ambient().schedule(
            move || {
                ran_in_task.set(true);
                tx.send(()).unwrap();
            },
            Some(DURATION_10_MS),
        );
        assert!(rx.await.is_ok());
    });
    assert!(ran.get());
}

#[test]
#[should_panic(expected = "LocalSet")]
fn test_tokio_local_ambient_panics_outside_of_a_local_set() {
    use rx_rust::scheduler::runtime::tokio::TokioLocalScheduler;

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("Failed building the Runtime");
    let _guard = runtime.enter();
    let _disposal = TokioLocalScheduler::ambient().schedule(|| {}, None);
}

#[test]
fn test_tokio_local_handle_schedules_before_the_local_set_runs() {
    use rx_rust::scheduler::runtime::tokio::TokioLocalScheduler;
    use std::{cell::Cell, rc::Rc};

    let local_set = Rc::new(tokio::task::LocalSet::new());
    let scheduler = TokioLocalScheduler::from_local_set(&local_set);
    let ran = Rc::new(Cell::new(false));
    let ran_in_task = ran.clone();
    let (tx, rx) = futures::channel::oneshot::channel();

    // Neither inside the `LocalSet` nor inside a runtime: the task waits in the `LocalSet`.
    let disposal = scheduler.schedule(
        move || {
            ran_in_task.set(true);
            tx.send(()).unwrap();
        },
        Some(DURATION_10_MS),
    );
    assert!(!ran.get());

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("Failed building the Runtime");
    assert!(runtime.block_on(local_set.run_until(rx)).is_ok());
    assert!(ran.get());
    disposal.dispose();
}

#[test]
fn test_tokio_local_handle_does_not_keep_the_local_set_alive() {
    use rx_rust::scheduler::runtime::tokio::TokioLocalScheduler;
    use std::rc::Rc;

    let local_set = Rc::new(tokio::task::LocalSet::new());
    let scheduler = TokioLocalScheduler::from_local_set(&local_set);
    let probe = Rc::new(());
    let probe_in_task = probe.clone();
    // The pending task holds the scheduler, and so would hold the `LocalSet` if the scheduler did.
    let scheduler_in_task = scheduler.clone();
    let _disposal = scheduler.schedule(
        move || {
            let _ = (&probe_in_task, &scheduler_in_task);
        },
        Some(DURATION_100_MS),
    );
    assert_eq!(Rc::strong_count(&probe), 2);

    // Dropping the `LocalSet` drops the pending task with it.
    drop(local_set);
    assert_eq!(Rc::strong_count(&probe), 1);
}

#[test]
#[should_panic(expected = "has been dropped")]
fn test_tokio_local_handle_panics_once_the_local_set_is_dropped() {
    use rx_rust::scheduler::runtime::tokio::TokioLocalScheduler;
    use std::rc::Rc;

    let scheduler = TokioLocalScheduler::from_local_set(&Rc::new(tokio::task::LocalSet::new()));
    let _disposal = scheduler.schedule(|| {}, None);
}

#[test]
fn test_smol_handle_runs_on_its_executor() {
    use rx_rust::scheduler::runtime::smol::SmolScheduler;
    use std::sync::Arc;

    let executor = Arc::new(smol::Executor::new());
    let scheduler = SmolScheduler::from_executor(&executor);
    let (tx, rx) = futures::channel::oneshot::channel();
    let disposal = scheduler.schedule(move || tx.send(()).unwrap(), Some(DURATION_10_MS));
    // Nothing else drives this executor: the task runs only because it is run here.
    assert!(smol::block_on(executor.run(rx)).is_ok());
    disposal.dispose();
}

#[test]
fn test_smol_handle_does_not_keep_the_executor_alive() {
    use rx_rust::scheduler::runtime::smol::SmolScheduler;
    use std::sync::Arc;

    let executor = Arc::new(smol::Executor::new());
    let scheduler = SmolScheduler::from_executor(&executor);
    let probe = Arc::new(());
    let probe_in_task = probe.clone();
    let scheduler_in_task = scheduler.clone();
    let _disposal = scheduler.schedule(
        move || {
            let _ = (&probe_in_task, &scheduler_in_task);
        },
        Some(DURATION_100_MS),
    );
    assert_eq!(Arc::strong_count(&probe), 2);

    drop(executor);
    assert_eq!(Arc::strong_count(&probe), 1);
}

#[test]
#[should_panic(expected = "has been dropped")]
fn test_smol_handle_panics_once_the_executor_is_dropped() {
    use rx_rust::scheduler::runtime::smol::SmolScheduler;
    use std::sync::Arc;

    let scheduler = SmolScheduler::from_executor(&Arc::new(smol::Executor::new()));
    let _disposal = scheduler.schedule(|| {}, None);
}

#[test]
fn test_smol_local_runs_on_its_executor() {
    use rx_rust::scheduler::runtime::smol::SmolLocalScheduler;
    use std::{cell::Cell, rc::Rc};

    let executor = Rc::new(smol::LocalExecutor::new());
    let scheduler = SmolLocalScheduler::from_executor(&executor);
    let ran = Rc::new(Cell::new(false));
    let ran_in_task = ran.clone();
    let (tx, rx) = futures::channel::oneshot::channel();
    let disposal = scheduler.schedule(
        move || {
            ran_in_task.set(true);
            tx.send(()).unwrap();
        },
        Some(DURATION_10_MS),
    );
    assert!(smol::block_on(executor.run(rx)).is_ok());
    assert!(ran.get());
    disposal.dispose();
}

#[test]
fn test_smol_local_does_not_keep_the_executor_alive() {
    use rx_rust::scheduler::runtime::smol::SmolLocalScheduler;
    use std::rc::Rc;

    let executor = Rc::new(smol::LocalExecutor::new());
    let scheduler = SmolLocalScheduler::from_executor(&executor);
    let probe = Rc::new(());
    let probe_in_task = probe.clone();
    let scheduler_in_task = scheduler.clone();
    let _disposal = scheduler.schedule(
        move || {
            let _ = (&probe_in_task, &scheduler_in_task);
        },
        Some(DURATION_100_MS),
    );
    assert_eq!(Rc::strong_count(&probe), 2);

    drop(executor);
    assert_eq!(Rc::strong_count(&probe), 1);
}

#[test]
#[should_panic(expected = "has been dropped")]
fn test_smol_local_panics_once_the_executor_is_dropped() {
    use rx_rust::scheduler::runtime::smol::SmolLocalScheduler;
    use std::rc::Rc;

    let scheduler = SmolLocalScheduler::from_executor(&Rc::new(smol::LocalExecutor::new()));
    let _disposal = scheduler.schedule(|| {}, None);
}

#[test]
fn test_tokio_current_takes_the_runtime_it_is_built_in() {
    use rx_rust::scheduler::runtime::tokio::TokioScheduler;

    assert!(TokioScheduler::try_current().is_none());
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("Failed building the Runtime");
    let scheduler = {
        let _guard = runtime.enter();
        TokioScheduler::current()
    };
    let (tx, rx) = futures::channel::oneshot::channel();
    // Built inside the runtime, used outside of it.
    let disposal = scheduler.schedule(move || tx.send(()).unwrap(), Some(DURATION_10_MS));
    assert!(runtime.block_on(rx).is_ok());
    disposal.dispose();
}

#[test]
#[should_panic(expected = "must be called from the context of a Tokio 1.x runtime")]
fn test_tokio_current_panics_outside_of_a_runtime() {
    let _ = rx_rust::scheduler::runtime::tokio::TokioScheduler::current();
}

/// A scheduler written the way a UI or game loop would write one: no async executor, only a loop
/// on its own thread that drives each task step by step through `Task::split` and
/// `Stepper::step`.
mod main_loop {
    use rx_rust::{
        disposable::Disposable,
        observable::Subscription,
        scheduler::{Scheduler, SchedulerTypes, Task, TaskState},
        thread_mode::Shared,
    };
    use std::{
        collections::{HashMap, VecDeque},
        pin::Pin,
        sync::{
            Arc,
            atomic::{AtomicU64, Ordering},
            mpsc,
        },
        task::{Context, Poll, Wake, Waker},
        thread::{self, JoinHandle, ThreadId},
        time::{Duration, Instant},
    };

    /// A task with its pinned state boxed and its type erased: calling it steps it once.
    type Job = Box<dyn FnMut(&mut Context<'_>) -> Poll<TaskState> + Send>;

    enum Message {
        /// Queue a task, to be stepped first once `Instant` is reached.
        Run(u64, Job, Instant),
        /// The waker of a pending task was woken.
        Wake(u64),
        /// The task was disposed: drop it, wherever it is waiting.
        Cancel(u64),
        Quit,
    }

    /// Where a job waits for its next step.
    enum Wait {
        /// In the ready queue.
        Ready,
        /// Until this instant.
        Until(Instant),
        /// For its waker.
        Waker,
    }

    /// The handle of a [`MainLoop`]: `Clone + Send`, so any thread can queue tasks on the loop.
    #[derive(Clone)]
    pub(super) struct MainLoopScheduler {
        sender: mpsc::Sender<Message>,
        next_id: Arc<AtomicU64>,
    }

    /// The loop thread a [`MainLoopScheduler`] queues its tasks on.
    pub(super) struct MainLoop {
        sender: mpsc::Sender<Message>,
        thread: JoinHandle<()>,
    }

    impl MainLoop {
        /// Starts the loop on a thread of its own.
        pub(super) fn start() -> (Self, MainLoopScheduler) {
            let (sender, receiver) = mpsc::channel();
            let loop_sender = sender.clone();
            let thread = thread::spawn(move || run(&receiver, &loop_sender));
            let scheduler = MainLoopScheduler {
                sender: sender.clone(),
                next_id: Arc::default(),
            };
            (Self { sender, thread }, scheduler)
        }

        /// The id of the loop thread, where every task runs.
        pub(super) fn thread_id(&self) -> ThreadId {
            self.thread.thread().id()
        }

        /// Stops the loop, dropping the tasks it still holds, and waits for its thread to end.
        pub(super) fn quit(self) {
            self.sender.send(Message::Quit).unwrap();
            self.thread.join().unwrap();
        }
    }

    fn run(receiver: &mpsc::Receiver<Message>, sender: &mpsc::Sender<Message>) {
        let mut jobs: HashMap<u64, (Job, Wait)> = HashMap::new();
        let mut ready: VecDeque<u64> = VecDeque::new();
        loop {
            // Move the sleepers whose instant has come to the ready queue.
            let now = Instant::now();
            for (id, (_, wait)) in &mut jobs {
                if matches!(wait, Wait::Until(at) if *at <= now) {
                    *wait = Wait::Ready;
                    ready.push_back(*id);
                }
            }

            // Step one ready task, if any; otherwise block until a message or the next sleeper is due.
            let message = if let Some(id) = ready.pop_front() {
                if let Some((job, wait)) = jobs.get_mut(&id) {
                    let waker = Waker::from(Arc::new(JobWaker {
                        id,
                        sender: sender.clone(),
                    }));
                    match job(&mut Context::from_waker(&waker)) {
                        Poll::Pending => *wait = Wait::Waker,
                        Poll::Ready(TaskState::Finished) => drop(jobs.remove(&id)),
                        Poll::Ready(TaskState::Yield) => {
                            *wait = Wait::Ready;
                            ready.push_back(id);
                        }
                        Poll::Ready(TaskState::SleepUntil(at)) => *wait = Wait::Until(at),
                    }
                }
                match receiver.try_recv() {
                    Ok(message) => message,
                    Err(_) => continue,
                }
            } else {
                let next_due = jobs
                    .values()
                    .filter_map(|(_, wait)| match wait {
                        Wait::Until(at) => Some(*at),
                        Wait::Ready | Wait::Waker => None,
                    })
                    .min();
                match next_due {
                    Some(at) => {
                        match receiver.recv_timeout(at.saturating_duration_since(Instant::now())) {
                            Ok(message) => message,
                            Err(_) => continue,
                        }
                    }
                    None => receiver.recv().unwrap(),
                }
            };

            match message {
                Message::Run(id, job, at) => {
                    jobs.insert(id, (job, Wait::Until(at)));
                }
                Message::Wake(id) => {
                    // A wake may arrive for a task that is already ready, sleeping or gone: only a task
                    // waiting for its waker moves.
                    if let Some((_, wait @ Wait::Waker)) = jobs.get_mut(&id) {
                        *wait = Wait::Ready;
                        ready.push_back(id);
                    }
                }
                Message::Cancel(id) => drop(jobs.remove(&id)),
                Message::Quit => return,
            }
        }
    }

    struct JobWaker {
        id: u64,
        sender: mpsc::Sender<Message>,
    }

    impl Wake for JobWaker {
        fn wake(self: Arc<Self>) {
            // The loop may be gone already, which leaves nothing to wake.
            let _ = self.sender.send(Message::Wake(self.id));
        }
    }

    /// Disposes a task queued on a [`MainLoop`]: the loop drops it at once, even while it sleeps or
    /// waits for its waker.
    pub(super) struct MainLoopDisposal {
        id: u64,
        sender: mpsc::Sender<Message>,
    }

    impl Disposable for MainLoopDisposal {
        fn dispose(self) {
            let _ = self.sender.send(Message::Cancel(self.id));
        }
    }

    impl SchedulerTypes for MainLoopScheduler {
        type Mode = Shared;
        type Disposal = MainLoopDisposal;
    }

    impl<TC, P> Scheduler<TC, P> for MainLoopScheduler
    where
        TC: Send + 'static,
        P: Send + 'static,
    {
        fn run_task(
            &self,
            task: Task<TC, P>,
            delay: Option<Duration>,
        ) -> Subscription<Self::Disposal> {
            let id = self.next_id.fetch_add(1, Ordering::Relaxed);
            let (mut stepper, pinned) = task.split();
            let mut pinned: Pin<Box<P>> = Box::pin(pinned);
            let job: Job = Box::new(move |cx| stepper.step(pinned.as_mut(), cx, Instant::now()));
            let at = Instant::now() + delay.unwrap_or_default();
            // A loop that has quit drops the task with the message.
            let _ = self.sender.send(Message::Run(id, job, at));
            Subscription::new(MainLoopDisposal {
                id,
                sender: self.sender.clone(),
            })
        }
    }
}

/// How long a `MainLoop` test waits for an event: far beyond what it takes, it only turns a hang
/// into a failure.
const MAIN_LOOP_TIMEOUT: Duration = Duration::from_secs(5);

#[test]
fn test_main_loop_delivers_on_its_thread() {
    let (main_loop, scheduler) = MainLoop::start();
    let main_thread = main_loop.thread_id();
    let values = Arc::new(Mutex::new(Vec::new()));
    let values_observer = values.clone();
    let (termination_tx, termination_rx) = mpsc::channel();
    let mut subject = PublishSubject::<i32, Infallible, _>::shared();
    let subscription = subject
        .clone()
        .observe_on(scheduler)
        .subscribe_with_callback(
            move |value| {
                values_observer.with_mut(|values| values.push((value, thread::current().id())))
            },
            move |termination| {
                termination_tx
                    .send((termination, thread::current().id()))
                    .unwrap()
            },
        );

    // Emitted from a thread of its own, received on the loop's.
    thread::spawn(move || {
        for value in 1..=3 {
            assert!(subject.on_next(value).is_continue());
        }
        subject.on_termination(Termination::Completed);
    })
    .join()
    .unwrap();

    let termination = termination_rx.recv_timeout(MAIN_LOOP_TIMEOUT).unwrap();
    assert_eq!(termination, (Termination::Completed, main_thread));
    assert_eq!(
        values.clone_value(),
        [(1, main_thread), (2, main_thread), (3, main_thread)]
    );
    drop(subscription);
    main_loop.quit();
}

// `delay` hands the loop a task with a delay, and then sleeps between its steps.
#[test]
fn test_main_loop_runs_a_delayed_task() {
    let (main_loop, scheduler) = MainLoop::start();
    let main_thread = main_loop.thread_id();
    let (event_tx, event_rx) = mpsc::channel();
    let termination_tx = event_tx.clone();
    let start_time = Instant::now();
    let subscription = Just::new(1)
        .delay(DURATION_30_MS, scheduler)
        .subscribe_with_callback(
            move |value| {
                event_tx
                    .send((Some(value), thread::current().id()))
                    .unwrap()
            },
            move |_| termination_tx.send((None, thread::current().id())).unwrap(),
        );

    assert_eq!(
        event_rx.recv_timeout(MAIN_LOOP_TIMEOUT).unwrap(),
        (Some(1), main_thread)
    );
    assert!(start_time.elapsed() >= DURATION_30_MS);
    assert_eq!(
        event_rx.recv_timeout(MAIN_LOOP_TIMEOUT).unwrap(),
        (None, main_thread)
    );
    drop(subscription);
    main_loop.quit();
}

// A pending task is stepped again once another thread wakes it.
#[test]
fn test_main_loop_steps_a_task_woken_from_another_thread() {
    let (main_loop, scheduler) = MainLoop::start();
    let main_thread = main_loop.thread_id();
    let (wake_tx, wake_rx) = futures::channel::oneshot::channel::<()>();
    let (done_tx, done_rx) = mpsc::channel();
    let subscription = scheduler.spawn_future(async move {
        wake_rx.await.unwrap();
        done_tx.send(thread::current().id()).unwrap();
    });

    thread::spawn(move || {
        thread::sleep(DURATION_10_MS);
        wake_tx.send(()).unwrap();
    })
    .join()
    .unwrap();

    assert_eq!(
        done_rx.recv_timeout(MAIN_LOOP_TIMEOUT).unwrap(),
        main_thread
    );
    drop(subscription);
    main_loop.quit();
}

// A task disposed before its delay ends never runs, and the loop drops it at once rather than
// when it would have been due.
#[test]
fn test_main_loop_disposes_a_queued_task() {
    let (main_loop, scheduler) = MainLoop::start();
    let (dropped_tx, dropped_rx) = mpsc::channel();
    let probe = DropProbe::new().on_drop(Box::new(move || dropped_tx.send(()).unwrap()));
    let ran = Arc::new(AtomicBool::new(false));
    let ran_task = ran.clone();
    let disposal = scheduler.schedule(
        move || {
            drop(probe);
            ran_task.write(true);
        },
        Some(Duration::from_secs(1)),
    );

    disposal.dispose();
    assert!(dropped_rx.recv_timeout(DURATION_100_MS).is_ok());
    assert!(!ran.read());
    main_loop.quit();
}
