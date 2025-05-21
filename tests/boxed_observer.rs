mod tests_utils;

use rx_rust::observer::{Observer, Termination, boxed_observer::BoxedObserver};
use tests_utils::{checker::Checker, test_struct::TestStruct};

#[test]
fn test_completed() {
    let (checker, observer) = Checker::new();
    let mut boxed_observer = BoxedObserver::new(observer);
    boxed_observer.on_next(111);
    boxed_observer.on_termination(Termination::<&str>::Completed);

    assert_eq!(checker.values(), [111]);
    assert!(checker.is_completed());
}

#[test]
fn test_error() {
    let (checker, observer) = Checker::new();
    let mut boxed_observer = BoxedObserver::new(observer);
    boxed_observer.on_next(111);
    boxed_observer.on_termination(Termination::Error("error"));

    assert_eq!(checker.values(), [111]);
    assert!(checker.is_error("error"));
}

#[test]
fn test_ref() {
    let value = 111;
    let error = 222;
    let (checker, observer) = Checker::new();
    let mut boxed_observer = BoxedObserver::new(observer);
    boxed_observer.on_next(&value);
    boxed_observer.on_termination(Termination::Error(&error));

    assert_eq!(checker.values(), [&value]);
    assert!(checker.is_error(&error));
}

#[test]
fn test_mut_ref() {
    struct MyObserver;
    impl Observer<&mut i32, &mut i32> for MyObserver {
        fn on_next(&mut self, value: &mut i32) {
            *value *= 2
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
    boxed_observer.on_next(&mut value);
    boxed_observer.on_termination(Termination::Error(&mut error));

    assert_eq!(value, 222);
    assert_eq!(error, 444);
}

#[tokio::test]
async fn test_async() {
    let (checker, observer) = Checker::new();
    let mut boxed_observer = tokio::spawn(async { BoxedObserver::new(observer) })
        .await
        .unwrap();
    tokio::spawn(async move {
        boxed_observer.on_next(111);
        boxed_observer.on_termination(Termination::Error("error"));
        assert_eq!(checker.values(), [111]);
        assert!(checker.is_error("error"));
    })
    .await
    .unwrap();
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
        observer.on_next(&life_marker);
        _boxed_observer = BoxedObserver::new(observer);
    }
}
