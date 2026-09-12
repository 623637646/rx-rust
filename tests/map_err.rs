mod tests_utils;

use crate::tests_utils::{checker::Checker, checker::State, test_struct::TestStruct};
use rx_rust::{
    observable::{Observable, ObservableExt, Subscription},
    observer::{Observer, Termination},
    operators::{creating::create::Create, error_handling::map_err::MapErr},
};
use std::convert::Infallible;

#[test]
fn test_completed() {
    let observable = Create::new(|mut observer| {
        assert!(observer.on_next(111).is_continue());
        observer.on_termination(Termination::<Infallible>::Completed);
        Subscription::default()
    });
    let (checker, observer) = Checker::<_, String>::new();

    let _subscription = observable
        .map_err(|error: Infallible| match error {})
        .subscribe(observer);

    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_error() {
    let observable = Create::new(|mut observer| {
        assert!(observer.on_next(111).is_continue());
        observer.on_termination(Termination::Error("boom"));
        Subscription::default()
    });
    let (checker, observer) = Checker::<_, usize>::new();

    let _subscription = observable.map_err(str::len).subscribe(observer);

    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Error(4));
}

#[test]
fn test_error_by_reference() {
    let error = String::from("boom");
    let observable = Create::new(|observer| {
        observer.on_termination(Termination::Error(error.as_str()));
        Subscription::default()
    });
    let (checker, observer) = Checker::<i32, _>::new();

    let _subscription = observable
        .map_err(|error| error.as_bytes())
        .subscribe(observer);

    assert_eq!(checker.state(), State::Error(b"boom".as_slice()));
}

#[test]
fn test_without_convenient_api() {
    let observable = Create::new(|observer| {
        observer.on_termination(Termination::Error("boom"));
        Subscription::default()
    });
    let (checker, observer) = Checker::<i32, usize>::new();

    let _subscription = MapErr::new(observable, str::len).subscribe(observer);

    assert_eq!(checker.state(), State::Error(4));
}

#[test]
fn test_clone() {
    let observable = Create::new(|mut observer| {
        assert!(observer.on_next(TestStruct).is_continue());
        observer.on_termination(Termination::Error(TestStruct));
        Subscription::default()
    });
    let observable = observable.map_err(|_| String::new());

    _ = observable.clone();
}

#[test]
fn test_type_inference_without_subscribe() {
    let observable = Create::new(|observer| {
        observer.on_termination(Termination::Error("boom"));
        Subscription::default()
    });

    observable
        .map_err(str::len)
        .filter(|value: &i32| *value > 0);
}
