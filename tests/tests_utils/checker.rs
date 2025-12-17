use educe::Educe;
use rx_rust::{
    observer::{Observer, Termination},
    utils::types::{Mutable, MutableHelper, NecessarySendSync, Shared},
};
use rx_rust::{safe_lock, safe_lock_vec};

#[derive(Educe)]
#[educe(Debug, Clone, PartialEq, Eq)]
pub(crate) enum State<E> {
    Active,
    Dropped,
    Completed,
    Error(E),
}

/// A helper struct for testing observables.
#[derive(Educe)]
#[educe(Debug, Clone)]
pub(crate) struct Checker<T, E> {
    values: Shared<Mutable<Vec<T>>>,
    state: Shared<Mutable<State<E>>>,
}

impl<T, E> Checker<T, E> {
    pub(crate) fn new() -> (Self, CheckerObserver<T, E>) {
        let values = Shared::new(Mutable::new(Vec::new()));
        let state = Shared::new(Mutable::new(State::Active));
        (
            Self {
                values: values.clone(),
                state: state.clone(),
            },
            CheckerObserver { values, state },
        )
    }

    pub(crate) fn values(&self) -> Vec<T>
    where
        T: Clone,
    {
        safe_lock!(clone: self.values)
    }

    pub(crate) fn state(&self) -> State<E>
    where
        E: Clone,
    {
        safe_lock!(clone: self.state)
    }
}

#[derive(Educe)]
#[educe(Debug)]
pub(crate) struct CheckerObserver<T, E> {
    values: Shared<Mutable<Vec<T>>>,
    state: Shared<Mutable<State<E>>>,
}

impl<T, E> CheckerObserver<T, E> {
    pub(crate) fn into_callbacks(
        self,
    ) -> (
        impl FnMut(T) + NecessarySendSync,
        impl FnOnce(Termination<E>) + NecessarySendSync,
    )
    where
        T: NecessarySendSync,
        E: NecessarySendSync,
    {
        let values = self.values.clone();
        (
            move |value| safe_lock_vec!(push: values, value),
            |termination| self.on_termination(termination),
        )
    }
}

impl<T, E> Drop for CheckerObserver<T, E> {
    fn drop(&mut self) {
        self.state.lock_mut(|mut lock| match &*lock {
            State::Active => *lock = State::Dropped,
            State::Completed | State::Error(_) => {}
            State::Dropped => panic!(),
        })
    }
}

impl<T, E> Observer<T, E> for CheckerObserver<T, E> {
    fn on_next(&mut self, value: T) {
        safe_lock_vec!(push: self.values, value);
    }

    fn on_termination(self, termination: Termination<E>) {
        let new_value = match termination {
            Termination::Completed => State::Completed,
            Termination::Error(error) => State::Error(error),
        };
        match safe_lock!(mem_replace: self.state, new_value) {
            State::Active => {}
            State::Completed | State::Error(_) | State::Dropped => panic!(),
        }
    }
}

#[cfg(feature = "futures")]
use {
    crate::tests_utils::test_runtime::TestRuntime, futures::Stream, futures::stream::StreamExt,
    rx_rust::disposable::subscription::Subscription, std::convert::Infallible,
};

#[cfg(feature = "futures")]
impl<T> Checker<T, Infallible> {
    pub(crate) fn from_stream(
        mut stream: impl Stream<Item = T> + NecessarySendSync + Unpin + 'static,
        runtime: TestRuntime,
    ) -> (Self, Subscription<'static>)
    where
        T: NecessarySendSync + 'static,
    {
        let values = Shared::new(Mutable::new(Vec::new()));
        let state = Shared::new(Mutable::new(State::Active));

        let values_cloned = values.clone();
        let state_cloned = state.clone();
        let handle = runtime.spawn(async move {
            while let Some(value) = stream.next().await {
                safe_lock_vec!(push: values_cloned, value);
            }
            match safe_lock!(mem_replace: state_cloned, State::Completed) {
                State::Active => {}
                State::Completed | State::Error(_) | State::Dropped => panic!(),
            }
        });
        (
            Self {
                values,
                state: state.clone(),
            },
            Subscription::new_with_disposal_callback(move || {
                use rx_rust::disposable::Disposable;
                handle.dispose();
                state.lock_mut(|mut lock| match &*lock {
                    State::Active => *lock = State::Dropped,
                    State::Completed | State::Error(_) => {}
                    State::Dropped => panic!(),
                });
            }),
        )
    }
}
