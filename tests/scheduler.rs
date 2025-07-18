mod tests_utils;

use crate::tests_utils::test_runtime::block_on;
use crate::tests_utils::{
    RECURSION_EXECUTION_TIMES, RECURSION_EXPECTED_DIFF, RECURSION_SLEEP_TIME,
};
use futures::StreamExt;
use rx_rust::scheduler::RecursionAction;
use rx_rust::{disposable::Disposable, scheduler::Scheduler};
use std::time::{Duration, Instant};

#[test]
fn test_schedule_without_delay() {
    block_on(|runtime| async move {
        let (tx, rx) = futures::channel::oneshot::channel();
        let task = || {
            tx.send(()).unwrap();
        };
        let start_time = Instant::now();
        let _disposal = runtime.clone().schedule(task, None);
        assert!(rx.await.is_ok());
        let elapsed_time = start_time.elapsed();
        assert!(elapsed_time < Duration::from_millis(10));
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
        let _disposal = runtime
            .clone()
            .schedule(task, Some(Duration::from_millis(100)));
        assert!(rx.await.is_ok());
        let elapsed_time = start_time.elapsed();
        assert!(elapsed_time >= Duration::from_millis(100));
        assert!(elapsed_time < Duration::from_millis(110));
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
        let disposal = runtime
            .clone()
            .schedule(task, Some(Duration::from_millis(100)));
        disposal.dispose();
        assert!(rx.await.is_err());
        let elapsed_time = start_time.elapsed();
        assert!(elapsed_time < Duration::from_millis(10));
    });
}

#[test]
fn test_schedule_with_late_abort() {
    block_on(|runtime| async move {
        let (tx, rx) = futures::channel::oneshot::channel();
        let task = || {
            tx.send(()).unwrap();
        };
        let disposal = runtime.clone().schedule(task, None);
        runtime.clone().sleep(Duration::from_millis(10)).await;
        disposal.dispose();
        assert!(rx.await.is_ok());
    });
}

#[test]
fn test_schedule_recursively_without_delay() {
    block_on(|runtime| async move {
        let start_instant = Instant::now();
        let (tx, mut rx) = futures::channel::mpsc::unbounded();
        let mut tx = Some(tx);
        let disposal = runtime.clone().schedule_recursively(
            move |index| {
                if index == RECURSION_EXECUTION_TIMES {
                    tx.take().unwrap();
                    RecursionAction::Stop
                } else {
                    tx.as_ref().unwrap().unbounded_send(Instant::now()).unwrap();
                    RecursionAction::ContinueAfterRevisedDelay(Duration::from_millis(
                        RECURSION_SLEEP_TIME,
                    ))
                }
            },
            None,
        );
        let mut count = 0;
        while let Some(call_instant) = rx.next().await {
            let duration = call_instant - start_instant;
            let diff =
                duration.as_micros() - (count * RECURSION_SLEEP_TIME as usize * 1000) as u128;
            assert!(
                diff < RECURSION_EXPECTED_DIFF,
                "diff: {}, count: {}",
                diff,
                count
            );
            count += 1;
        }
        assert_eq!(count, RECURSION_EXECUTION_TIMES);
        disposal.dispose();
    });
}

#[test]
fn test_schedule_recursively_with_delay() {
    block_on(|runtime| async move {
        let start_instant = Instant::now();
        let (tx, mut rx) = futures::channel::mpsc::unbounded();
        let mut tx = Some(tx);
        let disposal = runtime.clone().schedule_recursively(
            move |index| {
                if index == RECURSION_EXECUTION_TIMES {
                    tx.take().unwrap();
                    RecursionAction::Stop
                } else {
                    tx.as_ref().unwrap().unbounded_send(Instant::now()).unwrap();
                    RecursionAction::ContinueAfterRevisedDelay(Duration::from_millis(
                        RECURSION_SLEEP_TIME,
                    ))
                }
            },
            Some(Duration::from_millis(RECURSION_SLEEP_TIME)),
        );
        let mut count = 0;
        while let Some(call_instant) = rx.next().await {
            let duration = call_instant - start_instant;
            let diff =
                duration.as_micros() - ((count + 1) * RECURSION_SLEEP_TIME as usize * 1000) as u128;
            assert!(
                diff < RECURSION_EXPECTED_DIFF,
                "diff: {}, count: {}",
                diff,
                count
            );
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
        let disposal = runtime.clone().schedule_recursively(
            move |index| {
                if index == RECURSION_EXECUTION_TIMES {
                    tx.take().unwrap();
                    RecursionAction::Stop
                } else {
                    tx.as_ref().unwrap().unbounded_send(()).unwrap();
                    RecursionAction::ContinueAfterRevisedDelay(Duration::from_millis(1))
                }
            },
            Some(Duration::from_millis(1)),
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
fn test_schedule_period_without_delay() {
    block_on(|runtime| async move {
        let start_instant = Instant::now();
        let (tx, mut rx) = futures::channel::mpsc::unbounded();
        let mut tx = Some(tx);
        let disposal = runtime.clone().schedule_periodically(
            move |index| {
                if index == RECURSION_EXECUTION_TIMES {
                    tx.take().unwrap();
                    true
                } else {
                    tx.as_ref().unwrap().unbounded_send(Instant::now()).unwrap();
                    false
                }
            },
            Duration::from_millis(RECURSION_SLEEP_TIME),
            None,
        );
        let mut count = 0;
        while let Some(call_instant) = rx.next().await {
            let duration = call_instant - start_instant;
            let diff =
                duration.as_micros() - (count * RECURSION_SLEEP_TIME as usize * 1000) as u128;
            assert!(
                diff < RECURSION_EXPECTED_DIFF,
                "diff: {}, count: {}",
                diff,
                count
            );
            count += 1;
        }
        assert_eq!(count, RECURSION_EXECUTION_TIMES);
        disposal.dispose();
    });
}

#[test]
fn test_schedule_period_with_delay() {
    block_on(|runtime| async move {
        let start_instant = Instant::now();
        let (tx, mut rx) = futures::channel::mpsc::unbounded();
        let mut tx = Some(tx);
        let disposal = runtime.clone().schedule_periodically(
            move |index| {
                if index == RECURSION_EXECUTION_TIMES {
                    tx.take().unwrap();
                    true
                } else {
                    tx.as_ref().unwrap().unbounded_send(Instant::now()).unwrap();
                    false
                }
            },
            Duration::from_millis(RECURSION_SLEEP_TIME),
            Some(Duration::from_millis(RECURSION_SLEEP_TIME)),
        );
        let mut count = 0;
        while let Some(call_instant) = rx.next().await {
            let duration = call_instant - start_instant;
            let diff =
                duration.as_micros() - ((count + 1) * RECURSION_SLEEP_TIME as usize * 1000) as u128;
            assert!(
                diff < RECURSION_EXPECTED_DIFF,
                "diff: {}, count: {}",
                diff,
                count
            );
            count += 1;
        }
        assert_eq!(count, RECURSION_EXECUTION_TIMES);
        disposal.dispose();
    });
}
