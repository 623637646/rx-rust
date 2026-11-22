mod tests_utils;

use crate::tests_utils::DURATION_10_MS;
use crate::tests_utils::checker::State;
use crate::tests_utils::test_channel::ChannelState;
use crate::tests_utils::test_channel::test_channels;
use crate::tests_utils::test_scheduler::block_on;
use rx_rust::disposable::Disposable;
use rx_rust::disposable::callback_disposal::CallbackDisposal;
use rx_rust::disposable::dispose_on_drop::DisposeOnDrop;
use rx_rust::observer::boxed_observer::SendBoxedObserver;
use rx_rust::operators::creating::create::Create;
use rx_rust::operators::creating::empty::Empty;
use rx_rust::subject::publish_subject::PublishSubject;
use rx_rust::{
    observable::{Observable, ObservableExt},
    observer::{Observer, Termination},
    operators::{
        creating::{just::Just, throw::Throw},
        transforming::concat_eager_map::ConcatEagerMap,
    },
};
use std::convert::Infallible;
use std::num::NonZeroUsize;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use tests_utils::{checker::Checker, test_channel::test_channel, test_struct::TestStruct};

#[test]
fn test_completed_inner_finish() {
    let (mut sender, observable, channel_checker) = test_channel();
    let (inner_channels, inner) = test_channels::<'_, i32, Infallible>();
    let (checker, observer) = Checker::new();
    let calls = Arc::new(AtomicUsize::new(0));
    let calls_callback = calls.clone();

    let observable = observable.concat_eager_map(move |value: i32| {
        calls_callback.fetch_add(1, Ordering::SeqCst);
        inner.clone().map(move |inner_value| inner_value + value)
    });

    let _subscription = observable.subscribe(observer);
    assert!(sender.on_next(100).is_continue());
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert_eq!(inner_channels.state(0), ChannelState::Subscribed);
    assert!(inner_channels.on_next(0, 1).is_continue());
    assert_eq!(checker.values(), [101]);

    // The observable of each value is subscribed as it is made, while the first one runs.
    assert!(sender.on_next(200).is_continue());
    assert!(sender.on_next(300).is_continue());
    assert_eq!(calls.load(Ordering::SeqCst), 3);
    assert_eq!(inner_channels.len(), 3);

    // What they send is kept until their turn, each in its own order.
    assert!(inner_channels.on_next(2, 3).is_continue());
    assert!(inner_channels.on_next(1, 2).is_continue());
    assert_eq!(checker.values(), [101]);

    sender.on_termination(Termination::Completed);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Completed);

    assert!(inner_channels.on_next(0, 2).is_continue());
    inner_channels.on_termination(0, Termination::Completed);
    assert_eq!(checker.values(), [101, 102, 202]);
    assert_eq!(checker.state(), State::Active);

    inner_channels.on_termination(1, Termination::Completed);
    assert_eq!(checker.values(), [101, 102, 202, 303]);
    assert_eq!(checker.state(), State::Active);

    inner_channels.on_termination(2, Termination::Completed);
    assert_eq!(checker.values(), [101, 102, 202, 303]);
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_completed_outer_finish() {
    let (mut sender, observable, channel_checker) = test_channel();
    let (inner_channels, inner) = test_channels::<'_, i32, Infallible>();
    let (checker, observer) = Checker::new();

    let _subscription = observable
        .concat_eager_map(move |value: i32| {
            inner.clone().map(move |inner_value| inner_value + value)
        })
        .subscribe(observer);

    assert!(sender.on_next(100).is_continue());
    assert!(inner_channels.on_next(0, 1).is_continue());
    inner_channels.on_termination(0, Termination::Completed);

    // The first one is over: the next value starts the next one.
    assert!(sender.on_next(200).is_continue());
    assert_eq!(inner_channels.state(1), ChannelState::Subscribed);
    assert!(inner_channels.on_next(1, 2).is_continue());
    inner_channels.on_termination(1, Termination::Completed);
    assert_eq!(checker.values(), [101, 202]);
    assert_eq!(checker.state(), State::Active);

    sender.on_termination(Termination::Completed);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
}

#[test]
fn test_completed_inner_completed_fast() {
    let (mut sender, observable, _channel_checker) = test_channel::<'_, _, Infallible>();
    let (checker, observer) = Checker::new();

    let _subscription = observable
        .concat_eager_map(|value| Just::new(value * 10))
        .subscribe(observer);
    assert!(sender.on_next(1).is_continue());
    assert!(sender.on_next(2).is_continue());
    sender.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [10, 20]);
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_completed_empty() {
    let (sender, observable, channel_checker) = test_channel::<'_, i32, Infallible>();
    let (checker, observer) = Checker::new();

    let _subscription = observable.concat_eager_map(Just::new).subscribe(observer);
    sender.on_termination(Termination::Completed);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
}

#[test]
fn test_error_inner_finish() {
    let (mut sender, observable, channel_checker) = test_channel();
    let (mut sender_1, observable_1, channel_checker_1) = test_channel::<'_, i32, _>();
    let mut observable_1 = Some(observable_1);
    let (checker, observer) = Checker::new();

    let _subscription = observable
        .concat_eager_map(move |_: i32| observable_1.take().unwrap())
        .subscribe(observer);
    assert!(sender.on_next(1).is_continue());
    assert!(sender_1.on_next(111).is_continue());
    sender_1.on_termination(Termination::Error("error"));
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Error("error"));
    assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Error("error"));
}

#[test]
fn test_error_outer_finish() {
    let (mut sender, observable, channel_checker) = test_channel();
    let (mut sender_1, observable_1, channel_checker_1) = test_channel::<'_, i32, _>();
    let mut observable_1 = Some(observable_1);
    let (checker, observer) = Checker::new();

    let _subscription = observable
        .concat_eager_map(move |_: i32| observable_1.take().unwrap())
        .subscribe(observer);
    assert!(sender.on_next(1).is_continue());
    assert!(sender_1.on_next(111).is_continue());
    sender.on_termination(Termination::Error("error"));
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Error("error"));
    assert_eq!(channel_checker.state(), ChannelState::Error("error"));
    assert_eq!(channel_checker_1.state(), ChannelState::Unsubscribed);
}

#[test]
fn test_unsubscribe() {
    let (mut sender, observable, channel_checker) = test_channel();
    let (mut sender_1, observable_1, channel_checker_1) = test_channel::<'_, i32, Infallible>();
    let mut observable_1 = Some(observable_1);
    let (checker, observer) = Checker::new();

    let subscription = observable
        .concat_eager_map(move |_: i32| observable_1.take().unwrap())
        .subscribe(observer);
    assert!(sender.on_next(1).is_continue());
    assert!(sender_1.on_next(111).is_continue());

    subscription.dispose();
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Dropped);
    assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Unsubscribed);
}

#[test]
fn test_unsubscribe_with_publish_subject() {
    let mut source = PublishSubject::<_, Infallible, _>::shared();
    let mut inner = PublishSubject::<i32, Infallible, _>::shared();
    let (checker, observer) = Checker::new();

    let inner_source = inner.clone();
    let subscription = source
        .clone()
        .concat_eager_map(move |_: ()| inner_source.clone())
        .subscribe(observer);
    assert!(source.on_next(()).is_continue());
    assert!(inner.on_next(111).is_continue());
    assert_eq!(checker.values(), [111]);

    subscription.dispose();
    assert!(inner.on_next(222).is_continue());
    assert!(source.on_next(()).is_continue());
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Dropped);
}

#[test]
fn test_ref() {
    let value = 111;
    let error = 222;

    let (mut sender, observable, _channel_checker) = test_channel();
    let (checker, observer) = Checker::new();

    let _subscription = observable
        .concat_eager_map(|value| Just::new(value).with_error_type())
        .subscribe(observer);
    assert!(sender.on_next(&value).is_continue());
    sender.on_termination(Termination::Error(&error));
    assert_eq!(checker.values(), [&value]);
    assert_eq!(checker.state(), State::Error(&error));
}

#[test]
fn test_mut_ref() {
    let mut value = 111;

    let observable = Create::shared_boxed(|mut observer| {
        assert!(observer.on_next(&mut value).is_continue());
        observer.on_termination(Termination::<Infallible>::Completed);
        DisposeOnDrop::default()
    });

    let subscription = observable
        .concat_eager_map(|value| Just::new(value).with_error_type())
        .subscribe_with_callback(|value| *value *= 2, |_| {});
    subscription.dispose();
    assert_eq!(value, 222);
}

#[test]
fn test_async() {
    block_on(|scheduler| async move {
        let (mut sender, observable, channel_checker) = test_channel::<'_, i32, Infallible>();
        let (mut sender_1, observable_1, channel_checker_1) = test_channel::<'_, i32, _>();
        let mut observable_1 = Some(observable_1);
        let (checker, observer) = Checker::new();

        let observable = observable.concat_eager_map(move |_| observable_1.take().unwrap());

        let subscription = scheduler
            .spawn(async move { observable.subscribe(observer) })
            .await;

        let _senders = scheduler
            .spawn(async move {
                assert!(sender.on_next(1).is_continue());
                assert!(sender_1.on_next(111).is_continue());
                (sender, sender_1)
            })
            .await;
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Active);

        scheduler.spawn(async { subscription.dispose() }).await;
        scheduler.sleep(DURATION_10_MS).await;
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Dropped);
        assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
        assert_eq!(channel_checker_1.state(), ChannelState::Unsubscribed);
    });
}

#[test]
fn test_subscribe_by_different_observer() {
    let (channels, observable) = test_channels::<'_, i32, &str>();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    let observable = observable.concat_eager_map(|value| Just::new(value).with_error_type());
    let observable_1 = observable;
    let observable_2 = observable_1.clone();

    let _subscription_1 = observable_1.subscribe(observer_1);
    let (on_next, on_termination) = observer_2.into_callbacks();
    let _subscription_2 = observable_2.subscribe_with_callback(on_next, on_termination);

    for index in 0..2 {
        assert!(channels.on_next(index, 111).is_continue());
        channels.on_termination(index, Termination::Error("error"));
    }
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Error("error"));
    assert_eq!(checker_2.values(), [111]);
    assert_eq!(checker_2.state(), State::Error("error"));
}

#[test]
fn test_unsub_on_next_by_take() {
    let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
    let (checker, observer) = Checker::new();

    let observable = observable
        .concat_eager_map(|value| Just::new(value).with_error_type())
        .take(1);

    let _subscription = observable.subscribe(observer);
    assert!(sender.on_next(111).is_break());
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
}

#[test]
fn test_multiple_operation() {
    let (mut sender, observable, channel_checker) = test_channel::<'_, i32, Infallible>();
    let (checker, observer) = Checker::new();

    let observable = observable
        .concat_eager_map(|value| Just::new(value * 2))
        .concat_eager_map(|value| Just::new(value + 1));

    let _subscription = observable.subscribe(observer);
    assert!(sender.on_next(1).is_continue());
    assert!(sender.on_next(2).is_continue());
    sender.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [3, 5]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
}

#[test]
fn test_without_convenient_api() {
    let (mut sender, observable, channel_checker) = test_channel::<'_, i32, Infallible>();
    let (checker, observer) = Checker::new();

    let observable = ConcatEagerMap::new(observable, Just::new);

    let _subscription = observable.subscribe(observer);
    assert!(sender.on_next(111).is_continue());
    sender.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
}

#[test]
fn test_next_on_sub() {
    let (mut sender, observable, _) = test_channel::<'_, i32, Infallible>();
    let observable = observable.start_with([1]);
    let (checker, observer) = Checker::new();

    let _subscription = observable
        .concat_eager_map(|value| Just::new(value * 10))
        .subscribe(observer);
    assert_eq!(checker.values(), [10]);

    assert!(sender.on_next(2).is_continue());
    sender.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [10, 20]);
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_complete_on_sub() {
    let (checker, observer) = Checker::<Infallible, Infallible>::new();

    let _subscription = Empty.concat_eager_map(|_| Empty).subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_error_on_sub() {
    let (checker, observer) = Checker::<Infallible, _>::new();

    let _subscription = Throw::new("error")
        .concat_eager_map(|_: Infallible| Throw::new("inner"))
        .subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Error("error"));
}

#[test]
fn test_next_on_unsub() {
    let (checker, observer) = Checker::new();
    let calls = Arc::new(AtomicUsize::new(0));
    let calls_callback = calls.clone();

    let observable = Create::shared_boxed(|observer: SendBoxedObserver<'_, i32, Infallible>| {
        DisposeOnDrop::new(CallbackDisposal::new(move || {
            let mut observer = observer;
            assert!(observer.on_next(111).is_break());
        }))
    });

    let subscription = observable
        .concat_eager_map(move |value| {
            calls_callback.fetch_add(1, Ordering::SeqCst);
            Just::new(value)
        })
        .subscribe(observer);
    subscription.dispose();
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Dropped);
    // The callback runs as the value arrives; the observable it made finds the stream over, and
    // is dropped unsubscribed.
    assert_eq!(calls.load(Ordering::SeqCst), 1);
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
            observer.on_termination(Termination::Completed);
            DisposeOnDrop::new(CallbackDisposal::new(|| {
                life_marker.consume_ref();
            }))
        });

        let observable = observable.concat_eager_map(Just::new);

        let (_, observer) = Checker::<_, Infallible>::new();
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
        let observable =
            Create::shared_boxed(|observer: SendBoxedObserver<'_, &TestStruct, Infallible>| {
                life_marker_1 = Some(observer);
                DisposeOnDrop::default()
            });
        let observable = observable.concat_eager_map(Just::new);

        let (_, mut observer) = Checker::new();
        assert!(observer.on_next(&life_marker_2).is_continue());
        let _subscription = observable.subscribe(observer);
    }
}

#[test]
fn test_fn() {
    let mut s = TestStruct;

    let observable = Just::new(1).concat_eager_map(|value| {
        s.consume_mut();
        Just::new(value)
    });

    let _ = observable.subscribe_with_callback(|_| {}, |_| {});
}

#[test]
fn test_clone() {
    let observable = Create::shared_boxed(|mut observer| {
        assert!(observer.on_next(TestStruct).is_continue());
        observer.on_termination(Termination::Error(TestStruct));
        DisposeOnDrop::default()
    });
    let observable = observable.concat_eager_map(|value| Just::new(value).with_error_type());
    _ = observable.clone(); // Make sure it's Clone when T and E are not Clone.
}

#[test]
fn test_type_inference_with_subscribe() {
    let observable = Just::new(1).concat_eager_map(Just::new);

    let observable = observable.concat_eager_map(Just::new);
    let (_, observer) = Checker::new();
    let _ = observable.subscribe(observer);
}

#[test]
fn test_type_inference_without_subscribe() {
    let observable = Just::new(1).concat_eager_map(Just::new);

    observable.concat_eager_map(Just::new);
}

#[cfg(panic = "unwind")]
#[test]
fn test_unsubscribe_on_panic_while_subscribing() {
    use crate::tests_utils::panic::expect_panic_on_drop;

    let (mut sender, source, source_checker) = test_channel();
    let (_sender_1, observable_1, channel_checker_1) = test_channel::<'_, i32, Infallible>();
    let (checker, observer) = Checker::new();

    // The observable of the second value fails to subscribe: the stream is given up, and the one
    // still running is unsubscribed from, rather than left with a place taken for good, so that
    // the stream would never complete.
    expect_panic_on_drop(|panic_on_drop| {
        let failing = Create::shared_boxed(move |_: SendBoxedObserver<'_, i32, Infallible>| {
            drop(panic_on_drop);
            DisposeOnDrop::default()
        });
        let mut inners = [Some(observable_1.into_boxed()), Some(failing.into_boxed())];
        let _subscription = source
            .concat_eager_map(move |index: usize| inners[index].take().unwrap())
            .subscribe(observer);
        assert!(sender.on_next(0).is_continue());
        let _ = sender.on_next(1);
    });
    assert_eq!(source_checker.state(), ChannelState::Unsubscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Unsubscribed);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Dropped);
}

#[cfg(panic = "unwind")]
#[test]
fn test_panic_in_selector() {
    use crate::tests_utils::panic::expect_panic_on_drop;
    use rx_rust::thread_mode::mutable::MutableExt;
    use std::sync::Mutex;

    let (checker, observer) = Checker::<i32, Infallible>::new();
    let panic_on_drop = Arc::new(Mutex::new(None));

    // A source that goes on after its observer unwinds: a test channel lets go of the observer.
    let source = Create::shared_boxed({
        let panic_on_drop = panic_on_drop.clone();
        let checker = checker.clone();
        move |mut observer: SendBoxedObserver<'_, i32, Infallible>| {
            // The selector fails before it has made an observable: no position is taken for it,
            // so the next one is the first, rather than kept for good behind a position that never
            // completes.
            expect_panic_on_drop(|panic| {
                panic_on_drop.replace_value(Some(panic));
                let _ = observer.on_next(1);
            });
            assert!(checker.values().is_empty());
            assert_eq!(checker.state(), State::Active);

            assert!(observer.on_next(2).is_continue());
            assert_eq!(checker.values(), [2]);
            observer.on_termination(Termination::Completed);
            DisposeOnDrop::new(CallbackDisposal::new(|| {}))
        }
    });
    let _subscription = source
        .concat_eager_map(move |value| {
            drop(panic_on_drop.take_value());
            Just::new(value)
        })
        .subscribe(observer);

    assert_eq!(checker.values(), [2]);
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_max_concurrency() {
    let (mut sender, observable, _channel_checker) = test_channel();
    let (inner_channels, inner) = test_channels::<'_, i32, Infallible>();
    let (checker, observer) = Checker::new();

    let _subscription = observable
        .concat_eager_map(move |value: i32| {
            inner.clone().map(move |inner_value| inner_value + value)
        })
        .max_concurrency(NonZeroUsize::new(2).unwrap())
        .subscribe(observer);

    // Two at a time: the observable of the third value waits unsubscribed.
    assert!(sender.on_next(100).is_continue());
    assert!(sender.on_next(200).is_continue());
    assert!(sender.on_next(300).is_continue());
    sender.on_termination(Termination::Completed);
    assert_eq!(inner_channels.len(), 2);

    assert!(inner_channels.on_next(1, 2).is_continue());
    assert!(inner_channels.on_next(0, 1).is_continue());
    inner_channels.on_termination(0, Termination::Completed);
    assert_eq!(checker.values(), [101, 202]);
    assert_eq!(inner_channels.len(), 3);

    assert!(inner_channels.on_next(2, 3).is_continue());
    inner_channels.on_termination(1, Termination::Completed);
    assert_eq!(checker.values(), [101, 202, 303]);
    assert_eq!(checker.state(), State::Active);

    inner_channels.on_termination(2, Termination::Completed);
    assert_eq!(checker.state(), State::Completed);
}
