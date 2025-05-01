use super::{Observer, Terminal};

/// TODO: doc
/// https://stackoverflow.com/a/56447952/9315497
pub struct BoxedObserver<'or, T, E>(Box<dyn FnMut(HandleEvent<T, E>) + Send + 'or>);

enum HandleEvent<T, E> {
    Next(T),
    Terminal(Terminal<E>),
}

impl<'or, T, E> BoxedObserver<'or, T, E> {
    pub fn new(observer: impl Observer<T, E> + Send + 'or) -> Self {
        let mut observer = Some(observer);
        Self(Box::new(move |event| match event {
            HandleEvent::Next(value) => {
                if let Some(observer) = &mut observer {
                    observer.on_next(value);
                }
            }
            HandleEvent::Terminal(terminal) => {
                if let Some(observer) = observer.take() {
                    observer.on_terminal(terminal);
                }
            }
        }))
    }
}

impl<T, E> Observer<T, E> for BoxedObserver<'_, T, E> {
    fn on_next(&mut self, value: T) {
        self.0(HandleEvent::Next(value));
    }

    fn on_terminal(mut self, terminal: Terminal<E>) {
        self.0(HandleEvent::Terminal(terminal));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::tests_utils::{checker::Checker, test_struct::TestStruct};

    #[test]
    fn test_completed() {
        let (checker, observer) = Checker::new();
        let mut boxed_observer = BoxedObserver::new(observer);
        boxed_observer.on_next(111);
        boxed_observer.on_terminal(Terminal::<&str>::Completed);

        assert!(checker.is_values_matched(&[111]));
        assert!(checker.is_completed());
    }

    #[test]
    fn test_error() {
        let (checker, observer) = Checker::new();
        let mut boxed_observer = BoxedObserver::new(observer);
        boxed_observer.on_next(111);
        boxed_observer.on_terminal(Terminal::Error("error"));

        assert!(checker.is_values_matched(&[111]));
        assert!(checker.is_error("error"));
    }

    #[test]
    fn test_ref() {
        let value = 111;
        let error = 222;
        let (checker, observer) = Checker::new();
        let mut boxed_observer = BoxedObserver::new(observer);
        boxed_observer.on_next(&value);
        boxed_observer.on_terminal(Terminal::Error(&error));

        assert!(checker.is_values_matched(&[&value]));
        assert!(checker.is_error(&error));
    }

    #[test]
    fn test_mut_ref() {
        struct MyObserver;
        impl Observer<&mut i32, &mut i32> for MyObserver {
            fn on_next(&mut self, value: &mut i32) {
                *value *= 2
            }

            fn on_terminal(self, terminal: Terminal<&mut i32>) {
                match terminal {
                    Terminal::Completed => unreachable!(),
                    Terminal::Error(error) => *error *= 2,
                }
            }
        }
        let mut value = 111;
        let mut error = 222;
        let observer = MyObserver;
        let mut boxed_observer = BoxedObserver::new(observer);
        boxed_observer.on_next(&mut value);
        boxed_observer.on_terminal(Terminal::Error(&mut error));

        assert_eq!(value, 222);
        assert_eq!(error, 444);
    }

    #[tokio::test]
    async fn test_async() {
        let (checker, observer) = Checker::new();
        let checker_cloned = checker.clone();
        let mut boxed_observer = tokio::spawn(async { BoxedObserver::new(observer) })
            .await
            .unwrap();
        tokio::spawn(async move {
            boxed_observer.on_next(111);
            boxed_observer.on_terminal(Terminal::Error("error"));
            assert!(checker.is_values_matched(&[111]));
            assert!(checker.is_error("error"));
        })
        .await
        .unwrap();
    }

    #[test]
    fn test_lifetime() {
        // OK
        let life_marker = TestStruct;
        let boxed_observer;

        // Error
        // let boxed_observer;
        // let life_marker = TestStruct;

        {
            let (checker, mut observer) = Checker::<_, &str>::new();
            observer.on_next(&life_marker);
            boxed_observer = BoxedObserver::new(observer);
        }

        _ = boxed_observer;
    }
}
