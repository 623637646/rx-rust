mod tests_utils;

use crate::tests_utils::checker::State;
use crate::tests_utils::test_runtime::block_on;
use rx_rust::disposable::Disposable;
use rx_rust::observable::{Observable, ObservableExt};
use rx_rust::observer::{Observer, Termination};
use rx_rust::subject::unicast_subject::{
    UnicastSender, unicast_subject, unicast_subject_with_capacity,
};
use rx_rust::utils::types::{Mutable, Shared};
use rx_rust::{safe_lock_option, safe_lock_option_disposable};
use std::convert::Infallible;
use std::sync::atomic::{AtomicUsize, Ordering};
use tests_utils::checker::Checker;
use tests_utils::test_struct::TestStruct;

#[test]
fn test_completed() {
    let (mut sender, observable) = unicast_subject::<i32, Infallible>();
    let (checker, observer) = Checker::new();

    let _subscription = observable.subscribe(observer);
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Active);
    assert!(!sender.is_disposed());

    sender.on_next(111);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert!(!sender.is_disposed());

    sender.on_next(222);
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Active);
    assert!(!sender.is_disposed());

    sender.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_error() {
    let (mut sender, observable) = unicast_subject::<i32, &str>();
    let (checker, observer) = Checker::new();

    let _subscription = observable.subscribe(observer);

    sender.on_next(111);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);

    sender.on_termination(Termination::Error("error"));
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Error("error"));
}

#[test]
fn test_unsubscribe() {
    let (mut sender, observable) = unicast_subject::<i32, Infallible>();
    let (checker, observer) = Checker::new();

    let subscription = observable.subscribe(observer);
    sender.on_next(111);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert!(!sender.is_disposed());

    drop(subscription);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Dropped);
    assert!(sender.is_disposed());

    // The events after the disposal are dropped.
    sender.on_next(222);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Dropped);

    sender.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Dropped);
}

#[test]
fn test_next_before_subscribe() {
    let (mut sender, observable) = unicast_subject::<i32, Infallible>();

    // The values sent before the subscription are buffered instead of being dropped.
    sender.on_next(111);
    sender.on_next(222);
    assert!(!sender.is_disposed());

    let (checker, observer) = Checker::new();
    let _subscription = observable.subscribe(observer);
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Active);

    sender.on_next(333);
    assert_eq!(checker.values(), [111, 222, 333]);
    assert_eq!(checker.state(), State::Active);

    sender.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [111, 222, 333]);
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_complete_before_subscribe() {
    let (mut sender, observable) = unicast_subject::<i32, Infallible>();

    sender.on_next(111);
    sender.on_next(222);
    sender.on_termination(Termination::Completed);

    let (checker, observer) = Checker::new();
    let _subscription = observable.subscribe(observer);
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_error_before_subscribe() {
    let (mut sender, observable) = unicast_subject::<i32, &str>();

    sender.on_next(111);
    sender.on_next(222);
    sender.on_termination(Termination::Error("error"));

    // The buffered values are delivered before the error, unlike a `ReplaySubject`, which drops
    // them.
    let (checker, observer) = Checker::new();
    let _subscription = observable.subscribe(observer);
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Error("error"));
}

#[test]
fn test_with_capacity() {
    let (mut sender, observable) = unicast_subject_with_capacity::<i32, Infallible>(2);

    sender.on_next(111);
    sender.on_next(222);
    // The capacity is only a hint, so the buffer still grows.
    sender.on_next(333);

    let (checker, observer) = Checker::new();
    let _subscription = observable.subscribe(observer);
    assert_eq!(checker.values(), [111, 222, 333]);
    assert_eq!(checker.state(), State::Active);
}

#[test]
fn test_drop_observable_without_subscribe() {
    let (mut sender, observable) = unicast_subject::<i32, Infallible>();

    sender.on_next(111);
    assert!(!sender.is_disposed());

    // Dropping the observable end closes the pipe, so the buffer stops growing.
    drop(observable);
    assert!(sender.is_disposed());

    sender.on_next(222);
    sender.on_termination(Termination::Completed);
}

#[test]
fn test_drop_sender_without_termination() {
    let (mut sender, observable) = unicast_subject::<i32, Infallible>();
    let (checker, observer) = Checker::new();
    let _subscription = observable.subscribe(observer);

    sender.on_next(111);
    // A dropped sender is not reported as a completion.
    drop(sender);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
}

#[test]
fn test_ref() {
    let value_1 = 111;
    let value_2 = 222;

    let (mut sender, observable) = unicast_subject::<&i32, Infallible>();
    sender.on_next(&value_1);

    let (checker, observer) = Checker::new();
    let _subscription = observable.subscribe(observer);
    assert_eq!(checker.values(), [&value_1]);
    assert_eq!(checker.state(), State::Active);

    sender.on_next(&value_2);
    assert_eq!(checker.values(), [&value_1, &value_2]);

    sender.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [&value_1, &value_2]);
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_async() {
    block_on(|runtime| async move {
        let (sender, observable) = unicast_subject::<i32, Infallible>();
        let (checker, observer) = Checker::new();

        let mut sender = runtime
            .spawn(async move {
                let mut sender = sender;
                sender.on_next(111);
                sender
            })
            .await
            .unwrap();
        assert_eq!(checker.values(), []);
        assert_eq!(checker.state(), State::Active);

        let _subscription = runtime
            .spawn(async move { observable.subscribe(observer) })
            .await
            .unwrap();
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Active);

        let sender = runtime
            .spawn(async move {
                sender.on_next(222);
                sender
            })
            .await
            .unwrap();
        assert_eq!(checker.values(), [111, 222]);
        assert_eq!(checker.state(), State::Active);

        runtime
            .spawn(async move {
                sender.on_termination(Termination::Completed);
            })
            .await
            .unwrap();
        assert_eq!(checker.values(), [111, 222]);
        assert_eq!(checker.state(), State::Completed);
    });
}

/// Sending a value from an observer that the sender itself is delivering to is impossible: the
/// sender is uniquely owned, so [`Observer::on_next`] holds the only handle to it while the
/// observer runs. The re-entrant path is therefore the replay of the buffered values, which
/// [`Observable::subscribe`] drives while the sender is free.
#[test]
fn test_next_on_next() {
    let (mut sender, observable) = unicast_subject::<i32, Infallible>();
    sender.on_next(1);
    sender.on_next(2);

    let (checker, observer) = Checker::new();
    let sender_holder = Shared::new(Mutable::new(Some(sender)));
    let sender_holder_cloned = sender_holder.clone();
    let _subscription = observable
        .hook_on_next(move |downstream, value: i32| {
            downstream.on_next(value);
            if value == 1 {
                let mut sender = safe_lock_option!(take: sender_holder_cloned)
                    .expect("the sender is put back after every re-entrant call");
                sender.on_next(111);
                safe_lock_option!(replace: sender_holder_cloned, sender);
            }
        })
        .subscribe(observer);

    // The re-entrant value comes after the buffered values, in arrival order.
    assert_eq!(checker.values(), [1, 2, 111]);
    assert_eq!(checker.state(), State::Active);

    let mut sender = safe_lock_option!(take: sender_holder).unwrap();
    sender.on_next(222);
    assert_eq!(checker.values(), [1, 2, 111, 222]);
    assert_eq!(checker.state(), State::Active);
}

#[test]
fn test_complete_on_next() {
    let (mut sender, observable) = unicast_subject::<i32, Infallible>();
    sender.on_next(111);
    sender.on_next(222);

    let (checker, observer) = Checker::new();
    let sender_holder = Shared::new(Mutable::new(Some(sender)));
    let sender_holder_cloned = sender_holder.clone();
    let _subscription = observable
        .hook_on_next(move |downstream, value: i32| {
            downstream.on_next(value);
            if value == 111 {
                let sender = safe_lock_option!(take: sender_holder_cloned).unwrap();
                sender.on_termination(Termination::Completed);
            }
        })
        .subscribe(observer);

    // The termination is delivered after the remaining buffered values.
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Completed);
    assert!(safe_lock_option!(is_none: sender_holder));
}

#[test]
fn test_error_on_next() {
    let (mut sender, observable) = unicast_subject::<i32, &str>();
    sender.on_next(111);
    sender.on_next(222);

    let (checker, observer) = Checker::new();
    let sender_holder = Shared::new(Mutable::new(Some(sender)));
    let sender_holder_cloned = sender_holder.clone();
    let _subscription = observable
        .hook_on_next(move |downstream, value: i32| {
            downstream.on_next(value);
            if value == 111 {
                let sender = safe_lock_option!(take: sender_holder_cloned).unwrap();
                sender.on_termination(Termination::Error("error"));
            }
        })
        .subscribe(observer);

    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Error("error"));
    assert!(safe_lock_option!(is_none: sender_holder));
}

#[test]
fn test_unsub_on_next() {
    let (mut sender, observable) = unicast_subject::<i32, Infallible>();
    let (checker, observer) = Checker::new();

    let subscription = Shared::new(Mutable::new(None));
    let subscription_cloned = subscription.clone();
    safe_lock_option!(replace: subscription,
        observable
            .hook_on_next(move |downstream, value| {
                downstream.on_next(value);
                safe_lock_option_disposable!(dispose: subscription_cloned);
            })
            .subscribe(observer)
    );

    sender.on_next(111);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Dropped);
    assert!(sender.is_disposed());

    sender.on_next(222);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Dropped);
}

#[test]
fn test_unsub_on_next_by_take() {
    let (mut sender, observable) = unicast_subject::<i32, Infallible>();
    let (checker, observer) = Checker::new();

    let _subscription = observable.take(1).subscribe(observer);

    sender.on_next(111);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Completed);
    assert!(sender.is_disposed());

    sender.on_next(222);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_take_during_replay() {
    let (mut sender, observable) = unicast_subject::<i32, Infallible>();
    sender.on_next(111);
    sender.on_next(222);
    sender.on_next(333);

    // A downstream that stops during the replay of the buffered values closes the pipe once the
    // subscription it disposes exists, which is as soon as `subscribe` returns.
    let (checker, observer) = Checker::new();
    let _subscription = observable.take(1).subscribe(observer);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Completed);
    assert!(sender.is_disposed());

    sender.on_next(444);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_unsub_on_completed() {
    let (mut sender, observable) = unicast_subject::<i32, Infallible>();
    let (checker, observer) = Checker::new();

    let subscription = Shared::new(Mutable::new(None));
    let subscription_cloned = subscription.clone();
    safe_lock_option!(replace: subscription,
        observable
            .hook_on_termination(move |downstream, termination| {
                safe_lock_option_disposable!(dispose: subscription_cloned);
                downstream.on_termination(termination);
            })
            .subscribe(observer)
    );

    sender.on_next(111);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);

    sender.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Completed);
}

/// A value that sends another value into the pipe while being dropped, to check that a value is
/// never dropped while the state of the pipe is locked. Dropping it under the lock would panic in
/// single-threaded builds and deadlock otherwise.
struct ReentrantValue {
    sender: SenderHolder,
    reenter: bool,
    drop_count: Shared<AtomicUsize>,
}

type SenderHolder = Shared<Mutable<Option<UnicastSender<'static, ReentrantValue, Infallible>>>>;

impl ReentrantValue {
    fn new(sender: &SenderHolder, drop_count: &Shared<AtomicUsize>) -> Self {
        Self {
            sender: sender.clone(),
            reenter: true,
            drop_count: drop_count.clone(),
        }
    }
}

impl Drop for ReentrantValue {
    fn drop(&mut self) {
        self.drop_count.fetch_add(1, Ordering::SeqCst);
        if !self.reenter {
            return;
        }
        // The holder is emptied while the re-entrant call runs, so the value sent below re-enters
        // the pipe without recursing any further.
        let Some(mut sender) = safe_lock_option!(take: self.sender) else {
            return;
        };
        sender.on_next(ReentrantValue {
            sender: self.sender.clone(),
            reenter: false,
            drop_count: self.drop_count.clone(),
        });
        safe_lock_option!(replace: self.sender, sender);
    }
}

#[test]
fn test_drop_buffered_value_outside_lock() {
    let drop_count = Shared::new(AtomicUsize::new(0));
    let sender_holder: SenderHolder = Shared::new(Mutable::new(None));
    let (mut sender, observable) = unicast_subject::<ReentrantValue, Infallible>();
    sender.on_next(ReentrantValue::new(&sender_holder, &drop_count));
    sender.on_next(ReentrantValue::new(&sender_holder, &drop_count));
    safe_lock_option!(replace: sender_holder, sender);

    // Closing the pipe drops the buffered values, each of which re-enters the pipe while being
    // dropped. The re-entrant values are dropped as well, because the pipe is closed by then.
    drop(observable);
    assert_eq!(drop_count.load(Ordering::SeqCst), 4);
    assert!(
        safe_lock_option!(take: sender_holder)
            .unwrap()
            .is_disposed()
    );
}

#[test]
fn test_drop_replayed_value_outside_lock() {
    let drop_count = Shared::new(AtomicUsize::new(0));
    let sender_holder: SenderHolder = Shared::new(Mutable::new(None));
    let (mut sender, observable) = unicast_subject::<ReentrantValue, Infallible>();
    sender.on_next(ReentrantValue::new(&sender_holder, &drop_count));
    sender.on_next(ReentrantValue::new(&sender_holder, &drop_count));
    safe_lock_option!(replace: sender_holder, sender);

    // The observer drops each replayed value, which re-enters the pipe while the replay is still
    // running. The re-entrant values are then replayed and dropped in the same way.
    let _subscription = observable.subscribe_with_callback(drop, |_| {});
    assert_eq!(drop_count.load(Ordering::SeqCst), 4);
}

#[test]
fn test_non_clone() {
    // Make sure the pipe works when neither the item nor the error is `Clone`.
    let (mut sender, observable) = unicast_subject::<TestStruct, TestStruct>();
    sender.on_next(TestStruct);
    let _subscription = observable.subscribe_with_callback(TestStruct::consume, |_| {});
    sender.on_termination(Termination::Error(TestStruct));
}

#[test]
fn test_lifetime_or_sub() {
    // OK
    let life_marker = TestStruct;
    let _subscription;

    // Error
    // let _subscription;
    // let life_marker = TestStruct;

    {
        let (_, mut observer) = Checker::<_, Infallible>::new();
        observer.on_next(&life_marker);
        let (_sender, observable) = unicast_subject();
        _subscription = observable.subscribe(observer);
    }
}

#[test]
fn test_type_inference_with_subscribe() {
    // Custom operations
    let (_sender, observable) = unicast_subject::<i32, String>();

    let observable = observable.filter(|_| true);
    let (_, observer) = Checker::new();
    let _ = observable.subscribe(observer);
}

#[test]
fn test_type_inference_without_subscribe() {
    // Custom operations
    let (_sender, observable) = unicast_subject::<i32, String>();

    observable.filter(|_| true);
}
