mod tests_utils;

use crate::tests_utils::{
    DURATION_10_MS,
    checker::{Checker, State},
    test_channel::{ChannelState, test_channel, test_channels},
    test_scheduler::block_on,
    test_struct::TestStruct,
};
use rx_rust::{
    disposable::{Disposable, callback_disposal::CallbackDisposal, dispose_on_drop::DisposeOnDrop},
    observable::{Observable, ObservableExt},
    observer::{Observer, Termination},
    operators::{
        creating::{create::Create, just::Just},
        filtering::single::{Error, Single},
    },
};
use std::convert::Infallible;

#[test]
fn test_completed() {
    let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
    let (checker, observer) = Checker::new();

    let observable = observable.single();

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    assert!(sender.on_next(111).is_continue());
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    sender.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
}

#[test]
fn test_completed_empty() {
    let (sender, observable, channel_checker) = test_channel::<'_, i32, Infallible>();
    let (checker, observer) = Checker::new();

    let _subscription = observable.single().subscribe(observer);
    assert_eq!(checker.state(), State::Active);

    sender.on_termination(Termination::Completed);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Error(Error::Empty));
    assert_eq!(channel_checker.state(), ChannelState::Completed);
}

#[test]
fn test_more_than_one() {
    let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
    let (checker, observer) = Checker::new();

    let _subscription = observable.single().subscribe(observer);

    assert!(sender.on_next(111).is_continue());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    // The second item fails the stream at once, without waiting for the completion.
    assert!(sender.on_next(222).is_break());
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Error(Error::MoreThanOne));
    assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
}

#[test]
fn test_error() {
    let (mut sender, observable, channel_checker) = test_channel::<'_, _, _>();
    let (checker, observer) = Checker::new();

    let _subscription = observable.single().subscribe(observer);

    assert!(sender.on_next(111).is_continue());
    sender.on_termination(Termination::Error("error"));
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Error(Error::SourceError("error")));
    assert_eq!(channel_checker.state(), ChannelState::Error("error"));
}

#[test]
fn test_unsubscribe() {
    let (channels, observable) = test_channels::<'_, i32, Infallible>();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    let observable = observable.single();
    let observable_1 = observable;
    let observable_2 = observable_1.clone();

    let subscription_1 = observable_1.subscribe(observer_1);
    let _subscription_2 = observable_2.subscribe(observer_2);
    assert!(channels.on_next(0, 111).is_continue());
    assert!(channels.on_next(1, 222).is_continue());

    subscription_1.dispose();
    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(channels.state(0), ChannelState::Unsubscribed);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(channels.state(1), ChannelState::Subscribed);

    channels.on_termination(1, Termination::Completed);
    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(checker_2.values(), [222]);
    assert_eq!(checker_2.state(), State::Completed);
    assert_eq!(channels.state(1), ChannelState::Completed);
}

#[test]
fn test_ref() {
    let value = 111;
    let error = 222;

    let (checker, observer) = Checker::new();
    let (mut sender, observable, channel_checker) = test_channel();

    let _subscription = observable.single().subscribe(observer);
    assert!(sender.on_next(&value).is_continue());
    sender.on_termination(Termination::<&i32>::Completed);
    assert_eq!(checker.values(), [&value]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Completed);

    let (checker, observer) = Checker::<&i32, _>::new();
    let (sender, observable, channel_checker) = test_channel();
    let _subscription = observable.single().subscribe(observer);
    sender.on_termination(Termination::Error(&error));
    assert_eq!(checker.state(), State::Error(Error::SourceError(&error)));
    assert_eq!(channel_checker.state(), ChannelState::Error(&error));
}

#[test]
fn test_mut_ref() {
    let mut value = 111;
    let mut error = 222;

    let observable = Create::shared_boxed(|mut observer| {
        assert!(observer.on_next(&mut value).is_continue());
        observer.on_termination(Termination::Completed);
        DisposeOnDrop::default()
    });
    let (checker, observer) = Checker::<i32, Error<Infallible>>::new();
    let (mut on_next, on_termination) = observer.into_callbacks();
    let _subscription = observable.single().subscribe_with_callback(
        |value| {
            on_next(*value);
            *value *= 2;
        },
        on_termination,
    );
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(value, 222);

    let observable = Create::shared_boxed(|observer| {
        observer.on_termination(Termination::Error(&mut error));
        DisposeOnDrop::default()
    });
    let (checker, observer) = Checker::<Infallible, i32>::new();
    let (_, on_termination) = observer.into_callbacks();
    let _subscription = observable.single().subscribe_with_callback(
        |_: Infallible| {},
        |termination| match termination {
            Termination::Error(Error::SourceError(error)) => {
                on_termination(Termination::Error(*error));
                *error *= 2;
            }
            _ => panic!(),
        },
    );
    assert_eq!(checker.state(), State::Error(222));
    assert_eq!(error, 444);
}

#[test]
fn test_async() {
    block_on(|scheduler| async move {
        let (mut sender, observable, channel_checker) = test_channel::<'_, i32, Infallible>();
        let (checker, observer) = Checker::new();

        let observable = observable.single();

        let subscription = scheduler
            .spawn(async move { observable.subscribe(observer) })
            .await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        let _sender = scheduler
            .spawn(async move {
                assert!(sender.on_next(111).is_continue());
                sender
            })
            .await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);

        scheduler.spawn(async { subscription.dispose() }).await;
        scheduler.sleep(DURATION_10_MS).await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Dropped);
        assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
    });
}

#[test]
fn test_subscribe_by_different_observer() {
    let (channels, observable) = test_channels::<'_, i32, &str>();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    let observable = observable.single();
    let observable_1 = observable;
    let observable_2 = observable_1.clone();

    let _subscription_1 = observable_1.subscribe(observer_1);
    let (on_next, on_termination) = observer_2.into_callbacks();
    let _subscription_2 = observable_2.subscribe_with_callback(on_next, on_termination);

    for index in 0..2 {
        assert!(channels.on_next(index, 111).is_continue());
        channels.on_termination(index, Termination::Completed);
    }
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Completed);
    assert_eq!(checker_2.values(), [111]);
    assert_eq!(checker_2.state(), State::Completed);
}

#[test]
fn test_unsub_on_next_by_take() {
    let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
    let (checker, observer) = Checker::new();

    let observable = observable.single().take(1);

    let _subscription = observable.subscribe(observer);
    assert!(sender.on_next(111).is_continue());
    assert!(checker.values().is_empty());

    sender.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
}

#[test]
fn test_stop_on_next() {
    let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
    let (checker, observer) = Checker::stopping_after(1);

    let _subscription = observable.single().subscribe(observer);
    assert!(sender.on_next(111).is_continue());

    // The item is the last event of the stream, and downstream stops on it: it is not completed on
    // top of that.
    sender.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Dropped);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
}

#[test]
fn test_unsubscribe_on_next() {
    use rx_rust::thread_mode::mutable::MutableExt;
    use std::sync::{Arc, Mutex};

    let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
    let (checker, observer) = Checker::new();
    let (mut on_next, on_termination) = observer.into_callbacks();
    let subscription = Arc::new(Mutex::new(None));
    let subscription_observer = subscription.clone();
    let made = observable
        .single()
        .stop_on_disposal()
        .subscribe_with_callback(
            move |value| {
                on_next(value);
                drop(subscription_observer.take_value());
            },
            on_termination,
        );
    subscription.replace_value(Some(made));

    assert!(sender.on_next(111).is_continue());
    sender.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Dropped);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
}

#[test]
fn test_multiple_operation() {
    let (mut sender, observable, channel_checker) = test_channel::<'_, i32, Infallible>();
    let (checker, observer) = Checker::new();

    let observable = observable
        .filter(|value| value % 2 == 0)
        .single()
        .map(|value| value * 10);

    let _subscription = observable.subscribe(observer);
    for value in [1, 2, 3] {
        assert!(sender.on_next(value).is_continue());
    }
    sender.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [20]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
}

#[test]
fn test_without_convenient_api() {
    let (mut sender, observable, channel_checker) = test_channel::<'_, i32, Infallible>();
    let (checker, observer) = Checker::new();

    let observable = Single::new(observable);

    let _subscription = observable.subscribe(observer);
    assert!(sender.on_next(111).is_continue());
    sender.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [111]);
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

        let observable = observable.single();

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
        let observable = observable.single();

        let (_, mut observer) = Checker::<_, Error<Infallible>>::new();
        assert!(observer.on_next(Some(&life_marker_2)).is_continue());
        let _subscription = observable.subscribe(observer);
    }
}

#[test]
fn test_clone() {
    let observable = Create::shared_boxed(|mut observer| {
        assert!(observer.on_next(TestStruct).is_continue());
        observer.on_termination(Termination::Error(TestStruct));
        DisposeOnDrop::default()
    });
    let observable = observable.single();
    _ = observable.clone(); // Make sure it's Clone when T and E are not Clone.
}

#[test]
fn test_type_inference_with_subscribe() {
    let observable = Just::new(1).single();

    let observable = observable.single();
    let (_, observer) = Checker::new();
    let _ = observable.subscribe(observer);
}

#[test]
fn test_type_inference_without_subscribe() {
    let observable = Just::new(1).single();

    observable.single();
}
