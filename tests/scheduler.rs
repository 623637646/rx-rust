mod tests_utils;

use crate::tests_utils::DURATION_10_MS;
use crate::tests_utils::DURATION_30_MS;
use crate::tests_utils::DURATION_100_MS;
use crate::tests_utils::test_runtime::block_on;
use futures::StreamExt;
use rx_rust::scheduler::RecursionAction;
use rx_rust::{disposable::Disposable, scheduler::Scheduler};
use std::time::Instant;

const RECURSION_EXECUTION_TIMES: usize = 200;

#[test]
fn test_schedule_without_delay() {
    block_on(|runtime| async move {
        let (tx, rx) = futures::channel::oneshot::channel();
        let task = || {
            tx.send(()).unwrap();
        };
        let start_time = Instant::now();
        let disposal = runtime.schedule(task, None);
        assert!(rx.await.is_ok());
        let elapsed_time = start_time.elapsed();
        assert!(elapsed_time < DURATION_30_MS);
        disposal.dispose();
    });
}

#[test]
fn test_schedule_with_delay() {
    block_on(|runtime| async move {
        let (tx, rx) = futures::channel::oneshot::channel();
        let task = || {
            tx.send(()).unwrap();
        };
        let start_time = Instant::now();
        let disposal = runtime.schedule(task, Some(DURATION_100_MS));
        assert!(rx.await.is_ok());
        let elapsed_time = start_time.elapsed();
        assert!(elapsed_time >= DURATION_100_MS);
        assert!(elapsed_time < DURATION_100_MS + DURATION_30_MS);
        disposal.dispose();
    });
}

#[test]
fn test_schedule_with_abort() {
    block_on(|runtime| async move {
        let (tx, rx) = futures::channel::oneshot::channel();
        let task = || {
            tx.send(()).unwrap();
        };
        let start_time = Instant::now();
        let disposal = runtime.schedule(task, Some(DURATION_100_MS));
        disposal.dispose();
        assert!(rx.await.is_err());
        let elapsed_time = start_time.elapsed();
        assert!(elapsed_time < DURATION_30_MS);
    });
}

#[test]
fn test_schedule_with_late_abort() {
    block_on(|runtime| async move {
        let (tx, rx) = futures::channel::oneshot::channel();
        let task = || {
            tx.send(()).unwrap();
        };
        let disposal = runtime.schedule(task, None);
        runtime.sleep(DURATION_10_MS).await;
        disposal.dispose();
        assert!(rx.await.is_ok());
    });
}

#[test]
fn test_schedule_recursively_without_delay() {
    block_on(|runtime| async move {
        let (tx, mut rx) = futures::channel::mpsc::unbounded();
        let mut tx = Some(tx);
        let first = Instant::now();
        let start_instant = Instant::now();
        let disposal = runtime.schedule_recursively(
            move |index| {
                if index == RECURSION_EXECUTION_TIMES {
                    tx.take().unwrap();
                    RecursionAction::Stop
                } else {
                    tx.as_ref().unwrap().unbounded_send(Instant::now()).unwrap();
                    RecursionAction::ContinueAt(first + DURATION_10_MS * (index as u32 + 1))
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
    block_on(|runtime| async move {
        let (tx, mut rx) = futures::channel::mpsc::unbounded();
        let mut tx = Some(tx);
        let first = Instant::now() + DURATION_10_MS;
        let start_instant = Instant::now();
        let disposal = runtime.schedule_recursively(
            move |index| {
                if index == RECURSION_EXECUTION_TIMES {
                    tx.take().unwrap();
                    RecursionAction::Stop
                } else {
                    tx.as_ref().unwrap().unbounded_send(Instant::now()).unwrap();
                    RecursionAction::ContinueAt(first + DURATION_10_MS * (index as u32 + 1))
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
    block_on(|runtime| async move {
        let (tx, mut rx) = futures::channel::mpsc::unbounded();
        let mut tx = Some(tx);
        let small = DURATION_10_MS;
        let first = Instant::now() + small;
        let disposal = runtime.schedule_recursively(
            move |index| {
                if index == RECURSION_EXECUTION_TIMES {
                    tx.take().unwrap();
                    RecursionAction::Stop
                } else {
                    tx.as_ref().unwrap().unbounded_send(()).unwrap();
                    RecursionAction::ContinueAt(first + small * (index as u32 + 1))
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

#[test]
fn test_schedule_periodically_without_delay() {
    block_on(|runtime| async move {
        let (tx, mut rx) = futures::channel::mpsc::unbounded();
        let mut tx = Some(tx);
        let start_instant = Instant::now();
        let disposal = runtime.schedule_periodically(
            move |index| {
                if index == RECURSION_EXECUTION_TIMES {
                    tx.take().unwrap();
                    true
                } else {
                    tx.as_ref().unwrap().unbounded_send(Instant::now()).unwrap();
                    false
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
    block_on(|runtime| async move {
        let (tx, mut rx) = futures::channel::mpsc::unbounded();
        let mut tx = Some(tx);
        let start_instant = Instant::now();
        let disposal = runtime.schedule_periodically(
            move |index| {
                if index == RECURSION_EXECUTION_TIMES {
                    tx.take().unwrap();
                    true
                } else {
                    tx.as_ref().unwrap().unbounded_send(Instant::now()).unwrap();
                    false
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
