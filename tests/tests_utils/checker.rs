use crate::tests_utils::test_runtime::TestRuntime;
use educe::Educe;
use rx_rust::disposable::callback_disposal::CallbackDisposal;
use rx_rust::utils::mutable::MutableExt;
use rx_rust::{
    disposable::Disposable,
    observer::{Observer, Termination},
    scheduler::Scheduler,
    utils::mutable::{Mutable, MutableHelper},
    utils::types::{MaybeSend, Shared},
};
use {
    futures::Stream, futures::stream::StreamExt, rx_rust::observable::Subscription,
    std::convert::Infallible,
};

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
        self.values.clone_value()
    }

    pub(crate) fn state(&self) -> State<E>
    where
        E: Clone,
    {
        self.state.clone_value()
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
        impl FnMut(T) + MaybeSend,
        impl FnOnce(Termination<E>) + MaybeSend,
    )
    where
        T: MaybeSend,
        E: MaybeSend,
    {
        let values = self.values.clone();
        (
            move |value| values.with_mut(|values| values.push(value)),
            |termination| self.on_termination(termination),
        )
    }
}

impl<T, E> Drop for CheckerObserver<T, E> {
    fn drop(&mut self) {
        self.state.with_mut(|lock| match &*lock {
            State::Active => *lock = State::Dropped,
            State::Completed | State::Error(_) => {}
            State::Dropped => panic!(),
        })
    }
}

impl<T, E> Observer<T, E> for CheckerObserver<T, E> {
    fn on_next(&mut self, value: T) {
        self.values.with_mut(|values| values.push(value));
    }

    fn on_termination(self, termination: Termination<E>) {
        let new_value = match termination {
            Termination::Completed => State::Completed,
            Termination::Error(error) => State::Error(error),
        };
        match self.state.replace_value(new_value) {
            State::Active => {}
            State::Completed | State::Error(_) | State::Dropped => panic!(),
        }
    }
}

impl<T> Checker<T, Infallible> {
    pub(crate) fn from_stream(
        stream: impl Stream<Item = T> + MaybeSend + 'static,
        runtime: TestRuntime,
    ) -> (Self, Subscription<impl Disposable + MaybeSend + 'static>)
    where
        T: MaybeSend + 'static,
    {
        let values = Shared::new(Mutable::new(Vec::new()));
        let state = Shared::new(Mutable::new(State::Active));

        let values_cloned = values.clone();
        let state_cloned = state.clone();
        let handle = runtime.spawn_future(async move {
            let mut stream = std::pin::pin!(stream);
            while let Some(value) = stream.next().await {
                values_cloned.with_mut(|values| values.push(value));
            }
            match state_cloned.replace_value(State::Completed) {
                State::Active => {}
                State::Completed | State::Error(_) | State::Dropped => panic!(),
            }
        });
        (
            Self {
                values,
                state: state.clone(),
            },
            Subscription::new(CallbackDisposal::new(move || {
                use rx_rust::disposable::Disposable;
                handle.dispose();
                state.with_mut(|lock| match &*lock {
                    State::Active => *lock = State::Dropped,
                    State::Completed | State::Error(_) => {}
                    State::Dropped => panic!(),
                });
            })),
        )
    }
}
