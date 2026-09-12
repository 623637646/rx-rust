mod tests_utils;

use crate::tests_utils::checker::State;
use crate::tests_utils::test_runtime::block_on;
use std::convert::Infallible;

use rx_rust::observer::{Flow, Observer, Termination, boxed_observer::BoxedObserver};
use tests_utils::{checker::Checker, test_struct::TestStruct};

#[test]
fn test_completed() {
    let (checker, observer) = Checker::new();
    let mut boxed_observer = BoxedObserver::new(observer);
    assert!(boxed_observer.on_next(111).is_continue());
    boxed_observer.on_termination(Termination::<Infallible>::Completed);

    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_error() {
    let (checker, observer) = Checker::new();
    let mut boxed_observer = BoxedObserver::new(observer);
    assert!(boxed_observer.on_next(111).is_continue());
    boxed_observer.on_termination(Termination::Error("error"));

    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Error("error"));
}

#[test]
fn test_ref() {
    let value = 111;
    let error = 222;
    let (checker, observer) = Checker::new();
    let mut boxed_observer = BoxedObserver::new(observer);
    assert!(boxed_observer.on_next(&value).is_continue());
    boxed_observer.on_termination(Termination::Error(&error));

    assert_eq!(checker.values(), [&value]);
    assert_eq!(checker.state(), State::Error(&error));
}

#[test]
fn test_mut_ref() {
    struct MyObserver;
    impl Observer<&mut i32, &mut i32> for MyObserver {
        fn on_next(&mut self, value: &mut i32) -> Flow {
            *value *= 2;
            Flow::Continue
        }

        fn on_termination(self, termination: Termination<&mut i32>) {
            match termination {
                Termination::Completed => unreachable!(),
                Termination::Error(error) => *error *= 2,
            }
        }
    }
    let mut value = 111;
    let mut error = 222;
    let observer = MyObserver;
    let mut boxed_observer = BoxedObserver::new(observer);
    assert!(boxed_observer.on_next(&mut value).is_continue());
    boxed_observer.on_termination(Termination::Error(&mut error));

    assert_eq!(value, 222);
    assert_eq!(error, 444);
}

#[test]
fn test_async() {
    block_on(|runtime| async move {
        let (checker, observer) = Checker::new();
        let mut boxed_observer = runtime
            .spawn(async { BoxedObserver::new(observer) })
            .await
            .unwrap();
        runtime
            .spawn(async move {
                assert!(boxed_observer.on_next(111).is_continue());
                boxed_observer.on_termination(Termination::Error("error"));
                assert_eq!(checker.values(), [111]);
                assert_eq!(checker.state(), State::Error("error"));
            })
            .await
            .unwrap();
    });
}

#[test]
fn test_lifetime_or() {
    // OK
    let life_marker = TestStruct;
    let _boxed_observer;

    // Error
    // let _boxed_observer;
    // let life_marker = TestStruct;

    {
        let (_, mut observer) = Checker::<_, &str>::new();
        assert!(observer.on_next(&life_marker).is_continue());
        _boxed_observer = BoxedObserver::new(observer);
    }
}
