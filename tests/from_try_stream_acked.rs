mod tests_utils;

use crate::tests_utils::DURATION_10_MS;
use crate::tests_utils::checker::State;
use crate::tests_utils::test_scheduler::block_on;
use futures::{SinkExt, stream};
use rx_rust::scheduler::virtual_time::VirtualTime;
use rx_rust::thread_mode::Shared;
use rx_rust::thread_mode::mutable::MutableHelper;
use rx_rust::{
    disposable::Disposable,
    observable::{Observable, ObservableExt},
    operators::creating::{
        from_stream_acked::StreamAck, from_try_stream_acked::FromTryStreamAcked,
    },
};
use std::num::NonZeroUsize;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tests_utils::checker::Checker;

/// The `StreamAck`s a test holds, to acknowledge the items when it chooses.
type Acks = Arc<Mutex<Vec<StreamAck<Shared>>>>;

/// Keeps each `StreamAck` in `acks`, and sends the value on.
fn keep_ack<T>(acks: &Acks) -> impl FnMut((T, StreamAck<Shared>)) -> T + Send + use<T> {
    let acks = acks.clone();
    move |(value, ack)| {
        acks.with_mut(|acks| acks.push(ack));
        value
    }
}

/// Acknowledges the oldest item held.
fn ack_first(acks: &Acks) {
    drop(acks.with_mut(|acks| acks.remove(0)));
}

#[test]
fn test_completed() {
    let time = VirtualTime::new();
    let (tx, rx) = futures::channel::mpsc::unbounded::<Result<i32, String>>();
    let acks = Acks::default();
    let observable = FromTryStreamAcked::new(rx, time.scheduler()).map(keep_ack(&acks));
    let (checker, observer) = Checker::new();

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);

    tx.unbounded_send(Ok(111)).unwrap();
    time.advance_by(Duration::ZERO);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);

    drop(tx);
    time.advance_by(Duration::ZERO);
    assert_eq!(checker.state(), State::Active);

    ack_first(&acks);
    time.advance_by(Duration::ZERO);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_completed_without_next() {
    let time = VirtualTime::new();
    let stream = stream::empty::<Result<i32, String>>();
    let observable = FromTryStreamAcked::new(stream, time.scheduler()).map(|(value, _)| value);
    let (checker, observer) = Checker::new();

    let _subscription = observable.subscribe(observer);
    time.advance_by(Duration::ZERO);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_error() {
    let time = VirtualTime::new();
    let stream = stream::iter([Ok(111), Err("boom".to_owned()), Ok(333)]);
    let acks = Acks::default();
    let observable = FromTryStreamAcked::new(stream, time.scheduler()).map(keep_ack(&acks));
    let (checker, observer) = Checker::new();

    let _subscription = observable.subscribe(observer);
    time.advance_by(Duration::ZERO);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);

    // The error is polled, like any item, once a permit is back.
    ack_first(&acks);
    time.advance_by(Duration::ZERO);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Error("boom".to_owned()));
    assert_eq!(time.pending_tasks(), 0);
}

#[test]
fn test_error_without_next() {
    let time = VirtualTime::new();
    let stream = stream::iter([Err::<i32, _>("boom".to_owned())]);
    let observable = FromTryStreamAcked::new(stream, time.scheduler()).map(|(value, _)| value);
    let (checker, observer) = Checker::new();

    let _subscription = observable.subscribe(observer);
    time.advance_by(Duration::ZERO);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Error("boom".to_owned()));
}

#[test]
fn test_with_permits() {
    let time = VirtualTime::new();
    let stream = stream::iter([Ok::<_, String>(111), Ok(222), Ok(333)]);
    let acks = Acks::default();
    let observable =
        FromTryStreamAcked::with_permits(stream, time.scheduler(), NonZeroUsize::new(2).unwrap())
            .map(keep_ack(&acks));
    let (checker, observer) = Checker::new();

    let _subscription = observable.subscribe(observer);
    time.advance_by(Duration::ZERO);
    assert_eq!(checker.values(), [111, 222]);

    ack_first(&acks);
    time.advance_by(Duration::ZERO);
    assert_eq!(checker.values(), [111, 222, 333]);
}

#[test]
fn test_unsubscribe() {
    let time = VirtualTime::new();
    let stream = stream::iter([Ok::<_, String>(111), Ok(222)]);
    let acks = Acks::default();
    let observable = FromTryStreamAcked::new(stream, time.scheduler()).map(keep_ack(&acks));
    let (checker, observer) = Checker::new();

    let subscription = observable.subscribe(observer);
    time.advance_by(Duration::ZERO);
    assert_eq!(checker.values(), [111]);

    subscription.dispose();
    ack_first(&acks);
    time.advance_by(Duration::ZERO);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Dropped);
    assert_eq!(time.pending_tasks(), 0);
}

#[test]
fn test_async() {
    block_on(|scheduler| async move {
        let (mut tx, rx) = futures::channel::mpsc::unbounded::<Result<i32, String>>();
        let acks = Acks::default();
        let observable = FromTryStreamAcked::new(rx, scheduler.clone()).map(keep_ack(&acks));
        let (checker, observer) = Checker::new();

        let _subscription = scheduler
            .spawn(async move { observable.subscribe(observer) })
            .await;

        tx.send(Ok(111)).await.unwrap();
        tx.send(Err("boom".to_owned())).await.unwrap();
        scheduler.sleep(DURATION_10_MS).await;
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Active);

        ack_first(&acks);
        scheduler.sleep(DURATION_10_MS).await;
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Error("boom".to_owned()));
    });
}

#[test]
fn test_clone() {
    let time = VirtualTime::new();
    let stream = stream::iter(vec![Ok::<_, String>(111)]);
    let observable = FromTryStreamAcked::new(stream, time.scheduler());
    _ = observable.clone();
}

#[test]
fn test_type_inference_with_subscribe() {
    let time = VirtualTime::new();
    let stream = stream::iter(vec![Ok::<_, String>(111)]);
    let observable = FromTryStreamAcked::new(stream, time.scheduler());

    let observable = observable.map(|(value, _)| value);
    let (_, observer) = Checker::new();
    let _ = observable.subscribe(observer);
}

#[test]
fn test_type_inference_without_subscribe() {
    let time = VirtualTime::new();
    let stream = stream::iter(vec![Ok::<_, String>(111)]);
    let observable = FromTryStreamAcked::new(stream, time.scheduler());

    observable.filter(|_| true);
}
