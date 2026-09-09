mod tests_utils;

use crate::tests_utils::checker::State;
use crate::tests_utils::test_channel::ChannelState;
use crate::tests_utils::test_channel::test_channel;
use crate::tests_utils::test_runtime::block_on;
use paste::paste;
use rx_rust::disposable::Disposable;
use rx_rust::disposable::callback_disposal::CallbackDisposal;
use rx_rust::observable::Subscription;
use rx_rust::operators::mathematical_aggregate::average::Average;
use rx_rust::{
    observable::{Observable, ObservableExt},
    observer::{Observer, Termination},
    operators::creating::{create::Create, just::Just},
    subject::publish_subject::PublishSubject,
};
use std::convert::Infallible;
use tests_utils::{checker::Checker, test_struct::TestStruct};

macro_rules! average_observer_impl {
    ($($t:ty)*) => ($(
        paste! {
            #[test]
            fn [<test_completed_$t>]() {
                let (mut sender, observable, channel_checker) = test_channel::<'_, $t, Infallible>();
                let (checker, observer) = Checker::new();

                // Custom operations
                let observable = observable.average();

                let _subscription = observable.subscribe(observer);
                assert!(checker.values().is_empty());
                assert_eq!(checker.state(), State::Active);
                assert_eq!(channel_checker.state(), ChannelState::Subscribed);

                sender.on_next(1 as $t);
                assert_eq!(checker.values(), []);
                assert_eq!(checker.state(), State::Active);
                assert_eq!(channel_checker.state(), ChannelState::Subscribed);

                sender.on_next(2 as $t);
                assert_eq!(checker.values(), []);
                assert_eq!(checker.state(), State::Active);
                assert_eq!(channel_checker.state(), ChannelState::Subscribed);

                sender.on_next(4 as $t);
                assert_eq!(checker.values(), []);
                assert_eq!(checker.state(), State::Active);
                assert_eq!(channel_checker.state(), ChannelState::Subscribed);

                sender.on_termination(Termination::<Infallible>::Completed);
                assert_eq!(checker.values(), [2.3333333333333335]);
                assert_eq!(checker.state(), State::Completed);
                assert_eq!(channel_checker.state(), ChannelState::Completed);
            }
        }
    )*)
}

average_observer_impl! { usize u8 u16 u32 u64 u128 isize i8 i16 i32 i64 i128 f32 f64 }

/// The sum is accumulated in `f64`, the type of the result, so a sum that does not fit the source
/// type is still averaged correctly. Accumulating in the source type panics here in debug builds
/// and wraps around in release ones.
///
/// Two items of `MAX / 2 + 1` always overflow: their sum is `MAX + 1` at least. That value is a
/// power of two for every integer type, so both it and its double are exact in `f64` and the
/// expected average is exact too.
macro_rules! average_overflow_impl {
    ($($t:ty)*) => ($(
        paste! {
            #[test]
            fn [<test_completed_sum_overflows_$t>]() {
                let (mut sender, observable, channel_checker) = test_channel::<'_, $t, Infallible>();
                let (checker, observer) = Checker::new();

                // Custom operations
                let observable = observable.average();

                let _subscription = observable.subscribe(observer);
                assert!(checker.values().is_empty());
                assert_eq!(checker.state(), State::Active);
                assert_eq!(channel_checker.state(), ChannelState::Subscribed);

                let half = <$t>::MAX / 2 + 1;

                sender.on_next(half);
                assert_eq!(checker.values(), []);
                assert_eq!(checker.state(), State::Active);
                assert_eq!(channel_checker.state(), ChannelState::Subscribed);

                // `half + half` does not fit `$t` any more.
                sender.on_next(half);
                assert_eq!(checker.values(), []);
                assert_eq!(checker.state(), State::Active);
                assert_eq!(channel_checker.state(), ChannelState::Subscribed);

                sender.on_termination(Termination::<Infallible>::Completed);
                assert_eq!(checker.values(), [half as f64]);
                assert_eq!(checker.state(), State::Completed);
                assert_eq!(channel_checker.state(), ChannelState::Completed);
            }
        }
    )*)
}

average_overflow_impl! { usize u8 u16 u32 u64 u128 isize i8 i16 i32 i64 i128 }

/// `f32` benefits from the same widening: two `f32::MAX` items sum to `inf` in `f32` but stay far
/// inside the range of `f64`, so the average is the finite value it should be.
#[test]
fn test_completed_sum_overflows_f32() {
    let (mut sender, observable, channel_checker) = test_channel::<'_, f32, Infallible>();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = observable.average();

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    sender.on_next(f32::MAX);
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    // `f32::MAX + f32::MAX` is `inf` in `f32`.
    sender.on_next(f32::MAX);
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    // Doubling and halving are exact in binary floating point, so this is the exact average.
    sender.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), [f32::MAX as f64]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
}

/// Widening also keeps the small items of an `f32` stream from being swallowed by the large ones.
///
/// `f32` holds 24 significant bits, so `2^24 + 1` is not representable and `16_777_216f32 + 1f32`
/// rounds straight back to `16_777_216f32`. Summing in `f64` keeps the odd sum, which is what makes
/// the average land on the half.
#[test]
fn test_completed_precision_f32() {
    let (mut sender, observable, channel_checker) = test_channel::<'_, f32, Infallible>();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = observable.average();

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    sender.on_next(16_777_216f32); // 2^24
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    sender.on_next(1f32);
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    // 16_777_217 / 2, not 16_777_216 / 2.
    sender.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), [8_388_608.5]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
}

/// `f64` is where the widening stops: it is already the accumulator and the result type, so a sum
/// that leaves its range still saturates to infinity. This documents that limit rather than
/// guarding against a regression.
#[test]
fn test_completed_sum_overflows_f64() {
    let (mut sender, observable, channel_checker) = test_channel::<'_, f64, Infallible>();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = observable.average();

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    sender.on_next(f64::MAX);
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    sender.on_next(f64::MAX);
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    sender.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), [f64::INFINITY]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
}

#[test]
fn test_completed_empty() {
    let (sender, observable, channel_checker) = test_channel::<'_, i32, Infallible>();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = observable.average();

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    sender.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
}

#[test]
fn test_error() {
    let (mut sender, observable, channel_checker) = test_channel::<'_, i32, _>();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = observable.average();

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    sender.on_next(111);
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    sender.on_termination(Termination::Error("error"));
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Error("error"));
    assert_eq!(channel_checker.state(), ChannelState::Error("error"));
}

#[test]
fn test_unsubscribe() {
    let (mut sender, observable, channel_checker) = test_channel::<'_, i32, Infallible>();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = observable.average();

    let subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    sender.on_next(111);
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    subscription.dispose();
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Dropped);
    assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
}

#[test]
fn test_async() {
    block_on(|runtime| async move {
        let (mut sender, observable, channel_checker) = test_channel::<'_, i32, Infallible>();
        let (checker, observer) = Checker::new();

        // Custom operations
        let observable = observable.average();

        let _subscription = runtime
            .spawn(async move { observable.subscribe(observer) })
            .await
            .unwrap();
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        let mut sender = runtime
            .spawn(async move {
                sender.on_next(1);
                sender
            })
            .await
            .unwrap();
        assert_eq!(checker.values(), []);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        let mut sender = runtime
            .spawn(async move {
                sender.on_next(2);
                sender
            })
            .await
            .unwrap();
        assert_eq!(checker.values(), []);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        let sender = runtime
            .spawn(async move {
                sender.on_next(4);
                sender
            })
            .await
            .unwrap();
        assert_eq!(checker.values(), []);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        runtime
            .spawn(async move {
                sender.on_termination(Termination::<Infallible>::Completed);
            })
            .await
            .unwrap();
        assert_eq!(checker.values(), [2.3333333333333335]);
        assert_eq!(checker.state(), State::Completed);
        assert_eq!(channel_checker.state(), ChannelState::Completed);
    });
}

#[test]
fn test_subscribe_by_different_observer() {
    let mut subject: PublishSubject<'_, i32, &str> = PublishSubject::default();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable = observable.average();
    let observable_1 = observable;
    let observable_2 = observable_1.clone();

    let _subscription_1 = observable_1.subscribe(observer_1);
    let (on_next, on_termination) = observer_2.into_callbacks();
    let _subscription_2 = observable_2.subscribe_with_callback(on_next, on_termination);
    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);

    subject.on_next(1);
    assert_eq!(checker_1.values(), []);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), []);
    assert_eq!(checker_2.state(), State::Active);

    subject.on_next(2);
    assert_eq!(checker_1.values(), []);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), []);
    assert_eq!(checker_2.state(), State::Active);

    subject.on_next(4);
    assert_eq!(checker_1.values(), []);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), []);
    assert_eq!(checker_2.state(), State::Active);

    subject.on_termination(Termination::Completed);
    assert_eq!(checker_1.values(), [2.3333333333333335]);
    assert_eq!(checker_1.state(), State::Completed);
    assert_eq!(checker_2.values(), [2.3333333333333335]);
    assert_eq!(checker_2.state(), State::Completed);
}

#[test]
fn test_unsub_on_next_by_take() {
    let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = observable.average().take(1);

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    sender.on_next(1);
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    sender.on_next(2);
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    sender.on_next(4);
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    sender.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [2.3333333333333335]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
}

#[test]
fn test_unsub_on_next_by_take_2() {
    let (mut sender, observable, channel_checker) = test_channel::<'_, i32, Infallible>();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = observable.take(1).average();

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    sender.on_next(111);
    assert_eq!(checker.values(), [111.0]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
}

#[test]
fn test_multiple_operation() {
    let (mut sender, observable, channel_checker) = test_channel::<'_, i32, Infallible>();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = observable.average().average();

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    sender.on_next(1);
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    sender.on_next(2);
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    sender.on_next(4);
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    sender.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [2.3333333333333335]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
}

#[test]
fn test_without_convenient_api() {
    let (mut sender, observable, channel_checker) = test_channel::<'_, i32, Infallible>();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = Average::new(observable);

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    sender.on_next(1);
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    sender.on_next(2);
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    sender.on_next(4);
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    sender.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [2.3333333333333335]);
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
        let observable = Create::new(|mut observer| {
            observer.on_next(1);
            observer.on_termination(Termination::<String>::Completed);
            Subscription::new(CallbackDisposal::new(|| {
                life_marker.consume_ref();
            }))
        });

        let observable = observable.average();

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
        let observable = Create::new(|mut observer| {
            observer.on_next(111);
            life_marker_1 = Some(observer);
            Subscription::default()
        });
        let observable = observable.average();

        let _subscription = observable.subscribe_with_callback(
            |_| {
                life_marker_2.consume_ref();
            },
            |_: Termination<Infallible>| {},
        );
    }
}

#[test]
fn test_clone() {
    let observable = Create::new(|mut observer| {
        observer.on_next(TestStruct);
        observer.on_termination(Termination::Error(TestStruct));
        Subscription::default()
    });
    let observable = observable.average();
    _ = observable.clone(); // Make sure it's Clone when T and E are not Clone.
}

#[test]
fn test_type_inference_with_subscribe() {
    // Custom operations
    let observable = Just::new(1).average();

    let observable = observable.filter(|_| false);
    let (_, observer) = Checker::new();
    let _ = observable.subscribe(observer);
}

#[test]
fn test_type_inference_without_subscribe() {
    // Custom operations
    let observable = Just::new(1).average();

    observable.filter(|_| false);
}
