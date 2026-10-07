//! [`Scheduler`] for Tokio: [`TokioScheduler`] for a runtime, [`TokioLocalScheduler`] for a
//! [`LocalSet`].

use crate::{
    disposable::Disposable,
    observable::Subscription,
    scheduler::{Scheduler, SchedulerTypes, Task, drive},
    thread_mode::{Local, Shared},
};
use std::{
    rc::{Rc, Weak},
    time::Duration,
};
use tokio::{
    runtime::Handle,
    task::{JoinHandle, LocalSet},
};

/// The multi-threaded scheduler for Tokio: tasks run on a Tokio runtime, so they must be `Send`.
///
/// The scheduler holds a runtime [`Handle`], so it can run tasks from any thread, inside the
/// runtime or not: [`current`](Self::current) (also the [`Default`]) takes the current runtime's
/// when the scheduler is built, so that a missing runtime fails there, and
/// [`from_handle`](Self::from_handle) takes a given one.
///
/// The runtime must have its time driver enabled (`enable_time` or `enable_all` on the builder;
/// `#[tokio::main]` does), or running a delayed task panics.
///
/// # Examples
/// ```rust
/// # #[cfg(not(feature = "tokio-scheduler"))]
/// # fn main() {}
/// # #[cfg(feature = "tokio-scheduler")]
/// #[tokio::main]
/// async fn main() {
///     use futures::StreamExt;
///     use rx_rust::{
///         observable::ObservableExt, operators::creating::from_iter::FromIter,
///         scheduler::runtime::tokio::TokioScheduler,
///     };
///     use std::time::Duration;
///
///     let values = FromIter::new(vec![1, 2, 3])
///         .delay(Duration::from_millis(5), TokioScheduler::current())
///         .into_stream()
///         .collect::<Vec<_>>()
///         .await;
///     assert_eq!(values, [1, 2, 3]);
/// }
/// ```
///
/// The observer is handed to a task that may run on another thread, so an observer holding an
/// `Rc` is refused at compile time:
///
/// ```compile_fail
/// use rx_rust::{
///     observable::ObservableExt, operators::creating::just::Just,
///     scheduler::runtime::tokio::TokioScheduler,
/// };
/// use std::{cell::RefCell, rc::Rc, time::Duration};
///
/// let values = Rc::new(RefCell::new(Vec::new()));
/// let _subscription = Just::new(1)
///     .delay(Duration::from_millis(5), TokioScheduler::current())
///     .subscribe_with_callback(move |value| values.borrow_mut().push(value), |_| {});
/// ```
///
/// [`TokioLocalScheduler`] takes it (run inside a [`LocalSet`]):
///
/// ```no_run
/// use rx_rust::{
///     observable::ObservableExt, operators::creating::just::Just,
///     scheduler::runtime::tokio::TokioLocalScheduler,
/// };
/// use std::{cell::RefCell, rc::Rc, time::Duration};
///
/// let values = Rc::new(RefCell::new(Vec::new()));
/// let _subscription = Just::new(1)
///     .delay(Duration::from_millis(5), TokioLocalScheduler::ambient())
///     .subscribe_with_callback(move |value| values.borrow_mut().push(value), |_| {});
/// ```
#[derive(Debug, Clone)]
pub struct TokioScheduler {
    handle: Handle,
}

impl Default for TokioScheduler {
    fn default() -> Self {
        Self::current()
    }
}

impl TokioScheduler {
    /// Takes the handle of the runtime the calling thread is in.
    ///
    /// # Panics
    ///
    /// Panics outside of a Tokio runtime; [`try_current`](Self::try_current) does not.
    pub fn current() -> Self {
        Self::from_handle(Handle::current())
    }

    /// Like [`current`](Self::current), but `None` outside of a Tokio runtime.
    pub fn try_current() -> Option<Self> {
        Handle::try_current().ok().map(Self::from_handle)
    }

    /// Runs the tasks on the runtime `handle` belongs to.
    pub fn from_handle(handle: Handle) -> Self {
        Self { handle }
    }
}

impl SchedulerTypes for TokioScheduler {
    type Mode = Shared;
    type Disposal = TokioDisposal;
}

impl<TC, P> Scheduler<TC, P> for TokioScheduler
where
    TC: Send + 'static,
    P: Send + 'static,
{
    fn run_task(&self, task: Task<TC, P>, delay: Option<Duration>) -> Subscription<Self::Disposal> {
        let future = drive(task, delay, tokio::time::sleep);
        Subscription::new(TokioDisposal(self.handle.spawn(future)))
    }
}

/// The single-threaded scheduler for Tokio: tasks run on a [`LocalSet`], so they need not be
/// `Send` — an observer holding an `Rc` works.
///
/// It takes one of two forms:
/// - **Ambient** ([`ambient`](Self::ambient), also the [`Default`]): each task goes to the
///   `LocalSet` the calling thread is running, through [`tokio::task::spawn_local`]. Tokio offers
///   no way to check for one beforehand, so running a task outside of a `LocalSet` panics.
/// - **Handle** ([`from_local_set`](Self::from_local_set)): the tasks go to one given `LocalSet`,
///   from anywhere on its thread, inside it or not; they wait there until the caller drives it
///   (`run_until`, `block_on`, or awaiting it).
///
/// # Examples
/// ```rust
/// # #[cfg(not(feature = "tokio-scheduler"))]
/// # fn main() {}
/// # #[cfg(feature = "tokio-scheduler")]
/// fn main() {
///     use rx_rust::{
///         observable::ObservableExt, operators::creating::from_iter::FromIter,
///         scheduler::runtime::tokio::TokioLocalScheduler,
///     };
///     use std::{cell::RefCell, rc::Rc, time::Duration};
///     use tokio::task::LocalSet;
///
///     let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
///     let local_set = Rc::new(LocalSet::new());
///     let scheduler = TokioLocalScheduler::from_local_set(&local_set);
///
///     // Subscribed before the `LocalSet` runs: the delayed values wait in it.
///     let values = Rc::new(RefCell::new(Vec::new()));
///     let values_observer = Rc::clone(&values);
///     let _subscription = FromIter::new(vec![1, 2, 3])
///         .delay(Duration::from_millis(5), scheduler)
///         .subscribe_with_callback(move |value| values_observer.borrow_mut().push(value), |_| {});
///
///     runtime.block_on(local_set.run_until(async {
///         tokio::time::sleep(Duration::from_millis(20)).await;
///     }));
///     assert_eq!(*values.borrow(), [1, 2, 3]);
/// }
/// ```
#[derive(Debug, Clone, Default)]
pub struct TokioLocalScheduler {
    target: LocalTarget,
}

/// Where a [`TokioLocalScheduler`] sends its tasks.
#[derive(Debug, Clone, Default)]
enum LocalTarget {
    /// The `LocalSet` the calling thread is running.
    #[default]
    Ambient,
    /// A given `LocalSet`, held weakly (see the [module docs](crate::scheduler)).
    Handle(Weak<LocalSet>),
}

impl TokioLocalScheduler {
    /// Sends each task to the `LocalSet` the calling thread is running.
    ///
    /// # Panics
    ///
    /// Running a task panics outside of a `LocalSet`.
    pub fn ambient() -> Self {
        Self::default()
    }

    /// Sends the tasks to `local_set`, which the caller drives. The scheduler does not keep it
    /// alive: dropping it cancels the tasks it holds.
    ///
    /// # Panics
    ///
    /// Running a task panics once `local_set` has been dropped.
    pub fn from_local_set(local_set: &Rc<LocalSet>) -> Self {
        Self {
            target: LocalTarget::Handle(Rc::downgrade(local_set)),
        }
    }
}

impl SchedulerTypes for TokioLocalScheduler {
    type Mode = Local;
    type Disposal = TokioDisposal;
}

impl<TC, P> Scheduler<TC, P> for TokioLocalScheduler
where
    TC: 'static,
    P: 'static,
{
    fn run_task(&self, task: Task<TC, P>, delay: Option<Duration>) -> Subscription<Self::Disposal> {
        let future = drive(task, delay, tokio::time::sleep);
        let handle = match &self.target {
            LocalTarget::Ambient => tokio::task::spawn_local(future),
            LocalTarget::Handle(local_set) => local_set
                .upgrade()
                .expect("the LocalSet of the TokioLocalScheduler has been dropped")
                .spawn_local(future),
        };
        Subscription::new(TokioDisposal(handle))
    }
}

/// The handle of a task spawned on Tokio; disposing it aborts the task.
pub struct TokioDisposal(JoinHandle<()>);

impl Disposable for TokioDisposal {
    fn dispose(self) {
        // Aborting, since dropping a `JoinHandle` would only detach the task.
        self.0.abort();
    }
}
