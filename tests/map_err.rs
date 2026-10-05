mod tests_utils;

use crate::tests_utils::test_channel::{ChannelState, test_channel};
use crate::tests_utils::{checker::Checker, checker::State, test_struct::TestStruct};
use rx_rust::operators::creating::create::Create;
use rx_rust::{
    observable::{Observable, ObservableExt, Subscription},
    observer::{Observer, Termination},
    operators::error_handling::map_err::MapErr,
};
use std::convert::Infallible;

#[test]
fn test_completed() {
    let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
    let (checker, observer) = Checker::<_, String>::new();

    let _subscription = observable
        .map_err(|error: Infallible| match error {})
        .subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    assert!(sender.on_next(111).is_continue());
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    sender.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
}

#[test]
fn test_error() {
    let (mut sender, observable, channel_checker) = test_channel();
    let (checker, observer) = Checker::<_, usize>::new();

    let _subscription = observable.map_err(str::len).subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    assert!(sender.on_next(111).is_continue());
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    sender.on_termination(Termination::Error("boom"));
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Error(4));
    assert_eq!(channel_checker.state(), ChannelState::Error("boom"));
}

#[test]
fn test_error_by_reference() {
    let error = String::from("boom");
    let (sender, observable, channel_checker) = test_channel();
    let (checker, observer) = Checker::<i32, _>::new();

    let _subscription = observable
        .map_err(|error: &str| error.as_bytes())
        .subscribe(observer);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    sender.on_termination(Termination::Error(error.as_str()));
    assert_eq!(checker.state(), State::Error(b"boom".as_slice()));
    assert_eq!(channel_checker.state(), ChannelState::Error("boom"));
}

#[test]
fn test_without_convenient_api() {
    let (sender, observable, channel_checker) = test_channel();
    let (checker, observer) = Checker::<i32, usize>::new();

    let _subscription = MapErr::new(observable, str::len).subscribe(observer);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    sender.on_termination(Termination::Error("boom"));
    assert_eq!(checker.state(), State::Error(4));
    assert_eq!(channel_checker.state(), ChannelState::Error("boom"));
}

#[test]
fn test_clone() {
    let observable = Create::shared_boxed(|mut observer| {
        assert!(observer.on_next(TestStruct).is_continue());
        observer.on_termination(Termination::Error(TestStruct));
        Subscription::default()
    });
    let observable = observable.map_err(|_| String::new());

    _ = observable.clone();
}

#[test]
fn test_type_inference_without_subscribe() {
    let (_, observable, _) = test_channel::<'_, _, &str>();

    observable
        .map_err(str::len)
        .filter(|value: &i32| *value > 0);
}
