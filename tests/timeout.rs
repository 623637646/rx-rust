mod tests_utils;

use crate::tests_utils::{
    DURATION_DEVIATION, DURATION_LOGICAL, DURATION_NEXT_LOOP,
    checker::{Checker, State},
    test_channel::{ChannelState, test_channel},
    test_runtime::block_on,
    test_struct::TestStruct,
};
use rx_rust::{
    disposable::{Disposable, subscription::Subscription},
    observable::{Observable, observable_ext::ObservableExt},
    observer::{Observer, Termination},
    operators::{
        creating::create::Create,
        utility::timeout::{self, Timeout},
    },
    scheduler::Scheduler,
    subject::publish_subject::PublishSubject,
};
use std::{convert::Infallible, sync::atomic::Ordering, time::Duration};

#[test]
fn test_completed() {
    block_on(|runtime| async move {
        let (mut sender, observable, channel_checker) = test_channel();
        let (checker, observer) = Checker::new();

        // Custom operations
        let observable = observable.timeout(DURATION_LOGICAL, runtime.clone());

        let _subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        sender.on_next(111);
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        runtime.sleep(DURATION_LOGICAL - DURATION_DEVIATION).await;
        sender.on_next(222);
        assert_eq!(checker.values(), [111, 222]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        sender.on_termination(Termination::<Infallible>::Completed);
        assert_eq!(checker.values(), [111, 222]);
        assert_eq!(checker.state(), State::Completed);
        assert_eq!(channel_checker.state(), ChannelState::Completed);
    });
}

#[test]
fn test_error() {
    block_on(|runtime| async move {
        let (mut sender, observable, channel_checker) = test_channel();
        let (checker, observer) = Checker::new();

        // Custom operations
        let observable = observable.timeout(DURATION_LOGICAL, runtime.clone());

        let _subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        sender.on_next(111);
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        runtime.sleep(DURATION_LOGICAL - DURATION_DEVIATION).await;
        sender.on_next(222);
        assert_eq!(checker.values(), [111, 222]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        sender.on_termination(Termination::Error("error"));
        assert_eq!(checker.values(), [111, 222]);
        assert_eq!(
            checker.state(),
            State::Error(timeout::Error::SourceError("error"))
        );
        assert_eq!(channel_checker.state(), ChannelState::Error("error"));
    });
}

#[test]
fn test_timeout() {
    block_on(|runtime| async move {
        let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
        let (checker, observer) = Checker::new();

        // Custom operations
        let observable = observable.timeout(DURATION_LOGICAL, runtime.clone());

        let _subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        sender.on_next(111);
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        runtime.sleep(DURATION_LOGICAL - DURATION_DEVIATION).await;
        sender.on_next(222);
        assert_eq!(checker.values(), [111, 222]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        runtime.sleep(DURATION_LOGICAL + DURATION_DEVIATION).await;
        assert_eq!(checker.values(), [111, 222]);
        assert_eq!(checker.state(), State::Error(timeout::Error::Timeout));
        assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
    });
}

#[test]
fn test_timeout_0_duration() {
    block_on(|mut runtime| async move {
        runtime.mock_delay = true;
        let (_, observable, channel_checker) = test_channel::<'_, i32, Infallible>();
        let (checker, observer) = Checker::new();

        // Custom operations
        let observable = observable.timeout(Duration::ZERO, runtime.clone());

        let _subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        runtime.sleep(DURATION_DEVIATION).await;
        assert_eq!(checker.values(), []);
        assert_eq!(checker.state(), State::Error(timeout::Error::Timeout));
        assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
    });
}
#[test]
fn test_unsubscribe() {
    block_on(|mut runtime| async move {
        runtime.mock_delay = true;
        let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
        let (checker, observer) = Checker::new();

        // Custom operations
        let observable = observable.timeout(DURATION_LOGICAL, runtime.clone());

        let subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        sender.on_next(111);
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        runtime.sleep(DURATION_LOGICAL - DURATION_DEVIATION).await;
        sender.on_next(222);
        assert_eq!(checker.values(), [111, 222]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        subscription.dispose();
        assert_eq!(checker.values(), [111, 222]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);

        runtime.sleep(DURATION_DEVIATION).await;
        assert_eq!(checker.values(), [111, 222]);
        assert_eq!(checker.state(), State::Dropped);
        assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
    });
}

#[test]
fn test_async() {
    block_on(|runtime| async move {
        let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
        let (checker, observer) = Checker::new();

        // Custom operations
        let observable = observable.timeout(DURATION_LOGICAL, runtime.clone());

        let _subscription = runtime
            .spawn(async move { observable.subscribe(observer) })
            .await
            .unwrap();
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        let mut sender = runtime
            .spawn(async move {
                sender.on_next(111);
                sender
            })
            .await
            .unwrap();
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        runtime.sleep(DURATION_LOGICAL - DURATION_DEVIATION).await;
        let _sender = runtime
            .spawn(async move {
                sender.on_next(222);
                sender
            })
            .await
            .unwrap();
        assert_eq!(checker.values(), [111, 222]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        runtime.sleep(DURATION_LOGICAL + DURATION_DEVIATION).await;
        assert_eq!(checker.values(), [111, 222]);
        assert_eq!(checker.state(), State::Error(timeout::Error::Timeout));
        assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
    });
}

#[test]
fn test_subscribe_by_different_observer() {
    block_on(|runtime| async move {
        let mut subject = PublishSubject::<'_, _, Infallible>::default();
        let (checker_1, observer_1) = Checker::new();
        let (checker_2, observer_2) = Checker::new();
        // Custom operations
        let observable = subject.clone().timeout(DURATION_LOGICAL, runtime.clone());
        let observable_1 = observable;
        let observable_2 = observable_1.clone();

        let _subscription_1 = observable_1.subscribe(observer_1);
        let _subscription_1 = observable_2.subscribe(observer_2);
        assert!(checker_1.values().is_empty());
        assert_eq!(checker_1.state(), State::Active);
        assert!(checker_2.values().is_empty());
        assert_eq!(checker_2.state(), State::Active);

        subject.on_next(111);
        assert_eq!(checker_1.values(), [111]);
        assert_eq!(checker_1.state(), State::Active);
        assert_eq!(checker_2.values(), [111]);
        assert_eq!(checker_2.state(), State::Active);

        runtime.sleep(DURATION_LOGICAL - DURATION_DEVIATION).await;
        subject.on_next(222);
        assert_eq!(checker_1.values(), [111, 222]);
        assert_eq!(checker_1.state(), State::Active);
        assert_eq!(checker_2.values(), [111, 222]);
        assert_eq!(checker_2.state(), State::Active);

        runtime.sleep(DURATION_LOGICAL + DURATION_DEVIATION).await;
        assert_eq!(checker_1.values(), [111, 222]);
        assert_eq!(checker_1.state(), State::Error(timeout::Error::Timeout));
        assert_eq!(checker_2.values(), [111, 222]);
        assert_eq!(checker_2.state(), State::Error(timeout::Error::Timeout));
    });
}

#[test]
fn test_unsub_on_next_by_take() {
    block_on(|runtime| async move {
        let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
        let (checker, observer) = Checker::new();

        // Custom operations
        let observable = observable
            .timeout(DURATION_LOGICAL, runtime.clone())
            .take(1);

        let _subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        sender.on_next(111);
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Completed);
        assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
    });
}

#[test]
fn test_multiple_operation_timeout_at_first() {
    block_on(|runtime| async move {
        let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
        let (checker, observer) = Checker::new();

        // Custom operations
        let observable = observable
            .timeout(DURATION_LOGICAL, runtime.clone())
            .timeout(DURATION_LOGICAL * 2, runtime.clone());

        let _subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        sender.on_next(111);
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        runtime.sleep(DURATION_LOGICAL - DURATION_DEVIATION).await;
        sender.on_next(222);
        assert_eq!(checker.values(), [111, 222]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        runtime.sleep(DURATION_LOGICAL + DURATION_DEVIATION).await;
        assert_eq!(checker.values(), [111, 222]);
        assert_eq!(
            checker.state(),
            State::Error(timeout::Error::SourceError(timeout::Error::Timeout))
        );
        assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
    });
}

#[test]
fn test_multiple_operation_timeout_at_sencond() {
    block_on(|runtime| async move {
        let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
        let (checker, observer) = Checker::new();

        // Custom operations
        let observable = observable
            .timeout(DURATION_LOGICAL * 2, runtime.clone())
            .timeout(DURATION_LOGICAL, runtime.clone());

        let _subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        sender.on_next(111);
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        runtime.sleep(DURATION_LOGICAL - DURATION_DEVIATION).await;
        sender.on_next(222);
        assert_eq!(checker.values(), [111, 222]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        runtime.sleep(DURATION_LOGICAL + DURATION_DEVIATION).await;
        assert_eq!(checker.values(), [111, 222]);
        assert_eq!(checker.state(), State::Error(timeout::Error::Timeout));
        assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
    });
}
#[test]
fn test_without_convenient_api() {
    block_on(|runtime| async move {
        let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
        let (checker, observer) = Checker::new();

        // Custom operations
        let observable = Timeout::new(observable, DURATION_LOGICAL, runtime.clone());

        let _subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        sender.on_next(111);
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        runtime.sleep(DURATION_LOGICAL - DURATION_DEVIATION).await;
        sender.on_next(222);
        assert_eq!(checker.values(), [111, 222]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        runtime.sleep(DURATION_LOGICAL + DURATION_DEVIATION).await;
        assert_eq!(checker.values(), [111, 222]);
        assert_eq!(checker.state(), State::Error(timeout::Error::Timeout));
        assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
    });
}

#[test]
fn test_complete_after_next() {
    block_on(|runtime| async move {
        let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
        let (checker, observer) = Checker::new();

        // Custom operations
        let observable = observable.timeout(DURATION_LOGICAL, runtime.clone());

        let _subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        sender.on_next(111);
        sender.on_termination(Termination::<Infallible>::Completed);
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Completed);
        assert_eq!(channel_checker.state(), ChannelState::Completed);
    });
}

#[test]
fn test_error_after_next() {
    block_on(|runtime| async move {
        let (mut sender, observable, channel_checker) = test_channel();
        let (checker, observer) = Checker::new();

        // Custom operations
        let observable = observable.timeout(DURATION_LOGICAL, runtime.clone());

        let _subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        sender.on_next(111);
        sender.on_termination(Termination::Error("error"));
        assert_eq!(checker.values(), [111]);
        assert_eq!(
            checker.state(),
            State::Error(timeout::Error::SourceError("error"))
        );
        assert_eq!(channel_checker.state(), ChannelState::Error("error"));
    });
}

#[test]
fn test_unsub_after_next() {
    block_on(|runtime| async move {
        let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
        let (checker, observer) = Checker::new();

        // Custom operations
        let observable = observable.timeout(DURATION_LOGICAL, runtime.clone());

        let subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        sender.on_next(111);
        subscription.dispose();
        runtime.sleep(DURATION_NEXT_LOOP).await;
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Dropped);
        assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
    });
}

#[test]
fn test_unsub_after_completed() {
    block_on(|runtime| async move {
        let (sender, observable, channel_checker) = test_channel::<'_, i32, Infallible>();
        let (checker, observer) = Checker::new();

        // Custom operations
        let observable = observable.timeout(DURATION_LOGICAL, runtime.clone());

        let subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        sender.on_termination(Termination::Completed);
        subscription.dispose();
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Completed);
        assert_eq!(channel_checker.state(), ChannelState::Completed);
    });
}

#[test]
fn test_unsub_after_error() {
    block_on(|runtime| async move {
        let (sender, observable, channel_checker) = test_channel::<'_, i32, _>();
        let (checker, observer) = Checker::new();

        // Custom operations
        let observable = observable.timeout(DURATION_LOGICAL, runtime.clone());

        let subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        sender.on_termination(Termination::Error("error"));
        subscription.dispose();
        assert!(checker.values().is_empty());
        assert_eq!(
            checker.state(),
            State::Error(timeout::Error::SourceError("error"))
        );
        assert_eq!(channel_checker.state(), ChannelState::Error("error"));
    });
}

#[test]
fn test_undisposed_schedule() {
    block_on(|runtime| async move {
        let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
        let (checker, observer) = Checker::new();

        // Custom operations
        let observable = observable.timeout(DURATION_LOGICAL, runtime.clone());

        let _subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        sender.on_next(111);
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    });
}

#[test]
fn test_scheduler_should_be_disposed_after_completed() {
    block_on(|runtime| async move {
        let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
        let (checker, observer) = Checker::new();

        // Custom operations
        let observable = observable.timeout(DURATION_LOGICAL, runtime.clone());
        assert_eq!(runtime.alive_tasks_count.load(Ordering::SeqCst), 0);

        let _subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);
        assert_eq!(runtime.alive_tasks_count.load(Ordering::SeqCst), 1);

        sender.on_next(111);
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);
        assert_eq!(runtime.alive_tasks_count.load(Ordering::SeqCst), 1);

        runtime.sleep(DURATION_LOGICAL - DURATION_DEVIATION).await;
        sender.on_next(222);
        assert_eq!(checker.values(), [111, 222]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);
        assert_eq!(runtime.alive_tasks_count.load(Ordering::SeqCst), 1);

        sender.on_termination(Termination::Completed);
        assert_eq!(checker.values(), [111, 222]);
        assert_eq!(checker.state(), State::Completed);
        assert_eq!(channel_checker.state(), ChannelState::Completed);
        assert_eq!(runtime.alive_tasks_count.load(Ordering::SeqCst), 0);
    });
}

#[test]
fn test_scheduler_should_be_disposed_after_error() {
    block_on(|runtime| async move {
        let (mut sender, observable, channel_checker) = test_channel::<'_, _, _>();
        let (checker, observer) = Checker::new();

        // Custom operations
        let observable = observable.timeout(DURATION_LOGICAL, runtime.clone());
        assert_eq!(runtime.alive_tasks_count.load(Ordering::SeqCst), 0);

        let _subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);
        assert_eq!(runtime.alive_tasks_count.load(Ordering::SeqCst), 1);

        sender.on_next(111);
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);
        assert_eq!(runtime.alive_tasks_count.load(Ordering::SeqCst), 1);

        runtime.sleep(DURATION_LOGICAL - DURATION_DEVIATION).await;
        sender.on_next(222);
        assert_eq!(checker.values(), [111, 222]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);
        assert_eq!(runtime.alive_tasks_count.load(Ordering::SeqCst), 1);

        sender.on_termination(Termination::Error("error"));
        assert_eq!(checker.values(), [111, 222]);
        assert_eq!(
            checker.state(),
            State::Error(timeout::Error::SourceError("error"))
        );
        assert_eq!(channel_checker.state(), ChannelState::Error("error"));
        assert_eq!(runtime.alive_tasks_count.load(Ordering::SeqCst), 0);
    });
}

#[test]
fn test_scheduler_should_be_disposed_after_unsub() {
    block_on(|runtime| async move {
        let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
        let (checker, observer) = Checker::new();

        // Custom operations
        let observable = observable.timeout(DURATION_LOGICAL, runtime.clone());
        assert_eq!(runtime.alive_tasks_count.load(Ordering::SeqCst), 0);

        let subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);
        assert_eq!(runtime.alive_tasks_count.load(Ordering::SeqCst), 1);

        sender.on_next(111);
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);
        assert_eq!(runtime.alive_tasks_count.load(Ordering::SeqCst), 1);

        runtime.sleep(DURATION_LOGICAL - DURATION_DEVIATION).await;
        sender.on_next(222);
        assert_eq!(checker.values(), [111, 222]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);
        assert_eq!(runtime.alive_tasks_count.load(Ordering::SeqCst), 1);

        subscription.dispose();
        assert_eq!(runtime.alive_tasks_count.load(Ordering::SeqCst), 0);

        runtime.sleep(DURATION_NEXT_LOOP).await;
        assert_eq!(checker.values(), [111, 222]);
        assert_eq!(checker.state(), State::Dropped);
        assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
        assert_eq!(runtime.alive_tasks_count.load(Ordering::SeqCst), 0);
    });
}

#[test]
fn test_order_with_continuous_next() {
    block_on(|runtime| async move {
        let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
        let (checker, observer) = Checker::new();

        // Custom operations
        let observable = observable.timeout(DURATION_LOGICAL, runtime.clone());

        let _subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        let values = (0..1000).collect::<Vec<_>>();
        for i in &values {
            sender.on_next(*i);
        }
        assert_eq!(checker.values(), values);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        runtime.sleep(DURATION_LOGICAL + DURATION_DEVIATION).await;
        assert_eq!(checker.values(), values);
        assert_eq!(checker.state(), State::Error(timeout::Error::Timeout));
        assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
    });
}

#[test]
fn test_stress_testing() {
    block_on(|runtime| async move {
        const COUNT: usize = 100000;
        let mut handles = Vec::with_capacity(COUNT);
        for i in 0..COUNT {
            let runtime_cloned = runtime.clone();
            let handle = runtime_cloned.clone().spawn(async move {
                let mut subject = PublishSubject::<'_, _, Infallible>::default();
                let (checker, observer) = Checker::new();

                // Custom operations
                let observable = subject
                    .clone()
                    .timeout(DURATION_NEXT_LOOP, runtime_cloned.clone());
                let _subscription = observable.subscribe(observer);
                runtime_cloned.sleep(DURATION_NEXT_LOOP).await;
                subject.on_next(111);
                if checker.values() == [111]
                    && checker.state() == State::Error(timeout::Error::Timeout)
                {
                    panic!("Panic at {i}");
                }
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.await.unwrap();
        }
    });
}

#[test]
fn test_clone() {
    block_on(|runtime| async move {
        let observable = Create::new(|mut observer| {
            observer.on_next(TestStruct);
            observer.on_termination(Termination::Error(TestStruct));
            Subscription::default()
        });
        let observable = observable.timeout(DURATION_LOGICAL, runtime.clone());
        _ = observable.clone(); // Make sure it's Clone when T and E are not Clone.
    });
}

#[test]
fn test_type_inference_with_subscribe() {
    block_on(|runtime| async move {
        // Custom operations
        let (_, observable, _) = test_channel::<'_, i32, Infallible>();

        let observable = observable.timeout(DURATION_LOGICAL, runtime.clone());
        let (_, observer) = Checker::new();
        observable.subscribe(observer);
    });
}

#[test]
fn test_type_inference_without_subscribe() {
    block_on(|runtime| async move {
        // Custom operations
        let (_, observable, _) = test_channel::<'_, i32, Infallible>();

        observable.timeout(DURATION_LOGICAL, runtime.clone());
    });
}
