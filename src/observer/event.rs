/// A `Terminated` is a value that an `Observable` can send to an `Observer` to indicate that the observable has terminated.
#[derive(Debug, PartialEq, Eq)]
pub enum Terminated<E> {
    Error(E),
    Cancelled,
    Completed,
}

/// An `Event` is a value that an `Observable` can send to an `Observer`.
#[derive(Debug, PartialEq, Eq)]
pub enum Event<T, E> {
    Next(T),
    Terminated(Terminated<E>),
}

impl<T, E> Event<T, E> {
    /**
    Maps the value type of the event to a new value type using the given function.

    # Example
    ```rust
    use rx_rust::observer::event::Event;
    let event = Event::<i32, String>::Next(123);
    let new_event = event.map_value(|value| value.to_string());
    assert_eq!(new_event, Event::Next("123".to_owned()));
    ```
     */
    pub fn map_value<T2, F>(self, f: F) -> Event<T2, E>
    where
        F: Fn(T) -> T2,
    {
        match self {
            Event::Next(value) => Event::Next(f(value)),
            Event::Terminated(terminated) => Event::Terminated(terminated),
        }
    }

    /**
    Maps the error type of the event to a new error type using the given function.

    # Example
    ```rust
    use rx_rust::observer::event::Event;
    use rx_rust::observer::event::Terminated;
    let event = Event::<i32, i32>::Terminated(Terminated::Error(123));
    let new_event = event.map_error(|error_code| error_code.to_string());
    assert_eq!(new_event, Event::Terminated(Terminated::Error("123".to_owned())));
    ```
     */
    pub fn map_error<E2, F>(self, f: F) -> Event<T, E2>
    where
        F: Fn(E) -> E2,
    {
        match self {
            Event::Next(value) => Event::Next(value),
            Event::Terminated(terminated) => match terminated {
                Terminated::Error(error) => Event::Terminated(Terminated::Error(f(error))),
                Terminated::Cancelled => Event::Terminated(Terminated::Cancelled),
                Terminated::Completed => Event::Terminated(Terminated::Completed),
            },
        }
    }
}
