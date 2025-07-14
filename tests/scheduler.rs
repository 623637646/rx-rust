mod tests_utils;

use crate::tests_utils::checker::State;
use crate::tests_utils::{checker::Checker, test_runtime::block_on};
use rx_rust::{
    disposable::Disposable,
    observer::{Observer, Termination},
    scheduler::Scheduler,
};
use std::{
    convert::Infallible,
    time::{Duration, Instant},
};

#[test]
fn test_schedule_without_delay() {
    block_on(|runtime| async move {
        let (tx, rx) = futures::channel::oneshot::channel();
        let task = || {
            tx.send(()).unwrap();
        };
        let start_time = Instant::now();
        let _disposal = runtime.schedule(task, None);
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
        let _disposal = runtime.schedule(task, Some(Duration::from_millis(100)));
        assert!(rx.await.is_ok());
        let elapsed_time = start_time.elapsed();
        assert!(elapsed_time >= Duration::from_millis(100));
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
        let disposal = runtime.schedule(task, Some(Duration::from_millis(100)));
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
        let disposal = runtime.schedule(task, None);
        runtime.sleep(Duration::from_millis(10)).await;
        disposal.dispose();
        assert!(rx.await.is_ok());
    });
}

#[test]
fn test_schedule_recursive() {
    block_on(|runtime| async move {
        let (checker, observer) = Checker::new();
        let mut observer = Some(observer);
        let _disposal = runtime.schedule_recursive(
            move |index| {
                observer.as_mut().unwrap().on_next(index);
                if index == 5 {
                    observer
                        .take()
                        .unwrap()
                        .on_termination(Termination::<Infallible>::Completed);
                    None
                } else {
                    Some(Duration::from_millis(
                        ((index + 1) * 10).try_into().unwrap(),
                    ))
                }
            },
            None,
        );
        runtime.sleep(Duration::from_millis(5)).await;
        assert_eq!(checker.values(), [0]);
        assert_eq!(checker.state(), State::Active);

        runtime.sleep(Duration::from_millis(10)).await;
        assert_eq!(checker.values(), [0, 1]);
        assert_eq!(checker.state(), State::Active);

        runtime.sleep(Duration::from_millis(20)).await;
        assert_eq!(checker.values(), [0, 1, 2]);
        assert_eq!(checker.state(), State::Active);

        runtime.sleep(Duration::from_millis(30)).await;
        assert_eq!(checker.values(), [0, 1, 2, 3]);
        assert_eq!(checker.state(), State::Active);

        runtime.sleep(Duration::from_millis(40)).await;
        assert_eq!(checker.values(), [0, 1, 2, 3, 4]);
        assert_eq!(checker.state(), State::Active);

        runtime.sleep(Duration::from_millis(50)).await;
        assert_eq!(checker.values(), [0, 1, 2, 3, 4, 5]);
        assert_eq!(checker.state(), State::Completed);

        runtime.sleep(Duration::from_millis(100)).await;
        assert_eq!(checker.values(), [0, 1, 2, 3, 4, 5]);
        assert_eq!(checker.state(), State::Completed);
    });
}

#[test]
fn test_schedule_period_without_delay() {
    block_on(|runtime| async move {
        let (checker, observer) = Checker::new();
        let mut observer = Some(observer);
        let _disposal = runtime.schedule_period(
            move |index| {
                observer.as_mut().unwrap().on_next(index);
                if index == 5 {
                    observer
                        .take()
                        .unwrap()
                        .on_termination(Termination::<Infallible>::Completed);
                    true
                } else {
                    false
                }
            },
            Duration::from_millis(100),
            None,
        );
        runtime.sleep(Duration::from_millis(50)).await;
        assert_eq!(checker.values(), [0]);
        assert_eq!(checker.state(), State::Active);

        runtime.sleep(Duration::from_millis(100)).await;
        assert_eq!(checker.values(), [0, 1]);
        assert_eq!(checker.state(), State::Active);

        runtime.sleep(Duration::from_millis(100)).await;
        assert_eq!(checker.values(), [0, 1, 2]);
        assert_eq!(checker.state(), State::Active);

        runtime.sleep(Duration::from_millis(100)).await;
        assert_eq!(checker.values(), [0, 1, 2, 3]);
        assert_eq!(checker.state(), State::Active);

        runtime.sleep(Duration::from_millis(100)).await;
        assert_eq!(checker.values(), [0, 1, 2, 3, 4]);
        assert_eq!(checker.state(), State::Active);

        runtime.sleep(Duration::from_millis(100)).await;
        assert_eq!(checker.values(), [0, 1, 2, 3, 4, 5]);
        assert_eq!(checker.state(), State::Completed);

        runtime.sleep(Duration::from_millis(100)).await;
        assert_eq!(checker.values(), [0, 1, 2, 3, 4, 5]);
        assert_eq!(checker.state(), State::Completed);
    });
}

#[test]
fn test_schedule_period_with_delay() {
    block_on(|runtime| async move {
        let (checker, observer) = Checker::new();
        let mut observer = Some(observer);
        let _disposal = runtime.schedule_period(
            move |index| {
                observer.as_mut().unwrap().on_next(index);
                if index == 5 {
                    observer
                        .take()
                        .unwrap()
                        .on_termination(Termination::<Infallible>::Completed);
                    true
                } else {
                    false
                }
            },
            Duration::from_millis(100),
            Some(Duration::from_millis(20)),
        );
        assert_eq!(checker.values(), []);
        assert_eq!(checker.state(), State::Active);

        runtime.sleep(Duration::from_millis(50)).await;
        assert_eq!(checker.values(), [0]);
        assert_eq!(checker.state(), State::Active);

        runtime.sleep(Duration::from_millis(100)).await;
        assert_eq!(checker.values(), [0, 1]);
        assert_eq!(checker.state(), State::Active);

        runtime.sleep(Duration::from_millis(100)).await;
        assert_eq!(checker.values(), [0, 1, 2]);
        assert_eq!(checker.state(), State::Active);

        runtime.sleep(Duration::from_millis(100)).await;
        assert_eq!(checker.values(), [0, 1, 2, 3]);
        assert_eq!(checker.state(), State::Active);

        runtime.sleep(Duration::from_millis(100)).await;
        assert_eq!(checker.values(), [0, 1, 2, 3, 4]);
        assert_eq!(checker.state(), State::Active);

        runtime.sleep(Duration::from_millis(100)).await;
        assert_eq!(checker.values(), [0, 1, 2, 3, 4, 5]);
        assert_eq!(checker.state(), State::Completed);

        runtime.sleep(Duration::from_millis(100)).await;
        assert_eq!(checker.values(), [0, 1, 2, 3, 4, 5]);
        assert_eq!(checker.state(), State::Completed);
    });
}
