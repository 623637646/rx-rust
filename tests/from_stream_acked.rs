mod tests_utils;

use crate::tests_utils::DURATION_10_MS;
use crate::tests_utils::checker::State;
use crate::tests_utils::test_channel::ChannelState;
use crate::tests_utils::test_scheduler::block_on;
use futures::executor::ThreadPool;
use futures::{SinkExt, StreamExt, stream};
use rx_rust::operators::creating::empty::Empty;
use rx_rust::operators::creating::from_try_stream_acked::FromTryStreamAcked;
use rx_rust::operators::creating::timer::Timer;
use rx_rust::scheduler::runtime::futures::ThreadPoolScheduler;
use rx_rust::scheduler::virtual_time::VirtualTime;
use rx_rust::thread_mode::Shared;
use rx_rust::thread_mode::mutable::{MutableExt, MutableHelper};
use rx_rust::{
    disposable::Disposable,
    observable::{Observable, ObservableExt},
    observer::{Observer, Termination},
    operators::creating::from_stream_acked::{FromStreamAcked, StreamAck},
};
use std::convert::Infallible;
use std::num::NonZeroUsize;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::time::Duration;
use tests_utils::{checker::Checker, test_channel::test_channel};

const RACE_TIMEOUT: Duration = Duration::from_secs(5);

/// The `StreamAck`s a test holds, to acknowledge the items when it chooses.
type Acks = Arc<Mutex<Vec<StreamAck<Shared>>>>;

/// Keeps each `StreamAck` in `acks`, and sends the value on.
fn keep_ack<T>(acks: &Acks) -> impl FnMut((T, StreamAck<Shared>)) -> T + Clone + Send + use<T> {
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
    let (tx, rx) = futures::channel::mpsc::unbounded();
    let acks = Acks::default();
    let observable = FromStreamAcked::new(rx, time.scheduler()).map(keep_ack(&acks));
    let (checker, observer) = Checker::new();

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);

    tx.unbounded_send(111).unwrap();
    time.advance_by(Duration::ZERO);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);

    // The end of the stream is seen only when it is polled, once a permit is back.
    drop(tx);
    time.advance_by(Duration::ZERO);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);

    ack_first(&acks);
    time.advance_by(Duration::ZERO);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_completed_without_next() {
    let time = VirtualTime::new();
    let stream = stream::empty::<i32>();
    let observable = FromStreamAcked::new(stream, time.scheduler()).map(|(value, _)| value);
    let (checker, observer) = Checker::new();

    let _subscription = observable.subscribe(observer);
    time.advance_by(Duration::ZERO);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_wait_for_ack() {
    let time = VirtualTime::new();
    let polled = Arc::new(Mutex::new(0));
    let stream = stream::iter([111, 222, 333]).inspect({
        let polled = polled.clone();
        move |_| polled.with_mut(|polled| *polled += 1)
    });
    let acks = Acks::default();
    let observable = FromStreamAcked::new(stream, time.scheduler()).map(keep_ack(&acks));
    let (checker, observer) = Checker::new();

    let _subscription = observable.subscribe(observer);
    time.advance_by(Duration::ZERO);
    assert_eq!(checker.values(), [111]);
    assert_eq!(polled.clone_value(), 1);

    // Nothing more without an acknowledgement, however long the wait.
    time.advance_by(Duration::from_secs(1));
    assert_eq!(checker.values(), [111]);
    assert_eq!(polled.clone_value(), 1);

    ack_first(&acks);
    time.advance_by(Duration::ZERO);
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(polled.clone_value(), 2);

    ack_first(&acks);
    time.advance_by(Duration::ZERO);
    assert_eq!(checker.values(), [111, 222, 333]);
    assert_eq!(checker.state(), State::Active);

    ack_first(&acks);
    time.advance_by(Duration::ZERO);
    assert_eq!(checker.values(), [111, 222, 333]);
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_with_permits() {
    let time = VirtualTime::new();
    let stream = stream::iter([111, 222, 333, 444]);
    let acks = Acks::default();
    let observable =
        FromStreamAcked::with_permits(stream, time.scheduler(), NonZeroUsize::new(2).unwrap())
            .map(keep_ack(&acks));
    let (checker, observer) = Checker::new();

    let _subscription = observable.subscribe(observer);
    time.advance_by(Duration::ZERO);
    assert_eq!(checker.values(), [111, 222]);

    ack_first(&acks);
    time.advance_by(Duration::ZERO);
    assert_eq!(checker.values(), [111, 222, 333]);

    // Two acknowledgements at once give two permits back.
    drop(acks.take_value());
    time.advance_by(Duration::ZERO);
    assert_eq!(checker.values(), [111, 222, 333, 444]);
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_ack_on_next() {
    // A `StreamAck` dropped while its item is delivered gives the permit back at once: the stream
    // is read as fast as with `FromStream`.
    let time = VirtualTime::new();
    let stream = stream::iter([111, 222, 333]);
    let observable = FromStreamAcked::new(stream, time.scheduler()).map(|(value, _)| value);
    let (checker, observer) = Checker::new();

    let _subscription = observable.subscribe(observer);
    time.advance_by(Duration::ZERO);
    assert_eq!(checker.values(), [111, 222, 333]);
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_ack_by_filter() {
    // An operator that drops an item drops its `StreamAck`, which acknowledges it.
    let time = VirtualTime::new();
    let stream = stream::iter([111, 222, 333]);
    let acks = Acks::default();
    let observable = FromStreamAcked::new(stream, time.scheduler())
        .filter(|(value, _)| *value != 222)
        .map(keep_ack(&acks));
    let (checker, observer) = Checker::new();

    let _subscription = observable.subscribe(observer);
    time.advance_by(Duration::ZERO);
    assert_eq!(checker.values(), [111]);

    ack_first(&acks);
    time.advance_by(Duration::ZERO);
    assert_eq!(checker.values(), [111, 333]);
}

#[test]
fn test_ack_on_another_thread() {
    let time = VirtualTime::new();
    let stream = stream::iter([111, 222]);
    let acks = Acks::default();
    let observable = FromStreamAcked::new(stream, time.scheduler()).map(keep_ack(&acks));
    let (checker, observer) = Checker::new();

    let _subscription = observable.subscribe(observer);
    time.advance_by(Duration::ZERO);
    assert_eq!(checker.values(), [111]);

    let ack = acks.with_mut(|acks| acks.remove(0));
    std::thread::spawn(move || drop(ack)).join().unwrap();
    time.advance_by(Duration::ZERO);
    assert_eq!(checker.values(), [111, 222]);
}

#[test]
fn test_unsubscribe() {
    let time = VirtualTime::new();
    let (tx, rx) = futures::channel::mpsc::unbounded();
    let acks = Acks::default();
    let observable = FromStreamAcked::new(rx, time.scheduler()).map(keep_ack(&acks));
    let (checker, observer) = Checker::new();

    let subscription = observable.subscribe(observer);
    tx.unbounded_send(111).unwrap();
    tx.unbounded_send(222).unwrap();
    time.advance_by(Duration::ZERO);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);

    subscription.dispose();
    time.advance_by(Duration::ZERO);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Dropped);
    assert_eq!(time.pending_tasks(), 0);
}

#[test]
fn test_ack_after_unsubscribe() {
    let time = VirtualTime::new();
    let stream = stream::iter([111, 222]);
    let acks = Acks::default();
    let observable = FromStreamAcked::new(stream, time.scheduler()).map(keep_ack(&acks));
    let (checker, observer) = Checker::new();

    let subscription = observable.subscribe(observer);
    time.advance_by(Duration::ZERO);
    assert_eq!(checker.values(), [111]);

    subscription.dispose();
    time.advance_by(Duration::ZERO);
    ack_first(&acks);
    time.advance_by(Duration::ZERO);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Dropped);
    assert_eq!(time.pending_tasks(), 0);
}

#[test]
fn test_unsub_on_next_by_take() {
    let time = VirtualTime::new();
    let stream = stream::iter([111, 222, 333]);
    let observable = FromStreamAcked::new(stream, time.scheduler())
        .map(|(value, _)| value)
        .take(2);
    let (checker, observer) = Checker::new();

    let _subscription = observable.subscribe(observer);
    time.advance_by(Duration::ZERO);
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(time.pending_tasks(), 0);
}

#[test]
fn test_async() {
    block_on(|scheduler| async move {
        let (mut tx, rx) = futures::channel::mpsc::unbounded();
        let acks = Acks::default();
        let observable = FromStreamAcked::new(rx, scheduler.clone()).map(keep_ack(&acks));
        let (checker, observer) = Checker::new();

        let _subscription = scheduler
            .spawn(async move { observable.subscribe(observer) })
            .await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);

        tx.send(111).await.unwrap();
        tx.send(222).await.unwrap();
        scheduler.sleep(DURATION_10_MS).await;
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Active);

        ack_first(&acks);
        scheduler.sleep(DURATION_10_MS).await;
        assert_eq!(checker.values(), [111, 222]);
        assert_eq!(checker.state(), State::Active);

        drop(tx);
        ack_first(&acks);
        scheduler.sleep(DURATION_10_MS).await;
        assert_eq!(checker.values(), [111, 222]);
        assert_eq!(checker.state(), State::Completed);
    });
}

#[test]
fn test_unsubscribe_with_always_ready_stream() {
    block_on(|scheduler| async move {
        // Infinite and always ready, each `StreamAck` dropped as its item is delivered: a permit is
        // always back by the next step, so only the yield between items lets the disposal in.
        let stream = stream::repeat(111);
        let observable = FromStreamAcked::new(stream, scheduler.clone()).map(|(value, _)| value);
        let (checker, observer) = Checker::new();

        let subscription = observable.subscribe(observer);
        scheduler.sleep(DURATION_10_MS).await;
        assert!(!checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);

        subscription.dispose();
        // Let the abort take effect before sampling the count.
        scheduler.sleep(DURATION_10_MS).await;
        let count = checker.values().len();
        scheduler.sleep(DURATION_10_MS).await;
        assert_eq!(checker.values().len(), count);
        assert_eq!(checker.state(), State::Dropped);
    });
}

#[test]
fn test_subscribe_by_different_observer() {
    // Each subscription has permits of its own: acknowledging the item of one lets only that one
    // go on.
    let time = VirtualTime::new();
    let stream = stream::iter(vec![111, 222, 333]);
    let acks = Acks::default();

    let observable = FromStreamAcked::new(stream, time.scheduler()).map(keep_ack(&acks));
    let observable_1 = observable;
    let observable_2 = observable_1.clone();

    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    let _subscription_1 = observable_1.subscribe(observer_1);
    time.advance_by(Duration::ZERO);

    let (on_next, on_termination) = observer_2.into_callbacks();
    let _subscription_2 = observable_2.subscribe_with_callback(on_next, on_termination);
    time.advance_by(Duration::ZERO);
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_2.values(), [111]);

    // The oldest `StreamAck` is the first subscription's.
    ack_first(&acks);
    time.advance_by(Duration::ZERO);
    assert_eq!(checker_1.values(), [111, 222]);
    assert_eq!(checker_2.values(), [111]);

    // The second subscription's, then the first's again.
    ack_first(&acks);
    ack_first(&acks);
    time.advance_by(Duration::ZERO);
    assert_eq!(checker_1.values(), [111, 222, 333]);
    assert_eq!(checker_2.values(), [111, 222]);

    drop(acks.take_value());
    time.advance_by(Duration::ZERO);
    drop(acks.take_value());
    time.advance_by(Duration::ZERO);
    assert_eq!(checker_1.values(), [111, 222, 333]);
    assert_eq!(checker_1.state(), State::Completed);
    assert_eq!(checker_2.values(), [111, 222, 333]);
    assert_eq!(checker_2.state(), State::Completed);
}

#[test]
fn test_stop_on_next() {
    let time = VirtualTime::new();
    let pulled = Arc::new(AtomicUsize::new(0));
    let pulled_stream = pulled.clone();
    // Infinite and always ready, each `StreamAck` dropped at once: only the observer's answer can
    // end it.
    let stream = stream::iter(1..).inspect(move |_| {
        pulled_stream.fetch_add(1, Ordering::SeqCst);
    });
    let observable = FromStreamAcked::new(stream, time.scheduler()).map(|(value, _)| value);
    let (checker, observer) = Checker::<_, Infallible>::stopping_after(2);

    let _subscription = observable.subscribe(observer);
    time.advance_by(Duration::ZERO);
    // The observer ended its own stream on the second value, so the scheduler stops polling
    // the stream right there and the observer is dropped instead of being completed.
    assert_eq!(checker.values(), [1, 2]);
    assert_eq!(checker.state(), State::Dropped);
    assert_eq!(pulled.load(Ordering::SeqCst), 2);

    time.advance_by(Duration::ZERO);
    assert_eq!(pulled.load(Ordering::SeqCst), 2);
    assert_eq!(time.pending_tasks(), 0);
}

#[test]
fn test_stop_on_next_by_take() {
    let time = VirtualTime::new();
    let pulled = Arc::new(AtomicUsize::new(0));
    let pulled_stream = pulled.clone();
    let stream = stream::iter(1..).inspect(move |_| {
        pulled_stream.fetch_add(1, Ordering::SeqCst);
    });
    let observable = FromStreamAcked::new(stream, time.scheduler())
        .map(|(value, _)| value)
        .take(2);
    let (checker, observer) = Checker::new();

    let _subscription = observable.subscribe(observer);
    time.advance_by(Duration::ZERO);
    // `take` stops the source once it has its values, so the values behind them are never
    // pulled: only the completion of the operator itself reaches the observer.
    assert_eq!(checker.values(), [1, 2]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(pulled.load(Ordering::SeqCst), 2);
}

#[test]
fn test_complete_after_next() {
    let time = VirtualTime::new();
    let (tx, rx) = futures::channel::mpsc::unbounded();
    let acks = Acks::default();
    let observable = FromStreamAcked::new(rx, time.scheduler()).map(keep_ack(&acks));
    let (checker, observer) = Checker::new();

    let _subscription = observable.subscribe(observer);
    time.advance_by(Duration::ZERO); // make sure it's subscribed
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);

    tx.unbounded_send(111).unwrap();
    drop(tx);
    time.advance_by(Duration::ZERO);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);

    ack_first(&acks);
    time.advance_by(Duration::ZERO);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_unsub_after_next() {
    let time = VirtualTime::new();
    let (tx, rx) = futures::channel::mpsc::unbounded();
    let acks = Acks::default();
    let observable = FromStreamAcked::new(rx, time.scheduler()).map(keep_ack(&acks));
    let (checker, observer) = Checker::new();

    let subscription = observable.subscribe(observer);
    time.advance_by(Duration::ZERO); // make sure it's subscribed
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);

    tx.unbounded_send(111).unwrap();
    time.advance_by(Duration::ZERO);
    subscription.dispose();
    time.advance_by(Duration::ZERO);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Dropped);
    assert_eq!(time.pending_tasks(), 0);
}

#[test]
fn test_unsub_after_completed() {
    let time = VirtualTime::new();
    let (tx, rx) = futures::channel::mpsc::unbounded::<i32>();
    let observable = FromStreamAcked::new(rx, time.scheduler()).map(|(value, _)| value);
    let (checker, observer) = Checker::new();

    let subscription = observable.subscribe(observer);
    time.advance_by(Duration::ZERO); // make sure it's subscribed
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);

    drop(tx);
    time.advance_by(Duration::ZERO);
    subscription.dispose();
    time.advance_by(Duration::ZERO);
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_order_with_continuous_next() {
    let time = VirtualTime::new();
    let (tx, rx) = futures::channel::mpsc::unbounded();
    let acks = Acks::default();
    let observable =
        FromStreamAcked::with_permits(rx, time.scheduler(), NonZeroUsize::new(10).unwrap())
            .map(keep_ack(&acks));
    let (checker, observer) = Checker::new();

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);

    let values = (0..1000).collect::<Vec<_>>();
    for i in &values {
        tx.unbounded_send(*i).unwrap();
    }
    // Ten at a time, in order, each batch acknowledged at once.
    for delivered in (10..=1000).step_by(10) {
        time.advance_by(Duration::ZERO);
        assert_eq!(checker.values(), values[..delivered]);
        drop(acks.take_value());
    }
    assert_eq!(checker.state(), State::Active);

    drop(tx);
    time.advance_by(Duration::ZERO);
    assert_eq!(checker.values(), values);
    assert_eq!(checker.state(), State::Completed);
}

/// The other tests decide when each `StreamAck` is dropped. Here they are dropped on a thread of
/// their own while the source's task runs on a pool, so that an acknowledgement and the task
/// finding no permit, then registering its waker, race: whatever the interleaving, every value
/// arrives once and in order, never more of them unacknowledged than there are permits, and the
/// stream completes rather than waiting for a wake that was lost.
#[test]
fn test_race_condition() {
    const VALUES: usize = 100;
    const PERMITS: usize = 2;
    let scheduler = ThreadPoolScheduler::from_pool(ThreadPool::new().unwrap());

    for _ in 0..500 {
        // The values delivered and not acknowledged yet, and the most of them at any time.
        let unacknowledged = Arc::new(AtomicUsize::new(0));
        let most = Arc::new(AtomicUsize::new(0));
        let (ack_sender, ack_receiver) = mpsc::channel::<StreamAck<Shared>>();
        let acknowledger = std::thread::spawn({
            let unacknowledged = unacknowledged.clone();
            move || {
                for ack in ack_receiver {
                    // Counted before the permit is back, so the count never runs behind it.
                    unacknowledged.fetch_sub(1, Ordering::SeqCst);
                    drop(ack);
                }
            }
        });
        let (checker, observer) = Checker::new();
        let (terminated, termination) = mpsc::channel();

        let observable = FromStreamAcked::with_permits(
            stream::iter(0..VALUES),
            scheduler.clone(),
            NonZeroUsize::new(PERMITS).unwrap(),
        )
        .map({
            let unacknowledged = unacknowledged.clone();
            let most = most.clone();
            move |(value, ack)| {
                let count = unacknowledged.fetch_add(1, Ordering::SeqCst) + 1;
                most.fetch_max(count, Ordering::SeqCst);
                ack_sender.send(ack).unwrap();
                value
            }
        })
        .do_after_termination(move |_| terminated.send(()).unwrap());

        let subscription = observable.subscribe(observer);
        termination
            .recv_timeout(RACE_TIMEOUT)
            .expect("the stream completes");
        assert_eq!(checker.values(), (0..VALUES).collect::<Vec<_>>());
        assert_eq!(checker.state(), State::Completed);
        assert!(most.load(Ordering::SeqCst) <= PERMITS);

        // The observer, with the sender of the acknowledgements, is dropped once it has been
        // terminated: the thread then runs out of acknowledgements and ends.
        drop(subscription);
        acknowledger.join().unwrap();
    }
}

#[test]
fn test_clone() {
    let time = VirtualTime::new();
    let stream = stream::iter(vec![111, 222, 333]);
    let observable = FromStreamAcked::new(stream, time.scheduler());
    _ = observable.clone();
}

#[test]
fn test_type_inference_with_subscribe() {
    let time = VirtualTime::new();
    let stream = stream::iter(vec![111, 222, 333]);
    let observable = FromStreamAcked::new(stream, time.scheduler());

    let observable = observable.map(|(value, _)| value);
    let (_, observer) = Checker::new();
    let _ = observable.subscribe(observer);
}

#[test]
fn test_type_inference_without_subscribe() {
    let time = VirtualTime::new();
    let stream = stream::iter(vec![111, 222, 333]);
    let observable = FromStreamAcked::new(stream, time.scheduler());

    observable.filter(|_| true);
}

// MARK: - Handled by `concat_map` and `flat_map`, the `StreamAck` held by the inner

/// A stream of `values` that counts how many of them were polled.
fn counted<T>(values: Vec<T>) -> (impl futures::Stream<Item = T>, Arc<Mutex<usize>>) {
    let polled = Arc::new(Mutex::new(0));
    let stream = stream::iter(values).inspect({
        let polled = polled.clone();
        move |_| polled.with_mut(|polled| *polled += 1)
    });
    (stream, polled)
}

#[test]
fn test_concat_map_hold() {
    let time = VirtualTime::new();
    let (mut sender_1, observable_1, channel_checker_1) = test_channel();
    let (mut sender_2, observable_2, channel_checker_2) = test_channel();
    let (checker, observer) = Checker::new();
    let (stream, polled) = counted(vec![1, 2]);

    let mut observables = [Some(observable_1), Some(observable_2)];
    let observable = FromStreamAcked::new(stream, time.scheduler()).concat_map(
        move |(value, ack): (usize, _)| observables[value - 1].take().unwrap().hold(ack),
    );

    let _subscription = observable.subscribe(observer);
    time.advance_by(Duration::ZERO);
    assert!(checker.values().is_empty());
    assert_eq!(polled.clone_value(), 1);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Initialized);

    assert!(sender_1.on_next(111).is_continue());
    time.advance_by(Duration::ZERO);
    assert_eq!(checker.values(), [111]);
    assert_eq!(polled.clone_value(), 1);

    // The end of the first inner acknowledges its value: the second is read only then.
    sender_1.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(polled.clone_value(), 1);
    time.advance_by(Duration::ZERO);
    assert_eq!(polled.clone_value(), 2);
    assert_eq!(channel_checker_1.state(), ChannelState::Completed);
    assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);

    assert!(sender_2.on_next(222).is_continue());
    assert_eq!(checker.values(), [111, 222]);

    // The stream has ended, but that is seen only once the second value is acknowledged.
    sender_2.on_termination(Termination::Completed);
    assert_eq!(checker.state(), State::Active);
    time.advance_by(Duration::ZERO);
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_concat_map_hold_empty_inner() {
    // An inner that ends at once acknowledges its value at once.
    let time = VirtualTime::new();
    let (checker, observer) = Checker::new();
    let (stream, polled) = counted(vec![1, 2, 3]);
    let observable =
        FromStreamAcked::new(stream, time.scheduler()).concat_map(|(_, ack)| Empty.hold(ack));

    let _subscription = observable.subscribe(observer);
    time.advance_by(Duration::ZERO);
    assert!(checker.values().is_empty());
    assert_eq!(polled.clone_value(), 3);
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_concat_map_hold_with_permits() {
    // Two permits: one value is read ahead while the inner of the previous one runs.
    let time = VirtualTime::new();
    let (sender_1, observable_1, channel_checker_1) = test_channel();
    let (mut sender_2, observable_2, channel_checker_2) = test_channel();
    let (mut sender_3, observable_3, channel_checker_3) = test_channel();
    let (checker, observer) = Checker::new();
    let (stream, polled) = counted(vec![1, 2, 3]);

    let mut observables = [Some(observable_1), Some(observable_2), Some(observable_3)];
    let observable =
        FromStreamAcked::with_permits(stream, time.scheduler(), NonZeroUsize::new(2).unwrap())
            .concat_map(move |(value, ack): (usize, _)| {
                observables[value - 1].take().unwrap().hold(ack)
            });

    let _subscription = observable.subscribe(observer);
    time.advance_by(Duration::ZERO);
    assert_eq!(polled.clone_value(), 2);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Initialized);

    sender_1.on_termination(Termination::<Infallible>::Completed);
    time.advance_by(Duration::ZERO);
    assert_eq!(polled.clone_value(), 3);
    assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_3.state(), ChannelState::Initialized);

    assert!(sender_2.on_next(222).is_continue());
    sender_2.on_termination(Termination::Completed);
    assert!(sender_3.on_next(333).is_continue());
    sender_3.on_termination(Termination::Completed);
    time.advance_by(Duration::ZERO);
    assert_eq!(checker.values(), [222, 333]);
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_flat_map_hold_with_permits() {
    // Two permits under a `flat_map`: two values handled at once, the third once one is done.
    let time = VirtualTime::new();
    let (sender_1, observable_1, channel_checker_1) = test_channel();
    let (sender_2, observable_2, channel_checker_2) = test_channel();
    let (sender_3, observable_3, channel_checker_3) = test_channel();
    let (checker, observer) = Checker::<i32, Infallible>::new();
    let (stream, polled) = counted(vec![1, 2, 3]);

    let mut observables = [Some(observable_1), Some(observable_2), Some(observable_3)];
    let observable =
        FromStreamAcked::with_permits(stream, time.scheduler(), NonZeroUsize::new(2).unwrap())
            .flat_map(move |(value, ack): (usize, _)| {
                observables[value - 1].take().unwrap().hold(ack)
            });

    let _subscription = observable.subscribe(observer);
    time.advance_by(Duration::ZERO);
    assert_eq!(polled.clone_value(), 2);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_3.state(), ChannelState::Initialized);

    // The second ends first: its permit lets the third in, while the first still runs.
    sender_2.on_termination(Termination::Completed);
    time.advance_by(Duration::ZERO);
    assert_eq!(polled.clone_value(), 3);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_3.state(), ChannelState::Subscribed);

    sender_1.on_termination(Termination::Completed);
    sender_3.on_termination(Termination::Completed);
    time.advance_by(Duration::ZERO);
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_concat_map_without_hold() {
    // A `StreamAck` the inner does not hold is dropped as the closure returns: the stream is read
    // ahead, as fast as a `FromStream`.
    let time = VirtualTime::new();
    let (_sender_1, observable_1, channel_checker_1) = test_channel::<i32, Infallible>();
    let (_sender_2, observable_2, channel_checker_2) = test_channel::<i32, Infallible>();
    let (checker, observer) = Checker::new();
    let (stream, polled) = counted(vec![1, 2]);

    let mut observables = [Some(observable_1), Some(observable_2)];
    let observable = FromStreamAcked::new(stream, time.scheduler())
        .concat_map(move |(value, _ack): (usize, _)| observables[value - 1].take().unwrap());

    let _subscription = observable.subscribe(observer);
    time.advance_by(Duration::ZERO);
    assert_eq!(polled.clone_value(), 2);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Initialized);
    assert_eq!(checker.state(), State::Active);
}

#[test]
fn test_concat_map_hold_error_inner() {
    let time = VirtualTime::new();
    let (mut sender_1, observable_1, channel_checker_1) = test_channel();
    let (checker, observer) = Checker::new();
    let (stream, polled) = counted(vec![Ok::<_, String>(1), Ok(2)]);

    let mut observable_1_option = Some(observable_1);
    let observable = FromTryStreamAcked::new(stream, time.scheduler())
        .concat_map(move |(_, ack)| observable_1_option.take().unwrap().hold(ack));

    let _subscription = observable.subscribe(observer);
    time.advance_by(Duration::ZERO);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    assert!(sender_1.on_next(111).is_continue());
    sender_1.on_termination(Termination::Error("boom".to_owned()));
    time.advance_by(Duration::ZERO);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Error("boom".to_owned()));
    assert_eq!(polled.clone_value(), 1);
    assert_eq!(time.pending_tasks(), 0);
}

#[test]
fn test_concat_map_hold_unsubscribe() {
    let time = VirtualTime::new();
    let (mut sender_1, observable_1, channel_checker_1) = test_channel::<_, Infallible>();
    let (checker, observer) = Checker::new();
    let (stream, polled) = counted(vec![1, 2]);

    let mut observable_1_option = Some(observable_1);
    let observable = FromStreamAcked::new(stream, time.scheduler())
        .concat_map(move |(_, ack)| observable_1_option.take().unwrap().hold(ack));

    let subscription = observable.subscribe(observer);
    time.advance_by(Duration::ZERO);
    assert!(sender_1.on_next(111).is_continue());
    assert_eq!(checker.values(), [111]);

    subscription.dispose();
    time.advance_by(Duration::ZERO);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Dropped);
    assert_eq!(channel_checker_1.state(), ChannelState::Unsubscribed);
    assert_eq!(polled.clone_value(), 1);
    assert_eq!(time.pending_tasks(), 0);
}

#[test]
fn test_concat_map_hold_abandon_inner() {
    // An inner that lets go of its observer without a termination drops its `StreamAck` with it,
    // so the next value is read. `concat_map` still waits for that inner to end, so the next
    // value waits behind it.
    let time = VirtualTime::new();
    let (sender_1, observable_1, channel_checker_1) = test_channel::<i32, Infallible>();
    let (_sender_2, observable_2, channel_checker_2) = test_channel::<i32, Infallible>();
    let (checker, observer) = Checker::new();
    let (stream, polled) = counted(vec![1, 2]);

    let mut observables = [Some(observable_1), Some(observable_2)];
    let observable = FromStreamAcked::new(stream, time.scheduler()).concat_map(
        move |(value, ack): (usize, _)| observables[value - 1].take().unwrap().hold(ack),
    );

    let _subscription = observable.subscribe(observer);
    time.advance_by(Duration::ZERO);
    assert_eq!(polled.clone_value(), 1);

    sender_1.abandon();
    time.advance_by(Duration::ZERO);
    assert_eq!(polled.clone_value(), 2);
    assert_eq!(channel_checker_1.state(), ChannelState::Abandoned);
    assert_eq!(channel_checker_2.state(), ChannelState::Initialized);
    assert_eq!(checker.state(), State::Active);
}

#[test]
fn test_concat_map_hold_one_at_a_time() {
    // Each value takes 100 ms to handle, on a timer: the next one waits for it.
    let time = VirtualTime::new();
    let scheduler = time.scheduler();
    let (checker, observer) = Checker::new();
    let (stream, polled) = counted(vec![1, 2, 3]);
    let observable =
        FromStreamAcked::new(stream, scheduler.clone()).concat_map(move |(value, ack)| {
            Timer::new(value, Duration::from_millis(100), scheduler.clone()).hold(ack)
        });

    let _subscription = observable.subscribe(observer);
    time.advance_by(Duration::ZERO);
    assert_eq!(polled.clone_value(), 1);

    time.advance_by(Duration::from_millis(99));
    assert!(checker.values().is_empty());
    assert_eq!(polled.clone_value(), 1);

    time.advance_by(Duration::from_millis(1));
    assert_eq!(checker.values(), [1]);
    assert_eq!(polled.clone_value(), 2);

    time.advance_by(Duration::from_millis(100));
    assert_eq!(checker.values(), [1, 2]);
    assert_eq!(polled.clone_value(), 3);

    time.advance_by(Duration::from_millis(100));
    assert_eq!(checker.values(), [1, 2, 3]);
    assert_eq!(checker.state(), State::Completed);
}
