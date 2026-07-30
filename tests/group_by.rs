mod tests_utils;

use crate::tests_utils::DURATION_10_MS;
use crate::tests_utils::checker::State;
use crate::tests_utils::test_channel::{ChannelState, test_channel};
use crate::tests_utils::test_runtime::block_on;
use crate::tests_utils::types::TestMutableHelper;
use rx_rust::disposable::Disposable;
use rx_rust::disposable::callback_disposal::CallbackDisposal;
use rx_rust::observable::Subscription;
use rx_rust::operators::creating::empty::Empty;
use rx_rust::operators::creating::throw::Throw;
use rx_rust::scheduler::Scheduler;
use rx_rust::subject::behavior_subject::BehaviorSubject;
use rx_rust::utils::types::{Mutable, Shared};
use rx_rust::{
    observable::{Observable, ObservableExt},
    observer::{Observer, Termination},
    operators::{creating::create::Create, transforming::group_by::GroupBy},
    subject::publish_subject::PublishSubject,
};
use rx_rust::{safe_lock, safe_lock_vec};
use std::convert::Infallible;
use tests_utils::{checker::Checker, test_struct::TestStruct};

#[test]
fn test_completed() {
    let mut subject = PublishSubject::default();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable = observable.group_by(|value| value % 2);

    let mut index = -1;
    let _subscription = observable
        .flat_map(|v| {
            index += 1;
            let i = index;
            v.map(move |v| 10 * i + v)
        })
        .subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);

    for i in 0..=9 {
        subject.on_next(i);
    }

    assert_eq!(checker.values(), [0, 11, 2, 13, 4, 15, 6, 17, 8, 19]);
    assert_eq!(checker.state(), State::Active);

    subject.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), [0, 11, 2, 13, 4, 15, 6, 17, 8, 19]);
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_error() {
    let mut subject = PublishSubject::default();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable = observable.group_by(|value| value % 2);

    let mut index = -1;
    let _subscription = observable
        .flat_map(|v| {
            index += 1;
            let i = index;
            v.map(move |v| 10 * i + v)
        })
        .subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);

    for i in 0..=9 {
        subject.on_next(i);
    }

    assert_eq!(checker.values(), [0, 11, 2, 13, 4, 15, 6, 17, 8, 19]);
    assert_eq!(checker.state(), State::Active);

    subject.on_termination(Termination::Error("error"));
    assert_eq!(checker.values(), [0, 11, 2, 13, 4, 15, 6, 17, 8, 19]);
    assert_eq!(checker.state(), State::Error("error"));
}

#[test]
fn test_unsubscribe() {
    let mut subject = PublishSubject::default();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable = observable.group_by(|value| value % 2);
    let observable_1 = observable;
    let observable_2 = observable_1.clone();

    let mut index = -1;
    let subscription_1 = observable_1
        .flat_map(|v| {
            index += 1;
            let i = index;
            v.map(move |v| 10 * i + v)
        })
        .subscribe(observer_1);

    let mut index = -1;
    let _subscription_2 = observable_2
        .flat_map(|v| {
            index += 1;
            let i = index;
            v.map(move |v| 10 * i + v)
        })
        .subscribe(observer_2);

    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);

    for i in 0..=9 {
        subject.on_next(i);
    }

    assert_eq!(checker_1.values(), [0, 11, 2, 13, 4, 15, 6, 17, 8, 19]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), [0, 11, 2, 13, 4, 15, 6, 17, 8, 19]);
    assert_eq!(checker_2.state(), State::Active);

    subscription_1.dispose();
    assert_eq!(checker_1.values(), [0, 11, 2, 13, 4, 15, 6, 17, 8, 19]);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(checker_2.values(), [0, 11, 2, 13, 4, 15, 6, 17, 8, 19]);
    assert_eq!(checker_2.state(), State::Active);

    subject.on_next(111);
    assert_eq!(checker_1.values(), [0, 11, 2, 13, 4, 15, 6, 17, 8, 19]);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(checker_2.values(), [0, 11, 2, 13, 4, 15, 6, 17, 8, 19, 121]);
    assert_eq!(checker_2.state(), State::Active);

    subject.on_termination(Termination::Error("error"));
    assert_eq!(checker_1.values(), [0, 11, 2, 13, 4, 15, 6, 17, 8, 19]);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(checker_2.values(), [0, 11, 2, 13, 4, 15, 6, 17, 8, 19, 121]);
    assert_eq!(checker_2.state(), State::Error("error"));
}

#[test]
fn test_ref() {
    let value_1 = 111;
    let value_2 = 222;
    let value_3 = 333;
    let error = 444;

    let mut subject: PublishSubject<'_, &i32, &i32> = PublishSubject::default();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable = observable.group_by(|value| *value % 2);

    let mut index = -1;
    let _subscription = observable
        .flat_map(|v| {
            index += 1;
            let i = index;
            v.map(move |v| (i, v))
        })
        .subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);

    subject.on_next(&value_1);
    assert_eq!(checker.values(), [(0, &value_1)]);
    assert_eq!(checker.state(), State::Active);

    subject.on_next(&value_2);
    assert_eq!(checker.values(), [(0, &value_1), (1, &value_2)]);
    assert_eq!(checker.state(), State::Active);

    subject.on_next(&value_3);
    assert_eq!(
        checker.values(),
        [(0, &value_1), (1, &value_2), (0, &value_3)]
    );
    assert_eq!(checker.state(), State::Active);

    subject.on_termination(Termination::Error(&error));
    assert_eq!(
        checker.values(),
        [(0, &value_1), (1, &value_2), (0, &value_3)]
    );
    assert_eq!(checker.state(), State::Error(&error));
}

#[test]
fn test_async() {
    block_on(|runtime| async move {
        let subject = PublishSubject::default();
        let (checker, observer) = Checker::new();

        // Custom operations
        let observable = subject.clone();
        let observable = observable.group_by(|value| value % 2);

        let subscription = runtime
            .spawn(async {
                let mut index = -1;
                observable
                    .flat_map(move |v| {
                        index += 1;
                        let i = index;
                        v.map(move |v| 10 * i + v)
                    })
                    .subscribe(observer)
            })
            .await
            .unwrap();
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);

        let mut subject_cloned = subject.clone();
        runtime
            .spawn(async move {
                for i in 0..=9 {
                    subject_cloned.on_next(i);
                }
            })
            .await
            .unwrap();
        assert_eq!(checker.values(), [0, 11, 2, 13, 4, 15, 6, 17, 8, 19]);
        assert_eq!(checker.state(), State::Active);

        runtime
            .spawn(async { subscription.dispose() })
            .await
            .unwrap();
        runtime.sleep(DURATION_10_MS).await;
        assert_eq!(checker.values(), [0, 11, 2, 13, 4, 15, 6, 17, 8, 19]);
        assert_eq!(checker.state(), State::Dropped);

        let subject_cloned = subject.clone();
        runtime
            .spawn(async move {
                subject_cloned.on_termination(Termination::Error("error"));
            })
            .await
            .unwrap();
        assert_eq!(checker.values(), [0, 11, 2, 13, 4, 15, 6, 17, 8, 19]);
        assert_eq!(checker.state(), State::Dropped);
    });
}

#[test]
fn test_subscribe_by_different_observer() {
    let mut subject = PublishSubject::default();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable = observable.group_by(|value| value % 2);
    let observable_1 = observable;
    let observable_2 = observable_1.clone();

    let mut index_1 = -1;
    let mut index_2 = -1;
    let _subscription_1 = observable_1
        .flat_map(|v| {
            index_1 += 1;
            let i = index_1;
            v.map(move |v| 10 * i + v)
        })
        .subscribe(observer_1);
    let (on_next, on_termination) = observer_2.into_callbacks();
    let _subscription_2 = observable_2
        .flat_map(|v| {
            index_2 += 1;
            let i = index_2;
            v.map(move |v| 10 * i + v)
        })
        .subscribe_with_callback(on_next, on_termination);
    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);

    for i in 0..=9 {
        subject.on_next(i);
    }
    assert_eq!(checker_1.values(), [0, 11, 2, 13, 4, 15, 6, 17, 8, 19]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), [0, 11, 2, 13, 4, 15, 6, 17, 8, 19]);
    assert_eq!(checker_2.state(), State::Active);

    subject.on_termination(Termination::Error("error"));
    assert_eq!(checker_1.values(), [0, 11, 2, 13, 4, 15, 6, 17, 8, 19]);
    assert_eq!(checker_1.state(), State::Error("error"));
    assert_eq!(checker_2.values(), [0, 11, 2, 13, 4, 15, 6, 17, 8, 19]);
    assert_eq!(checker_2.state(), State::Error("error"));
}

#[test]
fn test_unsub_on_next_by_take() {
    let mut index = -1;
    let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = observable.group_by(|value| value % 2);

    let _subscription = observable
        .flat_map(|v| {
            index += 1;
            let i = index;
            v.map(move |v| 10 * i + v)
        })
        .take(1)
        .subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    sender.on_next(111);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
}

#[test]
fn test_multiple_operation() {
    let mut subject = PublishSubject::default();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let mut index = -1;
    let observable = observable.group_by(|value| value % 4).group_by(|_| {
        index += 1;
        index % 2
    });

    let mut index_1 = -1;
    let mut index_2 = -1;
    let _subscription = observable
        .flat_map(|v| {
            index_1 += 1;
            let i = index_1;
            v.map(move |v| (i, v))
        })
        .flat_map(|v| {
            index_2 += 1;
            let i0 = v.0;
            let i = index_2;
            v.1.map(move |v| 100 * i0 + 10 * i + v)
        })
        .subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);

    for i in 0..=9 {
        subject.on_next(i);
    }

    assert_eq!(checker.values(), [0, 111, 22, 133, 4, 115, 26, 137, 8, 119]);
    assert_eq!(checker.state(), State::Active);

    subject.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), [0, 111, 22, 133, 4, 115, 26, 137, 8, 119]);
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_without_convenient_api() {
    let mut subject = PublishSubject::default();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable = GroupBy::new(observable, |value| value % 2);

    let mut index = -1;
    let _subscription = observable
        .flat_map(|v| {
            index += 1;
            let i = index;
            v.map(move |v| 10 * i + v)
        })
        .subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);

    for i in 0..=9 {
        subject.on_next(i);
    }

    assert_eq!(checker.values(), [0, 11, 2, 13, 4, 15, 6, 17, 8, 19]);
    assert_eq!(checker.state(), State::Active);

    subject.on_termination(Termination::Error("error"));
    assert_eq!(checker.values(), [0, 11, 2, 13, 4, 15, 6, 17, 8, 19]);
    assert_eq!(checker.state(), State::Error("error"));
}

#[test]
fn test_revert_completed() {
    let mut subject: PublishSubject<'_, i32, &'static str> = PublishSubject::default();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();
    let (checker_3, observer_3) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable = observable.group_by(|value| value % 2);
    let observable_1 = observable.clone().merge_all();
    let observable_2 = observable.clone().concat_all();
    let observable_3 = observable.switch();

    let _subscription_1 = observable_1.subscribe(observer_1);
    let _subscription_2 = observable_2.subscribe(observer_2);
    let _subscription_3 = observable_3.subscribe(observer_3);
    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert!(checker_3.values().is_empty());
    assert_eq!(checker_3.state(), State::Active);

    for i in 0..=9 {
        subject.on_next(i);
    }

    assert_eq!(checker_1.values(), [0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), [0, 2, 4, 6, 8]);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(checker_3.values(), [0, 1, 3, 5, 7, 9]);
    assert_eq!(checker_3.state(), State::Active);

    // The odd group is still queued in `concat_all` while the even one is active, so its values
    // are buffered. The source completion closes the even group, `concat_all` subscribes to the
    // odd group, and the buffered values are replayed instead of being lost.
    subject.on_termination(Termination::Completed);
    assert_eq!(checker_1.values(), [0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);
    assert_eq!(checker_1.state(), State::Completed);
    assert_eq!(checker_2.values(), [0, 2, 4, 6, 8, 1, 3, 5, 7, 9]);
    assert_eq!(checker_2.state(), State::Completed);
    assert_eq!(checker_3.values(), [0, 1, 3, 5, 7, 9]);
    assert_eq!(checker_3.state(), State::Completed);
}

#[test]
fn test_revert_error() {
    let mut subject: PublishSubject<'_, i32, &'static str> = PublishSubject::default();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();
    let (checker_3, observer_3) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable = observable.group_by(|value| value % 2);
    let observable_1 = observable.clone().merge_all();
    let observable_2 = observable.clone().concat_all();
    let observable_3 = observable.switch();

    let _subscription_1 = observable_1.subscribe(observer_1);
    let _subscription_2 = observable_2.subscribe(observer_2);
    let _subscription_3 = observable_3.subscribe(observer_3);
    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert!(checker_3.values().is_empty());
    assert_eq!(checker_3.state(), State::Active);

    for i in 0..=9 {
        subject.on_next(i);
    }

    assert_eq!(checker_1.values(), [0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), [0, 2, 4, 6, 8]);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(checker_3.values(), [0, 1, 3, 5, 7, 9]);
    assert_eq!(checker_3.state(), State::Active);

    subject.on_termination(Termination::Error("error"));
    assert_eq!(checker_1.values(), [0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);
    assert_eq!(checker_1.state(), State::Error("error"));
    assert_eq!(checker_2.values(), [0, 2, 4, 6, 8]);
    assert_eq!(checker_2.state(), State::Error("error"));
    assert_eq!(checker_3.values(), [0, 1, 3, 5, 7, 9]);
    assert_eq!(checker_3.state(), State::Error("error"));
}

#[test]
fn test_next_on_sub() {
    let mut subject = BehaviorSubject::new(111);
    let (termination_checker, termination_observer) = Checker::<Infallible, _>::new();
    let checker_sub_vec = Shared::new(Mutable::new(Vec::new()));

    // Custom operations
    let observable = subject.clone().group_by(|value| value % 2);

    let checker_sub_vec_cloned = checker_sub_vec.clone();
    let _subscription = observable.subscribe_with_callback(
        move |group| {
            let (checker, observer) = Checker::new();
            let sub = group.subscribe(observer);
            safe_lock_vec!(push: checker_sub_vec_cloned, (checker, sub));
        },
        |termination| {
            termination_observer.on_termination(termination);
        },
    );
    // The value the subject replays on subscription opens the first group, and the
    // group buffers it until the outer observer subscribes.
    assert_eq!(safe_lock_vec!(len: checker_sub_vec), 1);
    for (index, (checker, _)) in checker_sub_vec.test_lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), [111]);
                assert_eq!(checker.state(), State::Active);
            }
            _ => panic!(),
        }
    }
    assert_eq!(termination_checker.state(), State::Active);

    subject.on_next(222);
    assert_eq!(safe_lock_vec!(len: checker_sub_vec), 2);
    for (index, (checker, _)) in checker_sub_vec.test_lock_ref().iter().enumerate() {
        match index {
            0 => assert_eq!(checker.values(), [111]),
            1 => assert_eq!(checker.values(), [222]),
            _ => panic!(),
        }
    }

    subject.on_next(333);
    assert_eq!(safe_lock_vec!(len: checker_sub_vec), 2);
    for (index, (checker, _)) in checker_sub_vec.test_lock_ref().iter().enumerate() {
        match index {
            0 => assert_eq!(checker.values(), [111, 333]),
            1 => assert_eq!(checker.values(), [222]),
            _ => panic!(),
        }
    }

    subject.on_termination(Termination::<Infallible>::Completed);
    for (index, (checker, _)) in checker_sub_vec.test_lock_ref().iter().enumerate() {
        match index {
            0 => {
                assert_eq!(checker.values(), [111, 333]);
                assert_eq!(checker.state(), State::Completed);
            }
            1 => {
                assert_eq!(checker.values(), [222]);
                assert_eq!(checker.state(), State::Completed);
            }
            _ => panic!(),
        }
    }
    assert_eq!(termination_checker.state(), State::Completed);
}

#[test]
fn test_complete_on_sub() {
    let (termination_checker, termination_observer) = Checker::<Infallible, _>::new();
    let group_vec = Shared::new(Mutable::new(Vec::new()));

    // Custom operations
    let observable = Empty.group_by(|_value| 0);

    let group_vec_cloned = group_vec.clone();
    let _subscription = observable.subscribe_with_callback(
        move |group| safe_lock_vec!(push: group_vec_cloned, group),
        |termination| {
            termination_observer.on_termination(termination);
        },
    );
    // No value means no group.
    assert_eq!(safe_lock_vec!(len: group_vec), 0);
    assert_eq!(termination_checker.state(), State::Completed);
}

#[test]
fn test_error_on_sub() {
    let (termination_checker, termination_observer) = Checker::<Infallible, _>::new();
    let group_vec = Shared::new(Mutable::new(Vec::new()));

    // Custom operations
    let observable = Throw::new("error").group_by(|_value| 0);

    let group_vec_cloned = group_vec.clone();
    let _subscription = observable.subscribe_with_callback(
        move |group| safe_lock_vec!(push: group_vec_cloned, group),
        |termination| {
            termination_observer.on_termination(termination);
        },
    );
    assert_eq!(safe_lock_vec!(len: group_vec), 0);
    assert_eq!(termination_checker.state(), State::Error("error"));
}

#[test]
fn test_subscribe_groups_late_with_buffered_values() {
    use rx_rust::utils::types::MutableHelper;

    let (mut sender, observable, _channel_checker) = test_channel::<'_, i32, Infallible>();

    // Custom operations
    let observable = observable.group_by(|value| value % 2);

    // Collect the group observables without subscribing to them immediately.
    let group_vec = Shared::new(Mutable::new(Vec::new()));
    let group_vec_cloned = group_vec.clone();
    let _subscription = observable.subscribe_with_callback(
        move |group| safe_lock_vec!(push: group_vec_cloned, group),
        |_termination| {},
    );

    // Each key opens a group on its first value, and both groups buffer while
    // they have no subscriber.
    sender.on_next(1);
    sender.on_next(2);
    sender.on_next(3);
    sender.on_next(4);
    assert_eq!(safe_lock_vec!(len: group_vec), 2);

    let mut groups = group_vec.lock_mut(|mut lock| std::mem::take(&mut *lock));
    let group_even = groups.pop().unwrap();
    let group_odd = groups.pop().unwrap();

    // Subscribing replays the values of that group only.
    let (checker_odd, observer_odd) = Checker::new();
    let _sub_odd = group_odd.subscribe(observer_odd);
    assert_eq!(checker_odd.values(), [1, 3]);
    assert_eq!(checker_odd.state(), State::Active);

    let (checker_even, observer_even) = Checker::new();
    let _sub_even = group_even.subscribe(observer_even);
    assert_eq!(checker_even.values(), [2, 4]);
    assert_eq!(checker_even.state(), State::Active);

    // Later values are delivered directly.
    sender.on_next(5);
    sender.on_next(6);
    assert_eq!(checker_odd.values(), [1, 3, 5]);
    assert_eq!(checker_even.values(), [2, 4, 6]);

    // No extra group is emitted for keys that already have one.
    assert_eq!(safe_lock_vec!(len: group_vec), 0);

    sender.on_termination(Termination::Completed);
    assert_eq!(checker_odd.state(), State::Completed);
    assert_eq!(checker_even.state(), State::Completed);
}

#[test]
fn test_subscribe_group_after_termination() {
    use rx_rust::utils::types::MutableHelper;

    let (mut sender, observable, _channel_checker) = test_channel::<'_, i32, Infallible>();
    let (termination_checker, termination_observer) = Checker::<Infallible, _>::new();

    // Custom operations
    let observable = observable.group_by(|value| value % 2);

    // Hold the group observable without subscribing to it.
    let group_vec = Shared::new(Mutable::new(Vec::new()));
    let group_vec_cloned = group_vec.clone();
    let _subscription = observable.subscribe_with_callback(
        move |group| safe_lock_vec!(push: group_vec_cloned, group),
        |termination| termination_observer.on_termination(termination),
    );

    sender.on_next(111);
    assert_eq!(safe_lock_vec!(len: group_vec), 1);

    // The source terminates while the group is still unsubscribed. The group
    // itself was completed by the source termination.
    sender.on_termination(Termination::Completed);
    assert_eq!(termination_checker.state(), State::Completed);

    // A late subscriber observes the buffered value and the completion.
    let group = group_vec.lock_mut(|mut lock| lock.pop()).unwrap();
    let (checker, observer) = Checker::new();
    let _sub = group.subscribe(observer);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_subscribe_group_after_unsubscribe() {
    use rx_rust::utils::types::MutableHelper;

    let (mut sender, observable, _channel_checker) = test_channel::<'_, i32, Infallible>();

    // Custom operations
    let observable = observable.group_by(|value| value % 2);

    // Hold the group observable without subscribing to it.
    let group_vec = Shared::new(Mutable::new(Vec::new()));
    let group_vec_cloned = group_vec.clone();
    let subscription = observable.subscribe_with_callback(
        move |group| safe_lock_vec!(push: group_vec_cloned, group),
        |_termination| {},
    );

    sender.on_next(111);
    assert_eq!(safe_lock_vec!(len: group_vec), 1);

    // Unsubscribing is not a termination: the group never completed nor errored.
    drop(subscription);

    // A late subscriber still observes the buffered value, then is dropped
    // without receiving a termination.
    let group = group_vec.lock_mut(|mut lock| lock.pop()).unwrap();
    let (checker, observer) = Checker::new();
    let _sub = group.subscribe(observer);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Dropped);
}

#[test]
fn test_values_of_ended_group_are_discarded() {
    use rx_rust::utils::types::MutableHelper;

    let (mut sender, observable, _channel_checker) = test_channel::<'_, i32, Infallible>();

    // Custom operations
    let observable = observable.group_by(|value| value % 2);

    let group_vec = Shared::new(Mutable::new(Vec::new()));
    let group_vec_cloned = group_vec.clone();
    let _subscription = observable.subscribe_with_callback(
        move |group| safe_lock_vec!(push: group_vec_cloned, group),
        |_termination| {},
    );

    sender.on_next(1);
    sender.on_next(2);
    let mut groups = group_vec.lock_mut(|mut lock| std::mem::take(&mut *lock));
    let group_even = groups.pop().unwrap();
    let group_odd = groups.pop().unwrap();

    let (checker_odd, observer_odd) = Checker::new();
    let sub_odd = group_odd.subscribe(observer_odd);
    let (checker_even, observer_even) = Checker::new();
    let _sub_even = group_even.subscribe(observer_even);
    assert_eq!(checker_odd.values(), [1]);
    assert_eq!(checker_even.values(), [2]);

    // Unsubscribing ends the odd group.
    drop(sub_odd);
    assert_eq!(checker_odd.state(), State::Dropped);

    // Values of the ended group have nowhere to go and no new group is emitted
    // for its key, while the other group keeps working.
    sender.on_next(3);
    sender.on_next(4);
    assert_eq!(checker_odd.values(), [1]);
    assert_eq!(checker_even.values(), [2, 4]);
    assert_eq!(safe_lock_vec!(len: group_vec), 0);

    sender.on_termination(Termination::Completed);
    assert_eq!(checker_even.state(), State::Completed);
}

#[test]
fn test_unsub_on_inner_termination_still_terminates_other_groups() {
    use rx_rust::utils::types::MutableHelper;

    let (mut sender, observable, _channel_checker) = test_channel::<'_, i32, Infallible>();
    let (outer_termination_checker, outer_termination_observer) = Checker::<Infallible, _>::new();

    // Custom operations
    let observable = observable.group_by(|value| value % 2);

    let group_vec = Shared::new(Mutable::new(Vec::new()));
    let group_vec_cloned = group_vec.clone();
    let subscription = observable.subscribe_with_callback(
        move |group| safe_lock_vec!(push: group_vec_cloned, group),
        |termination| outer_termination_observer.on_termination(termination),
    );
    let outer_subscription = Shared::new(Mutable::new(Some(subscription)));

    sender.on_next(1);
    sender.on_next(2);
    let mut groups = group_vec.lock_mut(|mut lock| std::mem::take(&mut *lock));
    let group_even = groups.pop().unwrap();
    let group_odd = groups.pop().unwrap();

    // Both inner observers dispose the outer subscription when they terminate. Whichever
    // one runs first stops the pipeline, and the other must still observe its termination:
    // the source termination ends all groups as a single, uninterruptible event.
    let (checker_odd, observer_odd) = Checker::new();
    let (on_next_odd, on_termination_odd) = observer_odd.into_callbacks();
    let outer_subscription_cloned = outer_subscription.clone();
    let _sub_odd = group_odd.subscribe_with_callback(on_next_odd, move |termination| {
        drop(safe_lock!(mem_take: outer_subscription_cloned));
        on_termination_odd(termination);
    });
    let (checker_even, observer_even) = Checker::new();
    let (on_next_even, on_termination_even) = observer_even.into_callbacks();
    let outer_subscription_cloned = outer_subscription.clone();
    let _sub_even = group_even.subscribe_with_callback(on_next_even, move |termination| {
        drop(safe_lock!(mem_take: outer_subscription_cloned));
        on_termination_even(termination);
    });
    assert_eq!(checker_odd.values(), [1]);
    assert_eq!(checker_even.values(), [2]);

    sender.on_termination(Termination::Completed);
    assert_eq!(checker_odd.values(), [1]);
    assert_eq!(checker_odd.state(), State::Completed);
    assert_eq!(checker_even.values(), [2]);
    assert_eq!(checker_even.state(), State::Completed);
    // The outer termination itself is suppressed: it is queued after the inner
    // terminations, so the disposal does take effect for it.
    assert_eq!(outer_termination_checker.state(), State::Dropped);
}

#[test]
fn test_dropping_unsubscribed_group_releases_buffered_values() {
    use rx_rust::utils::types::MutableHelper;

    struct DropTracker(Shared<Mutable<usize>>);

    impl Drop for DropTracker {
        fn drop(&mut self) {
            self.0.lock_mut(|mut lock| *lock += 1);
        }
    }

    let (mut sender, observable, _channel_checker) = test_channel::<'_, DropTracker, Infallible>();

    // Custom operations
    let observable = observable.group_by(|_value| 0);

    let group_vec = Shared::new(Mutable::new(Vec::new()));
    let group_vec_cloned = group_vec.clone();
    let _outer_subscription = observable.subscribe_with_callback(
        move |group| safe_lock_vec!(push: group_vec_cloned, group),
        |_termination| {},
    );

    let drop_count = Shared::new(Mutable::new(0usize));
    sender.on_next(DropTracker(drop_count.clone()));
    assert_eq!(safe_lock_vec!(len: group_vec), 1);
    assert_eq!(safe_lock!(clone: drop_count), 0);

    // Dropping the only handle to an unsubscribed group must release its
    // buffered values even while the outer subscription remains active.
    let group = group_vec.lock_mut(|mut lock| lock.pop()).unwrap();
    drop(group);
    assert_eq!(safe_lock!(clone: drop_count), 1);
}

#[test]
fn test_dropping_ignored_value_does_not_poison_group_context() {
    use crate::tests_utils::panic::{PanicOnDrop, expect_panic_on_drop};
    use rx_rust::utils::types::MutableHelper;

    let (mut sender, observable, channel_checker) =
        test_channel::<'_, Option<PanicOnDrop>, Infallible>();

    // Custom operations
    let observable = observable.group_by(|_value| 0);

    let inner_subscriptions = Shared::new(Mutable::new(Vec::new()));
    let inner_subscriptions_cloned = inner_subscriptions.clone();
    let outer_subscription = observable.subscribe_with_callback(
        move |group| {
            let sub = group.subscribe_with_callback(|_value| {}, |_termination| {});
            safe_lock_vec!(push: inner_subscriptions_cloned, sub);
        },
        |_termination| {},
    );

    // The first value opens the group and carries no panicking payload.
    sender.on_next(None);
    assert_eq!(safe_lock_vec!(len: inner_subscriptions), 1);

    // Ending the group makes the following value take the ignored-value path.
    let inner_subscription = inner_subscriptions.lock_mut(|mut lock| lock.pop()).unwrap();
    drop(inner_subscription);

    expect_panic_on_drop(|value| sender.on_next(Some(value)));

    // The ignored value is dropped after releasing the context lock, so the
    // context is still usable after the intentional drop panic: disposing the
    // outer subscription locks the context again and unsubscribes the source.
    drop(outer_subscription);
    assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
}

#[test]
fn test_lifetime_sub() {
    // OK
    let life_marker = TestStruct;
    let _subscription;

    // Error
    // let _subscription;
    // let life_marker = TestStruct;

    {
        let observable = Create::new(|mut observer| {
            observer.on_next(1);
            observer.on_termination(Termination::<String>::Completed);
            Subscription::new(CallbackDisposal::new(|| {
                life_marker.consume_ref();
            }))
        });

        let observable = observable.group_by(|value| value.to_string());

        let (_, observer) = Checker::new();
        _subscription = observable.subscribe(observer);
    }
}

#[test]
fn test_lifetime_or() {
    // OK
    let life_marker_2 = TestStruct;
    let mut life_marker_1 = None;

    // Error
    // let mut life_marker_1 = None;
    // let life_marker_2 = TestStruct;

    {
        let observable = Create::new(|observer| {
            life_marker_1 = Some(observer);
            Subscription::default()
        });
        let observable = observable.group_by(|v: &i32| *v).merge_all().map(|_| None);

        let (_, mut observer) = Checker::<_, Infallible>::new();
        observer.on_next(Some(&life_marker_2));
        let _subscription = observable.subscribe(observer);
    }
}

#[test]
fn test_fn() {
    let mut s = TestStruct;

    let subject: PublishSubject<'_, i32, &str> = PublishSubject::default();

    // Custom operations
    let observable = subject.clone();
    let observable = observable.group_by(|value| {
        s.consume_mut();
        value.to_string()
    });

    let _ = observable.subscribe_with_callback(|_| {}, |_| {});
}

#[test]
fn test_clone() {
    let observable = Create::new(|mut observer| {
        observer.on_next(TestStruct);
        observer.on_termination(Termination::Error(TestStruct));
        Subscription::default()
    });
    let observable = observable.group_by(|_value| 0);
    _ = observable.clone(); // Make sure it's Clone when T and E are not Clone.
}

#[test]
fn test_type_inference_with_subscribe() {
    // Custom operations
    let subject: PublishSubject<'_, i32, String> = PublishSubject::default();
    let observable = subject.group_by(|value| value.to_string());

    let observable = observable.filter(|_| true);
    let (_, observer) = Checker::new();
    let _ = observable.subscribe(observer);
}

#[test]
fn test_type_inference_without_subscribe() {
    // Custom operations
    let subject: PublishSubject<'_, i32, String> = PublishSubject::default();
    let observable = subject.group_by(|value| value.to_string());

    observable.filter(|_| true);
}
