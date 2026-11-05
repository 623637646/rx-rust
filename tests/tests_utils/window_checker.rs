use crate::tests_utils::checker::{Checker, CheckerObserver, State};
use rx_rust::{
    observable::{Observable, ObservableTypes},
    observer::{Flow, Observer, Termination},
    thread_mode::mutable::MutableHelper,
};
use std::sync::{Arc, Mutex};

/// A helper for testing an observable of windows: it subscribes every window as it is emitted, with
/// a [`Checker`] of its own, and keeps that subscription alive for the rest of the test, so that a
/// test asserts on every window at once, with [`Self::windows`].
///
/// The outer stream is recorded as a [`Checker`] of `()` would record it, one `()` for each window.
pub(crate) struct WindowChecker<T, E> {
    outer: Checker<(), E>,
    windows: Arc<Mutex<Vec<Checker<T, E>>>>,
    /// The subscriptions of the windows, which the observer pushes to.
    _subscriptions: Subscriptions,
}

/// Shared by the checker and its observer, so that the windows stay subscribed after the outer
/// observer is dropped: a disposal of the outer subscription then shows on the windows themselves.
type Subscriptions = Arc<Mutex<Vec<Box<dyn Send>>>>;

impl<T, E> WindowChecker<T, E> {
    pub(crate) fn new() -> (Self, WindowCheckerObserver<T, E>) {
        Self::with_outer(Checker::new())
    }

    /// A checker whose observer answers [`Flow::Stop`] once it has received `count` windows, as
    /// [`Checker::stopping_after`] does for values.
    pub(crate) fn stopping_after(count: usize) -> (Self, WindowCheckerObserver<T, E>) {
        Self::with_outer(Checker::stopping_after(count))
    }

    fn with_outer(
        (outer, outer_observer): (Checker<(), E>, CheckerObserver<(), E>),
    ) -> (Self, WindowCheckerObserver<T, E>) {
        let windows = Arc::new(Mutex::new(Vec::new()));
        let subscriptions = Subscriptions::default();
        (
            Self {
                outer,
                windows: windows.clone(),
                _subscriptions: subscriptions.clone(),
            },
            WindowCheckerObserver {
                outer: outer_observer,
                windows,
                subscriptions,
            },
        )
    }

    /// The values and the state of every window emitted so far, in order.
    pub(crate) fn windows(&self) -> Vec<(Vec<T>, State<E>)>
    where
        T: Clone,
        E: Clone,
    {
        self.windows.with_ref(|windows| {
            windows
                .iter()
                .map(|window| (window.values(), window.state()))
                .collect()
        })
    }

    /// The state of the outer observable.
    pub(crate) fn state(&self) -> State<E>
    where
        E: Clone,
    {
        self.outer.state()
    }
}

pub(crate) struct WindowCheckerObserver<T, E> {
    outer: CheckerObserver<(), E>,
    windows: Arc<Mutex<Vec<Checker<T, E>>>>,
    subscriptions: Subscriptions,
}

impl<T, E, W> Observer<W, E> for WindowCheckerObserver<T, E>
where
    W: Observable<CheckerObserver<T, E>, Item = T, Error = E>,
    <W as ObservableTypes>::Disposal: Send + 'static,
{
    fn on_next(&mut self, window: W) -> Flow {
        let (checker, observer) = Checker::new();
        self.windows.with_mut(|windows| windows.push(checker));
        let subscription = window.subscribe(observer);
        self.subscriptions
            .with_mut(|subscriptions| subscriptions.push(Box::new(subscription)));
        self.outer.on_next(())
    }

    fn on_termination(self, termination: Termination<E>) {
        self.outer.on_termination(termination);
    }
}
