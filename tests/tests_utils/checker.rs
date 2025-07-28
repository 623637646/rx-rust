use educe::Educe;
use rx_rust::utils::safe_lock::SafeLockVec;
use rx_rust::{
    observer::{Observer, Termination},
    utils::{
        safe_lock::SafeLock,
        types::{Mutable, MutableHelper, NecessarySend, Shared},
    },
};

#[derive(Educe)]
#[educe(Debug, Clone, PartialEq)]
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
        self.values.safe_lock_clone()
    }

    pub(crate) fn state(&self) -> State<E>
    where
        E: Clone,
    {
        self.state.safe_lock_clone()
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
        impl FnMut(T) + NecessarySend,
        impl FnOnce(Termination<E>) + NecessarySend,
    )
    where
        T: NecessarySend,
        E: NecessarySend,
    {
        let values = self.values.clone();
        (
            move |value| values.safe_lock_push(value),
            |termination| self.on_termination(termination),
        )
    }
}

impl<T, E> Drop for CheckerObserver<T, E> {
    fn drop(&mut self) {
        let mut lock = self.state.lock_mut();
        match &mut *lock {
            State::Active => *lock = State::Dropped,
            State::Completed | State::Error(_) => {}
            State::Dropped => panic!(),
        }
    }
}

impl<T, E> Observer<T, E> for CheckerObserver<T, E> {
    fn on_next(&mut self, value: T) {
        self.values.safe_lock_push(value);
    }

    fn on_termination(self, termination: Termination<E>) {
        let mut lock = self.state.lock_mut();
        match &mut *lock {
            State::Active => {
                *lock = match termination {
                    Termination::Completed => State::Completed,
                    Termination::Error(error) => State::Error(error),
                };
            }
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
        mut stream: impl Stream<Item = T> + NecessarySend + Unpin + 'static,
        runtime: TestRuntime,
    ) -> (Self, Subscription<'static>)
    where
        T: NecessarySend + 'static,
    {
        let values = Shared::new(Mutable::new(Vec::new()));
        let state = Shared::new(Mutable::new(State::Active));

        let values_cloned = values.clone();
        let state_cloned = state.clone();
        let handle = runtime.clone().spawn(async move {
            while let Some(value) = stream.next().await {
                values_cloned.safe_lock_push(value);
            }
            let mut lock = state_cloned.lock_mut();
            match &mut *lock {
                State::Active => *lock = State::Completed,
                State::Completed | State::Error(_) | State::Dropped => panic!(),
            }
        });
        (
            Self {
                values,
                state: state.clone(),
            },
            Subscription::new_with_disposal_callback(move || {
                handle.abort();
                let mut lock = state.lock_mut();
                match &mut *lock {
                    State::Active => *lock = State::Dropped,
                    State::Completed | State::Error(_) => {}
                    State::Dropped => panic!(),
                }
            }),
        )
    }
}
