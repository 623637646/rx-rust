mod tests_utils;

use crate::tests_utils::DURATION_10_MS;
use crate::tests_utils::checker::State;
use crate::tests_utils::test_channel::ChannelState;
use crate::tests_utils::test_channel::test_channel;
use crate::tests_utils::test_channel::test_channels;
use crate::tests_utils::test_scheduler::block_on;
use rx_rust::disposable::Disposable;
use rx_rust::disposable::callback_disposal::CallbackDisposal;
use rx_rust::disposable::dispose_on_drop::DisposeOnDrop;
use rx_rust::operators::creating::create::Create;
use rx_rust::operators::creating::from_iter::FromIter;
use rx_rust::{
    observable::{Observable, ObservableExt},
    observer::{Observer, Termination},
    operators::{
        creating::just::Just, transforming::buffer_with_count_and_skip::BufferWithCountAndSkip,
    },
};
use std::{convert::Infallible, num::NonZeroUsize};
use tests_utils::{checker::Checker, test_struct::TestStruct};

fn non_zero(value: usize) -> NonZeroUsize {
    NonZeroUsize::new(value).unwrap()
}

#[test]
fn test_completed_overlapping() {
    let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
    let (checker, observer) = Checker::new();

    let observable = observable.buffer_with_count_and_skip(non_zero(3), non_zero(1));

    let _subscription = observable.subscribe(observer);
    assert!(sender.on_next(1).is_continue());
    assert!(sender.on_next(2).is_continue());
    assert!(checker.values().is_empty());

    assert!(sender.on_next(3).is_continue());
    assert_eq!(checker.values(), [vec![1, 2, 3]]);

    assert!(sender.on_next(4).is_continue());
    assert_eq!(checker.values(), [vec![1, 2, 3], vec![2, 3, 4]]);
    assert_eq!(checker.state(), State::Active);

    // The bundles still open are emitted, oldest first.
    sender.on_termination(Termination::Completed);
    assert_eq!(
        checker.values(),
        [vec![1, 2, 3], vec![2, 3, 4], vec![3, 4], vec![4]]
    );
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
}

#[test]
fn test_completed_pairs() {
    let (checker, observer) = Checker::<_, Infallible>::new();

    let _subscription = FromIter::new([1, 2, 3, 4])
        .buffer_with_count_and_skip(non_zero(2), non_zero(1))
        .subscribe(observer);
    assert_eq!(
        checker.values(),
        [vec![1, 2], vec![2, 3], vec![3, 4], vec![4]]
    );
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_completed_with_gaps() {
    let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
    let (checker, observer) = Checker::new();

    let _subscription = observable
        .buffer_with_count_and_skip(non_zero(2), non_zero(3))
        .subscribe(observer);
    for value in 1..=7 {
        assert!(sender.on_next(value).is_continue());
    }
    // The bundles start at 1, 4 and 7; 3 and 6 fall in no bundle.
    assert_eq!(checker.values(), [vec![1, 2], vec![4, 5]]);

    sender.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [vec![1, 2], vec![4, 5], vec![7]]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
}

#[test]
fn test_completed_skip_equals_count() {
    let (checker, observer) = Checker::<_, Infallible>::new();

    let _subscription = FromIter::new([1, 2, 3, 4, 5])
        .buffer_with_count_and_skip(non_zero(2), non_zero(2))
        .subscribe(observer);
    assert_eq!(checker.values(), [vec![1, 2], vec![3, 4], vec![5]]);
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_completed_last_empty() {
    let (mut sender, observable, _channel_checker) = test_channel::<'_, _, Infallible>();
    let (checker, observer) = Checker::new();

    let _subscription = observable
        .buffer_with_count_and_skip(non_zero(1), non_zero(2))
        .subscribe(observer);
    assert!(sender.on_next(1).is_continue());
    assert!(sender.on_next(2).is_continue());
    // No bundle is open: nothing more is emitted.
    sender.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [vec![1]]);
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_error() {
    let (mut sender, observable, channel_checker) = test_channel::<'_, _, _>();
    let (checker, observer) = Checker::new();

    let _subscription = observable
        .buffer_with_count_and_skip(non_zero(2), non_zero(1))
        .subscribe(observer);
    assert!(sender.on_next(1).is_continue());
    assert!(sender.on_next(2).is_continue());
    assert!(sender.on_next(3).is_continue());

    // An error drops the bundles still open.
    sender.on_termination(Termination::Error("error"));
    assert_eq!(checker.values(), [vec![1, 2], vec![2, 3]]);
    assert_eq!(checker.state(), State::Error("error"));
    assert_eq!(channel_checker.state(), ChannelState::Error("error"));
}

#[test]
fn test_unsubscribe() {
    let (channels, observable) = test_channels::<'_, i32, Infallible>();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    let observable = observable.buffer_with_count_and_skip(non_zero(2), non_zero(1));
    let observable_1 = observable;
    let observable_2 = observable_1.clone();

    let subscription_1 = observable_1.subscribe(observer_1);
    let _subscription_2 = observable_2.subscribe(observer_2);
    for index in 0..2 {
        assert!(channels.on_next(index, 1).is_continue());
        assert!(channels.on_next(index, 2).is_continue());
    }

    subscription_1.dispose();
    assert_eq!(checker_1.values(), [vec![1, 2]]);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(channels.state(0), ChannelState::Unsubscribed);

    channels.on_termination(1, Termination::Completed);
    assert_eq!(checker_2.values(), [vec![1, 2], vec![2]]);
    assert_eq!(checker_2.state(), State::Completed);
}

#[test]
fn test_ref() {
    let values = [111, 222];
    let error = 333;

    let (checker, observer) = Checker::new();
    let (mut sender, observable, channel_checker) = test_channel();

    let _subscription = observable
        .buffer_with_count_and_skip(non_zero(2), non_zero(1))
        .subscribe(observer);
    assert!(sender.on_next(&values[0]).is_continue());
    assert!(sender.on_next(&values[1]).is_continue());
    sender.on_termination(Termination::Error(&error));
    assert_eq!(checker.values(), [vec![&values[0], &values[1]]]);
    assert_eq!(checker.state(), State::Error(&error));
    assert_eq!(channel_checker.state(), ChannelState::Error(&error));
}

#[test]
fn test_mut_ref() {
    let mut error = 222;

    let observable = Create::shared_boxed(|mut observer| {
        assert!(observer.on_next(1).is_continue());
        assert!(observer.on_next(2).is_continue());
        observer.on_termination(Termination::Error(&mut error));
        DisposeOnDrop::default()
    });
    let (checker, observer) = Checker::new();

    let (on_next, on_termination) = observer.into_callbacks();
    let _subscription = observable
        .buffer_with_count_and_skip(non_zero(2), non_zero(1))
        .subscribe_with_callback(on_next, |termination| match termination {
            Termination::Completed => panic!(),
            Termination::Error(error) => {
                on_termination(Termination::Error(*error));
                *error *= 2;
            }
        });

    assert_eq!(checker.values(), [vec![1, 2]]);
    assert_eq!(checker.state(), State::Error(222));
    assert_eq!(error, 444);
}

#[test]
fn test_async() {
    block_on(|scheduler| async move {
        let (mut sender, observable, channel_checker) = test_channel::<'_, i32, Infallible>();
        let (checker, observer) = Checker::new();

        let observable = observable.buffer_with_count_and_skip(non_zero(2), non_zero(1));

        let subscription = scheduler
            .spawn(async move { observable.subscribe(observer) })
            .await;

        let _sender = scheduler
            .spawn(async move {
                assert!(sender.on_next(1).is_continue());
                assert!(sender.on_next(2).is_continue());
                sender
            })
            .await;
        assert_eq!(checker.values(), [vec![1, 2]]);
        assert_eq!(checker.state(), State::Active);

        scheduler.spawn(async { subscription.dispose() }).await;
        scheduler.sleep(DURATION_10_MS).await;
        assert_eq!(checker.values(), [vec![1, 2]]);
        assert_eq!(checker.state(), State::Dropped);
        assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
    });
}

#[test]
fn test_subscribe_by_different_observer() {
    let (channels, observable) = test_channels::<'_, i32, &str>();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    let observable = observable.buffer_with_count_and_skip(non_zero(2), non_zero(1));
    let observable_1 = observable;
    let observable_2 = observable_1.clone();

    let _subscription_1 = observable_1.subscribe(observer_1);
    let (on_next, on_termination) = observer_2.into_callbacks();
    let _subscription_2 = observable_2.subscribe_with_callback(on_next, on_termination);

    for index in 0..2 {
        assert!(channels.on_next(index, 1).is_continue());
        assert!(channels.on_next(index, 2).is_continue());
        channels.on_termination(index, Termination::Error("error"));
    }
    assert_eq!(checker_1.values(), [vec![1, 2]]);
    assert_eq!(checker_1.state(), State::Error("error"));
    assert_eq!(checker_2.values(), [vec![1, 2]]);
    assert_eq!(checker_2.state(), State::Error("error"));
}

#[test]
fn test_unsub_on_next_by_take() {
    let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
    let (checker, observer) = Checker::new();

    let observable = observable
        .buffer_with_count_and_skip(non_zero(2), non_zero(1))
        .take(1);

    let _subscription = observable.subscribe(observer);
    assert!(sender.on_next(1).is_continue());
    assert!(sender.on_next(2).is_stop());
    assert_eq!(checker.values(), [vec![1, 2]]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
}

#[test]
fn test_stop_on_next() {
    let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
    let (checker, observer) = Checker::stopping_after(2);

    let _subscription = observable
        .buffer_with_count_and_skip(non_zero(3), non_zero(1))
        .subscribe(observer);
    for value in 1..=3 {
        assert!(sender.on_next(value).is_continue());
    }
    // The bundles still open are emitted on completion, and downstream stops on the second one:
    // the third is not emitted, nor is the completion.
    sender.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [vec![1, 2, 3], vec![2, 3]]);
    assert_eq!(checker.state(), State::Dropped);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
}

#[test]
fn test_unsubscribe_on_next_during_completion() {
    use rx_rust::thread_mode::mutable::MutableExt;
    use std::sync::{Arc, Mutex};

    for stop_after in 1..=2 {
        let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
        let (checker, observer) = Checker::new();
        let (mut on_next, on_termination) = observer.into_callbacks();
        let subscription = Arc::new(Mutex::new(None));
        let subscription_observer = subscription.clone();
        let mut seen = 0;
        let made = observable
            .buffer_with_count_and_skip(non_zero(3), non_zero(1))
            .stop_on_disposal()
            .subscribe_with_callback(
                move |bundle| {
                    on_next(bundle);
                    seen += 1;
                    if seen == stop_after {
                        drop(subscription_observer.take_value());
                    }
                },
                on_termination,
            );
        subscription.replace_value(Some(made));

        assert!(sender.on_next(1).is_continue());
        assert!(sender.on_next(2).is_continue());
        // With `stop_on_disposal` the disposal stops the remaining bundles and the completion,
        // even though the callback returns `()` rather than `Flow::Stop`.
        sender.on_termination(Termination::Completed);
        assert_eq!(checker.values(), [vec![1, 2], vec![2]][..stop_after]);
        assert_eq!(checker.state(), State::Dropped);
        assert_eq!(channel_checker.state(), ChannelState::Completed);
    }
}

#[test]
fn test_multiple_operation() {
    let (checker, observer) = Checker::<_, Infallible>::new();

    let _subscription = FromIter::new([1, 2, 3, 4])
        .buffer_with_count_and_skip(non_zero(2), non_zero(1))
        .map(|bundle| bundle.iter().sum::<i32>())
        .buffer_with_count_and_skip(non_zero(2), non_zero(2))
        .subscribe(observer);
    assert_eq!(checker.values(), [vec![3, 5], vec![7, 4]]);
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_without_convenient_api() {
    let (mut sender, observable, channel_checker) = test_channel::<'_, i32, Infallible>();
    let (checker, observer) = Checker::new();

    let observable = BufferWithCountAndSkip::new(observable, non_zero(2), non_zero(1));

    let _subscription = observable.subscribe(observer);
    assert!(sender.on_next(1).is_continue());
    assert!(sender.on_next(2).is_continue());
    sender.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [vec![1, 2], vec![2]]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
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
        let observable = Create::shared_boxed(|mut observer| {
            assert!(observer.on_next(1).is_continue());
            observer.on_termination(Termination::<String>::Completed);
            DisposeOnDrop::new(CallbackDisposal::new(|| {
                life_marker.consume_ref();
            }))
        });

        let observable = observable.buffer_with_count_and_skip(non_zero(2), non_zero(1));

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
        let observable = Create::shared_boxed(|observer| {
            life_marker_1 = Some(observer);
            DisposeOnDrop::default()
        });
        let observable = observable.buffer_with_count_and_skip(non_zero(2), non_zero(1));

        let (_, mut observer) = Checker::<_, Infallible>::new();
        assert!(observer.on_next(vec![Some(&life_marker_2)]).is_continue());
        let _subscription = observable.subscribe(observer);
    }
}

#[test]
fn test_clone() {
    let observable = Create::shared_boxed(|mut observer| {
        assert!(observer.on_next(1).is_continue());
        observer.on_termination(Termination::Error(TestStruct));
        DisposeOnDrop::default()
    });
    let observable = observable.buffer_with_count_and_skip(non_zero(2), non_zero(1));
    _ = observable.clone(); // Make sure it's Clone when E is not Clone.
}

#[test]
fn test_type_inference_with_subscribe() {
    let observable = Just::new(1).buffer_with_count_and_skip(non_zero(2), non_zero(1));

    let observable = observable.buffer_with_count_and_skip(non_zero(2), non_zero(1));
    let (_, observer) = Checker::new();
    let _ = observable.subscribe(observer);
}

#[test]
fn test_type_inference_without_subscribe() {
    let observable = Just::new(1).buffer_with_count_and_skip(non_zero(2), non_zero(1));

    observable.buffer_with_count_and_skip(non_zero(2), non_zero(1));
}
