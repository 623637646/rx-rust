mod tests_utils;

use crate::tests_utils::DURATION_10_MS;
use crate::tests_utils::DURATION_30_MS;
use crate::tests_utils::DURATION_100_MS;
use crate::tests_utils::test_scheduler::block_on;
use futures::StreamExt;
use rx_rust::disposable::Disposable;
use rx_rust::scheduler::SchedulerExt;
use rx_rust::scheduler::TaskState;
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
            move |index| {
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
            move |index| {
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
            move |index| {
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
            move |index| {
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
            move |index| {
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

#[test]
#[should_panic(expected = "period must be non-zero")]
fn test_schedule_periodically_rejects_zero_period() {
    block_on(|scheduler| async move {
        let _disposal = scheduler.schedule_periodically(|_| false, Duration::ZERO, None);
    });
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
#[should_panic]
fn test_tokio_current_panics_outside_of_a_runtime() {
    let _ = rx_rust::scheduler::runtime::tokio::TokioScheduler::current();
}
