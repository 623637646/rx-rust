use super::observable_subscribe_ext::ObservableSubscribeExt;
use super::Observable;
use crate::cancellable::Cancellable;
use crate::observer::{Event, Observer, Terminated};

/// An `ObservableCloned` is a type that can be subscribed to by an `Observer`. The `Observer` will receive cloned values.
pub trait ObservableCloned<T, E> {
    /// Subscribes an observer to this observable. Returns a cancellable that can be used to cancel the subscription.
    /// The Observer must be 'static because it will be stored in hot observables or pass to `subscribe_handler` of `Create`.
    /// The Cancellable must be 'static because it may be stored by callers.
    fn subscribe_cloned(
        &self,
        observer: impl Observer<T, E> + 'static,
    ) -> impl Cancellable + 'static;
}

impl<T, E, O> ObservableCloned<T, E> for O
where
    O: Observable<T, E>,
    T: Clone,
{
    fn subscribe_cloned(
        &self,
        observer: impl Observer<T, E> + 'static,
    ) -> impl Cancellable + 'static {
        self.subscribe_on_event(move |event| match event {
            Event::Next(value) => observer.on(Event::Next(value.clone())),
            Event::Terminated(terminated) => match terminated {
                Terminated::Error(error) => {
                    observer.on(Event::Terminated(Terminated::Error(error)))
                }
                Terminated::Cancelled => observer.on(Event::Terminated(Terminated::Cancelled)),
                Terminated::Completed => observer.on(Event::Terminated(Terminated::Completed)),
            },
        })
    }
}
