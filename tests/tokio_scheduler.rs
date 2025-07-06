use rx_rust::{
    scheduler::{Scheduler, tokio_scheduler::TokioScheduler},
    subscription::disposable::Disposable,
};
use std::sync::{Arc, Mutex};
use tokio::time::Duration;

#[tokio::test]
async fn test_schedule_without_delay() {
    let scheduler = TokioScheduler;
    let (tx, rx) = futures::channel::oneshot::channel();
    let task = || {
        tx.send(()).unwrap();
    };
    let start_time = tokio::time::Instant::now();
    let _disposal = scheduler.schedule(task, None);
    assert!(rx.await.is_ok());
    let elapsed_time = start_time.elapsed();
    assert!(elapsed_time < Duration::from_millis(10));
}

#[tokio::test]
async fn test_schedule_with_delay() {
    let scheduler = TokioScheduler;
    let (tx, rx) = futures::channel::oneshot::channel();
    let task = || {
        tx.send(()).unwrap();
    };
    let start_time = tokio::time::Instant::now();
    let _disposal = scheduler.schedule(task, Some(Duration::from_millis(100)));
    assert!(rx.await.is_ok());
    let elapsed_time = start_time.elapsed();
    assert!(elapsed_time >= Duration::from_millis(100));
}

#[tokio::test]
async fn test_schedule_with_abort() {
    let scheduler = TokioScheduler;
    let (tx, rx) = futures::channel::oneshot::channel();
    let task = || {
        tx.send(()).unwrap();
    };
    let start_time = tokio::time::Instant::now();
    let disposal = scheduler.schedule(task, None);
    disposal.dispose();
    assert!(rx.await.is_err());
    let elapsed_time = start_time.elapsed();
    assert!(elapsed_time < Duration::from_millis(10));
}

#[tokio::test]
async fn test_schedule_with_late_abort() {
    let scheduler = TokioScheduler;
    let (tx, rx) = futures::channel::oneshot::channel();
    let task = || {
        tx.send(()).unwrap();
    };
    let disposal = scheduler.schedule(task, None);
    tokio::time::sleep(Duration::from_millis(10)).await;
    disposal.dispose();
    assert!(rx.await.is_ok());
}

#[tokio::test]
async fn test_schedule_period_with_delay() {
    let scheduler = TokioScheduler;
    let counter = Arc::new(Mutex::new(Vec::new()));
    let counter_cloned = counter.clone();
    let task = move |count| {
        counter_cloned.lock().unwrap().push(count);
        false
    };
    let disposal = scheduler.schedule_period(
        task,
        Duration::from_millis(100),
        Some(Duration::from_millis(100)),
    );
    assert_eq!(counter.lock().unwrap().as_ref(), vec![]);

    tokio::time::sleep(Duration::from_millis(50)).await;
    assert_eq!(counter.lock().unwrap().as_ref(), vec![]);

    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_eq!(counter.lock().unwrap().as_ref(), vec![0]);

    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_eq!(counter.lock().unwrap().as_ref(), vec![0, 1]);

    disposal.dispose();
    assert_eq!(counter.lock().unwrap().as_ref(), vec![0, 1]);

    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_eq!(counter.lock().unwrap().as_ref(), vec![0, 1]);
}

#[tokio::test]
async fn test_schedule_period_without_delay() {
    let scheduler = TokioScheduler;
    let counter = Arc::new(Mutex::new(Vec::new()));
    let counter_cloned = counter.clone();
    let task = move |count| {
        counter_cloned.lock().unwrap().push(count);
        false
    };
    let disposal = scheduler.schedule_period(task, Duration::from_millis(100), None);
    assert_eq!(counter.lock().unwrap().as_ref(), vec![]);

    tokio::time::sleep(Duration::from_millis(50)).await;
    assert_eq!(counter.lock().unwrap().as_ref(), vec![0]);

    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_eq!(counter.lock().unwrap().as_ref(), vec![0, 1]);

    disposal.dispose();
    assert_eq!(counter.lock().unwrap().as_ref(), vec![0, 1]);

    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_eq!(counter.lock().unwrap().as_ref(), vec![0, 1]);
}

#[tokio::test]
async fn test_schedule_period_stop() {
    let scheduler = TokioScheduler;
    let counter = Arc::new(Mutex::new(Vec::new()));
    let counter_cloned = counter.clone();
    let task = move |count| {
        counter_cloned.lock().unwrap().push(count);
        count == 2
    };
    let _disposal = scheduler.schedule_period(task, Duration::from_millis(100), None);
    assert_eq!(counter.lock().unwrap().as_ref(), vec![]);

    tokio::time::sleep(Duration::from_millis(50)).await;
    assert_eq!(counter.lock().unwrap().as_ref(), vec![0]);

    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_eq!(counter.lock().unwrap().as_ref(), vec![0, 1]);

    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_eq!(counter.lock().unwrap().as_ref(), vec![0, 1, 2]);

    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_eq!(counter.lock().unwrap().as_ref(), vec![0, 1, 2]);

    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_eq!(counter.lock().unwrap().as_ref(), vec![0, 1, 2]);

    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_eq!(counter.lock().unwrap().as_ref(), vec![0, 1, 2]);
}

#[test]
fn test_clone() {
    let scheduler = TokioScheduler;
    _ = scheduler.clone();
}
