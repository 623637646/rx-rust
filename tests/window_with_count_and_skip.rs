mod tests_utils;

use crate::tests_utils::checker::State;
use crate::tests_utils::test_channel::ChannelState;
use crate::tests_utils::test_channel::test_channels;
use crate::tests_utils::test_scheduler::block_on;
use crate::tests_utils::window_checker::WindowChecker;
use rx_rust::disposable::Disposable;
use rx_rust::disposable::callback_disposal::CallbackDisposal;
use rx_rust::disposable::dispose_on_drop::DisposeOnDrop;
use rx_rust::observer::boxed_observer::SendBoxedObserver;
use rx_rust::operators::creating::create::Create;
use rx_rust::operators::creating::empty::Empty;
use rx_rust::operators::creating::from_iter::FromIter;
use rx_rust::operators::creating::throw::Throw;
use rx_rust::{
    observable::{Observable, ObservableExt},
    observer::{Observer, Termination},
    operators::transforming::window_with_count_and_skip::WindowWithCountAndSkip,
    subject::unicast_subject,
};
use std::{convert::Infallible, num::NonZeroUsize};
use tests_utils::{checker::Checker, test_channel::test_channel, test_struct::TestStruct};

fn non_zero(value: usize) -> NonZeroUsize {
    NonZeroUsize::new(value).unwrap()
}

#[test]
fn test_completed_overlapping() {
    let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
    let (checker, observer) = WindowChecker::new();

    let observable = observable.window_with_count_and_skip(non_zero(3), non_zero(1));

    let _subscription = observable.subscribe(observer);
    assert_eq!(checker.windows(), [(vec![], State::Active)]);

    assert!(sender.on_next(1).is_continue());
    assert_eq!(
        checker.windows(),
        [(vec![1], State::Active), (vec![], State::Active)]
    );

    assert!(sender.on_next(2).is_continue());
    assert!(sender.on_next(3).is_continue());
    // The first window is full; every item opened another one.
    assert_eq!(
        checker.windows(),
        [
            (vec![1, 2, 3], State::Completed),
            (vec![2, 3], State::Active),
            (vec![3], State::Active),
            (vec![], State::Active),
        ]
    );
    assert_eq!(checker.state(), State::Active);

    sender.on_termination(Termination::Completed);
    assert_eq!(
        checker.windows(),
        [
            (vec![1, 2, 3], State::Completed),
            (vec![2, 3], State::Completed),
            (vec![3], State::Completed),
            (vec![], State::Completed),
        ]
    );
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
}

#[test]
fn test_completed_with_gaps() {
    let (mut sender, observable, _channel_checker) = test_channel::<'_, _, Infallible>();
    let (checker, observer) = WindowChecker::new();

    let _subscription = observable
        .window_with_count_and_skip(non_zero(2), non_zero(3))
        .subscribe(observer);
    for value in 1..=5 {
        assert!(sender.on_next(value).is_continue());
    }
    // The windows start at 1 and 4; 3 falls in no window.
    assert_eq!(
        checker.windows(),
        [
            (vec![1, 2], State::Completed),
            (vec![4, 5], State::Completed)
        ]
    );

    assert!(sender.on_next(6).is_continue());
    assert_eq!(checker.windows().len(), 3);
    assert_eq!(checker.windows()[2], (vec![], State::Active));
    sender.on_termination(Termination::Completed);
    assert_eq!(checker.windows()[2], (vec![], State::Completed));
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_completed_skip_equals_count() {
    let (checker, observer) = WindowChecker::<_, Infallible>::new();

    let _subscription = FromIter::new([1, 2, 3])
        .into_shared()
        .window_with_count_and_skip(non_zero(2), non_zero(2))
        .subscribe(observer);
    assert_eq!(
        checker.windows(),
        [(vec![1, 2], State::Completed), (vec![3], State::Completed)]
    );
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_error() {
    let (mut sender, observable, channel_checker) = test_channel();
    let (checker, observer) = WindowChecker::new();

    let _subscription = observable
        .window_with_count_and_skip(non_zero(2), non_zero(1))
        .subscribe(observer);
    assert!(sender.on_next(1).is_continue());

    // Every window still open fails, then the outer stream.
    sender.on_termination(Termination::Error("error"));
    assert_eq!(
        checker.windows(),
        [
            (vec![1], State::Error("error")),
            (vec![], State::Error("error"))
        ]
    );
    assert_eq!(checker.state(), State::Error("error"));
    assert_eq!(channel_checker.state(), ChannelState::Error("error"));
}

#[test]
fn test_unsubscribe() {
    let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
    let (checker, observer) = WindowChecker::new();

    let subscription = observable
        .window_with_count_and_skip(non_zero(2), non_zero(1))
        .subscribe(observer);
    assert!(sender.on_next(1).is_continue());

    // Disposing the outer subscription drops the sending end of every open window.
    subscription.dispose();
    assert_eq!(
        checker.windows(),
        [(vec![1], State::Dropped), (vec![], State::Dropped)]
    );
    assert_eq!(checker.state(), State::Dropped);
    assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
}

#[test]
fn test_ref() {
    let values = [111, 222];
    let error = 333;

    let (mut sender, observable, channel_checker) = test_channel();
    let (checker, observer) = WindowChecker::new();

    let _subscription = observable
        .window_with_count_and_skip(non_zero(2), non_zero(1))
        .subscribe(observer);
    assert!(sender.on_next(&values[0]).is_continue());
    assert!(sender.on_next(&values[1]).is_continue());
    sender.on_termination(Termination::Error(&error));
    assert_eq!(
        checker.windows(),
        [
            (vec![&values[0], &values[1]], State::Completed),
            (vec![&values[1]], State::Error(&error)),
            (vec![], State::Error(&error)),
        ]
    );
    assert_eq!(checker.state(), State::Error(&error));
    assert_eq!(channel_checker.state(), ChannelState::Error(&error));
}

#[test]
fn test_async() {
    block_on(|scheduler| async move {
        let (mut sender, observable, channel_checker) = test_channel::<'_, i32, Infallible>();
        let (checker, observer) = WindowChecker::new();

        let observable = observable.window_with_count_and_skip(non_zero(2), non_zero(1));

        let subscription = scheduler
            .spawn(async move { observable.subscribe(observer) })
            .await;

        let _sender = scheduler
            .spawn(async move {
                assert!(sender.on_next(1).is_continue());
                sender
            })
            .await;
        assert_eq!(
            checker.windows(),
            [(vec![1], State::Active), (vec![], State::Active)]
        );

        scheduler.spawn(async { subscription.dispose() }).await;
        assert_eq!(checker.state(), State::Dropped);
        assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
    });
}

#[test]
fn test_subscribe_by_different_observer() {
    let (channels, observable) = test_channels::<'_, i32, &str>();
    let (checker_1, observer_1) = WindowChecker::new();
    let (checker_2, observer_2) = WindowChecker::new();

    let observable = observable.window_with_count_and_skip(non_zero(2), non_zero(2));
    let observable_1 = observable;
    let observable_2 = observable_1.clone();

    let _subscription_1 = observable_1.subscribe(observer_1);
    let _subscription_2 = observable_2.subscribe(observer_2);

    for index in 0..2 {
        assert!(channels.on_next(index, 1).is_continue());
        channels.on_termination(index, Termination::Error("error"));
    }
    for checker in [checker_1, checker_2] {
        assert_eq!(checker.windows(), [(vec![1], State::Error("error"))]);
        assert_eq!(checker.state(), State::Error("error"));
    }
}

#[test]
fn test_unsub_on_next_by_take() {
    let (mut sender, observable, channel_checker) = test_channel::<'_, i32, Infallible>();
    let (checker, observer) = WindowChecker::new();

    let observable = observable
        .window_with_count_and_skip(non_zero(3), non_zero(1))
        .take(2);

    let _subscription = observable.subscribe(observer);
    // The second window ends the stream, which drops the sending end of both.
    assert!(sender.on_next(1).is_break());
    assert_eq!(
        checker.windows(),
        [(vec![1], State::Dropped), (vec![], State::Dropped)]
    );
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
}

#[test]
fn test_stop_on_next() {
    let (_sender, observable, channel_checker) = test_channel::<'_, i32, Infallible>();
    let (checker, observer) = WindowChecker::stopping_after(1);

    // The first window stops the stream before the source is subscribed to.
    let _subscription = observable
        .window_with_count_and_skip(non_zero(2), non_zero(1))
        .subscribe(observer);
    assert_eq!(checker.windows(), [(vec![], State::Dropped)]);
    assert_eq!(checker.state(), State::Dropped);
    assert_eq!(channel_checker.state(), ChannelState::Initialized);
}

/// The consumer of a window disposes the subscription of the windows: no further window is
/// emitted, while the windows already handed out, subscriptions of their own, still get the value
/// being sent.
#[test]
fn test_unsubscribe_on_inner_next() {
    use rx_rust::thread_mode::mutable::MutableExt;
    use std::sync::{Arc, Mutex};

    let (mut sender, observable, channel_checker) = test_channel::<'_, i32, Infallible>();
    let (checker, observer) = WindowChecker::new();
    let subscription = Arc::new(Mutex::new(None));
    let subscription_observer = subscription.clone();
    let made = observable
        .window_with_count_and_skip(non_zero(3), non_zero(1))
        .map(move |window| {
            let subscription = subscription_observer.clone();
            window.do_before_next(move |value| {
                if *value == 2 {
                    drop(subscription.take_value());
                }
            })
        })
        .stop_on_disposal()
        .subscribe(observer);
    subscription.replace_value(Some(made));

    assert!(sender.on_next(1).is_continue());
    assert!(sender.on_next(2).is_break());
    assert_eq!(
        checker.windows(),
        [(vec![1, 2], State::Dropped), (vec![2], State::Dropped)]
    );
    assert_eq!(checker.state(), State::Dropped);
    assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
}

/// The same on the termination of a window: the other windows are terminated too, and the
/// observer of the windows is not.
#[test]
fn test_unsubscribe_on_inner_termination() {
    use rx_rust::thread_mode::mutable::MutableExt;
    use std::sync::{Arc, Mutex};

    for termination in [Termination::Completed, Termination::Error("error")] {
        let (mut sender, observable, channel_checker) = test_channel();
        let (checker, observer) = WindowChecker::new();
        let subscription = Arc::new(Mutex::new(None));
        let subscription_observer = subscription.clone();
        let made = observable
            .window_with_count_and_skip(non_zero(3), non_zero(1))
            .map(move |window| {
                let subscription = subscription_observer.clone();
                window.do_before_termination(move |_| drop(subscription.take_value()))
            })
            .stop_on_disposal()
            .subscribe(observer);
        subscription.replace_value(Some(made));

        assert!(sender.on_next(1).is_continue());
        sender.on_termination(termination.clone());
        let (window_state, channel_state) = match termination {
            Termination::Completed => (State::Completed, ChannelState::Completed),
            Termination::Error(error) => (State::Error(error), ChannelState::Error(error)),
        };
        assert_eq!(
            checker.windows(),
            [(vec![1], window_state.clone()), (vec![], window_state)]
        );
        assert_eq!(checker.state(), State::Dropped);
        assert_eq!(channel_checker.state(), channel_state);
    }
}

/// The same on the completion of a full window: the window that would open next is not emitted.
#[test]
fn test_unsubscribe_on_full_window() {
    use rx_rust::thread_mode::mutable::MutableExt;
    use std::sync::{Arc, Mutex};

    let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
    let (checker, observer) = WindowChecker::new();
    let subscription = Arc::new(Mutex::new(None));
    let subscription_observer = subscription.clone();
    let made = observable
        .window_with_count_and_skip(non_zero(1), non_zero(1))
        .map(move |window| {
            let subscription = subscription_observer.clone();
            window.do_before_termination(move |_| drop(subscription.take_value()))
        })
        .stop_on_disposal()
        .subscribe(observer);
    subscription.replace_value(Some(made));

    assert!(sender.on_next(1).is_break());
    assert_eq!(checker.windows(), [(vec![1], State::Completed)]);
    assert_eq!(checker.state(), State::Dropped);
    assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
}

#[test]
fn test_multiple_operation() {
    let (checker, observer) = Checker::<_, Infallible>::new();

    let _subscription = FromIter::new([1, 2, 3])
        .window_with_count_and_skip(non_zero(2), non_zero(1))
        .concat_map(|window| window.to_vec())
        .subscribe(observer);
    assert_eq!(checker.values(), [vec![1, 2], vec![2, 3], vec![3], vec![]]);
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_without_convenient_api() {
    let (mut sender, observable, channel_checker) = test_channel::<'_, i32, Infallible>();
    let (checker, observer) = WindowChecker::new();

    let observable = WindowWithCountAndSkip::new(observable, non_zero(1), non_zero(2));

    let _subscription = observable.subscribe(observer);
    assert!(sender.on_next(1).is_continue());
    assert!(sender.on_next(2).is_continue());
    sender.on_termination(Termination::Completed);
    assert_eq!(
        checker.windows(),
        [(vec![1], State::Completed), (vec![], State::Completed)]
    );
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
}

#[test]
fn test_complete_on_sub() {
    let (checker, observer) = WindowChecker::<Infallible, Infallible>::new();

    let _subscription = Empty
        .into_shared()
        .window_with_count_and_skip(non_zero(2), non_zero(1))
        .subscribe(observer);
    assert_eq!(checker.windows(), [(vec![], State::Completed)]);
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_error_on_sub() {
    let (checker, observer) = WindowChecker::<i32, _>::new();

    let _subscription = Throw::new("error")
        .with_item_type()
        .into_shared()
        .window_with_count_and_skip(non_zero(2), non_zero(1))
        .subscribe(observer);
    assert_eq!(checker.windows(), [(vec![], State::Error("error"))]);
    assert_eq!(checker.state(), State::Error("error"));
}

#[test]
fn test_next_on_unsub() {
    let (checker, observer) = WindowChecker::new();

    // The source emits from inside its own disposal: the operator has no context, so the item
    // still reaches the windows open then.
    let observable = Create::shared_boxed(|observer: SendBoxedObserver<'_, i32, Infallible>| {
        DisposeOnDrop::new(CallbackDisposal::new(move || {
            let mut observer = observer;
            let _ = observer.on_next(111);
        }))
    });

    let subscription = observable
        .window_with_count_and_skip(non_zero(2), non_zero(1))
        .subscribe(observer);
    subscription.dispose();
    assert_eq!(checker.windows()[0].0, [111]);
    assert_eq!(checker.state(), State::Dropped);
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

        let observable = observable.window_with_count_and_skip(non_zero(2), non_zero(1));

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
        let observable = observable.window_with_count_and_skip(non_zero(2), non_zero(1));

        let (_, mut observer) = Checker::<_, Infallible>::new();
        let (mut sender, window) = unicast_subject::new_boxed();
        assert!(sender.on_next(&life_marker_2).is_continue());
        assert!(observer.on_next(window).is_continue());
        let _subscription = observable.subscribe(observer);
    }
}

#[test]
fn test_clone() {
    let observable = Create::shared_boxed(|mut observer| {
        assert!(observer.on_next(1).is_continue());
        observer.on_termination(Termination::Error("error"));
        DisposeOnDrop::default()
    });
    let observable = observable.window_with_count_and_skip(non_zero(2), non_zero(1));
    _ = observable.clone();
}

#[test]
fn test_type_inference_with_subscribe() {
    let observable = FromIter::new([1]).window_with_count_and_skip(non_zero(2), non_zero(1));

    let observable = observable.map(|window| window.to_vec()).merge_all();
    let (_, observer) = Checker::<_, Infallible>::new();
    let _ = observable.subscribe(observer);
}

#[test]
fn test_type_inference_without_subscribe() {
    let observable = FromIter::new([1]).window_with_count_and_skip(non_zero(2), non_zero(1));

    observable.map(|window| window.to_vec());
}
