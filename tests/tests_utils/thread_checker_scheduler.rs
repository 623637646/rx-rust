use futures::FutureExt;
use futures::StreamExt;
use futures::channel::{mpsc as futures_mpsc, oneshot};
use futures::executor::LocalPool;
use futures::future::{BoxFuture, abortable};
use futures::stream::AbortHandle;
use futures::task::LocalSpawnExt;
use rx_rust::disposable::Disposable;
use rx_rust::disposable::dispose_on_drop::DisposeOnDrop;
use rx_rust::scheduler::{Scheduler, SchedulerTypes, SystemClock, Task, drive};
use rx_rust::thread_mode::Shared;
use std::cell::Cell;
use std::panic::{self, AssertUnwindSafe};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

thread_local! {
    static THREAD_NAME: Cell<Option<&'static str>> = const { Cell::new(None) };
}

/// The name of the [`ThreadCheckerExecutor`] whose thread this is, `None` on any other thread.
/// The thread's own name would not do: the test harness names every test thread after its test,
/// so a test asserting that a callback did *not* run on the executor's thread needs a mark only
/// the executor sets.
pub(crate) fn get_thread_name() -> Option<&'static str> {
    THREAD_NAME.get()
}

/// What the owner asks its thread to do. Futures to spawn travel on a channel of their own,
/// which the pool itself drains, so that a run also reaches the tasks a running task spawns.
enum Request {
    RunUntilStalled(oneshot::Sender<thread::Result<()>>),
    Shutdown,
}

/// The owner of a thread, named for [`get_thread_name`], that runs its tasks only when the test
/// asks for it. [`Scheduler::run_task`] only queues the task; [`Self::run_until_stalled`] runs
/// every queued task until none can make progress, those a running task spawns included. So the
/// test decides when the scheduler's thread runs relative to its own, instead of racing it, and
/// waits for exactly the work it started instead of sleeping.
/// Concurrent callers are served in request order; a panic is returned to the call that ran the
/// task.
///
/// Dropping the owner stops the thread, which releases all its tasks even if they hold scheduler
/// clones or their cancellation handles remain alive. On any other thread, dropping the owner also
/// waits for that shutdown. On the executor's own thread, it returns to the event loop to finish
/// the shutdown without joining itself.
///
/// [`Self::scheduler`] gives out handles that can queue tasks without keeping the executor alive.
#[derive(Debug)]
pub(crate) struct ThreadCheckerExecutor {
    scheduler: ThreadCheckerScheduler,
    requests: mpsc::Sender<Request>,
    thread: Option<thread::JoinHandle<()>>,
}

impl ThreadCheckerExecutor {
    /// Starts the executor's thread; tasks remain queued until [`Self::run_until_stalled`].
    pub(crate) fn new(name: &'static str) -> Self {
        let (spawns, mut incoming) = futures_mpsc::unbounded();
        let (requests, receiver) = mpsc::channel();
        let thread = thread::Builder::new()
            .name(name.to_owned())
            .spawn(move || {
                THREAD_NAME.set(Some(name));
                let mut pool = LocalPool::new();
                let spawner = pool.spawner();
                // The pool installs the futures that arrive, including those a task it is running
                // queues: a newly spawned task is drained into the pool and polled by the same
                // `run_until_stalled`.
                pool.spawner()
                    .spawn_local(async move {
                        while let Some(future) = incoming.next().await {
                            spawner
                                .spawn_local(future)
                                .expect("the pool lives as long as this thread");
                        }
                    })
                    .expect("the pool lives as long as this thread");
                // Only the owner's `Shutdown` ends the loop; scheduler clones, those held by the
                // tasks included, keep the spawn channel open. The pool then releases its tasks
                // and the futures still queued on that channel, on this thread.
                while let Ok(request) = receiver.recv() {
                    match request {
                        Request::RunUntilStalled(done) => {
                            // A panic of a task is the test's failure: it is handed to the test
                            // thread rather than lost with this one.
                            let result =
                                panic::catch_unwind(AssertUnwindSafe(|| pool.run_until_stalled()));
                            let _ = done.send(result);
                        }
                        Request::Shutdown => break,
                    }
                }
            })
            .expect("the executor's thread starts");
        Self {
            scheduler: ThreadCheckerScheduler { name, spawns },
            requests,
            thread: Some(thread),
        }
    }

    /// Returns a scheduling handle that does not keep this executor alive.
    pub(crate) fn scheduler(&self) -> ThreadCheckerScheduler {
        self.scheduler.clone()
    }

    /// Runs the queued tasks on the executor's thread until none of them can make progress, and
    /// resumes on the calling thread a panic one of them raised.
    pub(crate) async fn run_until_stalled(&self) {
        let (done, result) = oneshot::channel();
        if self.requests.send(Request::RunUntilStalled(done)).is_err() {
            self.scheduler.gone();
        }
        let result = result.await.unwrap_or_else(|_| self.scheduler.gone());
        if let Err(payload) = result {
            panic::resume_unwind(payload);
        }
    }
}

impl Drop for ThreadCheckerExecutor {
    fn drop(&mut self) {
        // Fails only if the thread already panicked, which the join below reports.
        let _ = self.requests.send(Request::Shutdown);
        let thread = self.thread.take().expect("the executor owns its thread");
        if thread.thread().id() == thread::current().id() {
            // The event loop finishes shutting down once this task returns; it cannot join itself.
            return;
        }
        if let Err(payload) = thread.join()
            && !thread::panicking()
        {
            panic::resume_unwind(payload);
        }
    }
}

/// A scheduling handle of a [`ThreadCheckerExecutor`]. It does not keep the executor alive: once
/// the executor is dropped, scheduling panics.
///
/// [`Scheduler::run_task`] only queues the task. [`ThreadCheckerExecutor::run_until_stalled`] runs
/// the queued tasks on that thread until none can make progress. A disposed task is dropped on the
/// executor's thread by the next such run, or when the executor is dropped.
#[derive(Debug, Clone)]
pub(crate) struct ThreadCheckerScheduler {
    name: &'static str,
    spawns: futures_mpsc::UnboundedSender<BoxFuture<'static, ()>>,
}

impl ThreadCheckerScheduler {
    fn spawn(&self, future: BoxFuture<'static, ()>) {
        if self.spawns.unbounded_send(future).is_err() {
            self.gone();
        }
    }

    fn gone(&self) -> ! {
        panic!("the {} thread is gone", self.name)
    }
}

pub(crate) struct ThreadCheckerDisposal(AbortHandle);

impl Disposable for ThreadCheckerDisposal {
    fn dispose(self) {
        self.0.abort();
    }
}

impl SchedulerTypes for ThreadCheckerScheduler {
    type Mode = Shared;
    type Disposal = ThreadCheckerDisposal;
    type Clock = SystemClock;

    fn clock(&self) -> SystemClock {
        SystemClock
    }
}

impl<TC, P> Scheduler<TC, P> for ThreadCheckerScheduler
where
    TC: Send + 'static,
    P: Send + 'static,
{
    fn run_task(
        &self,
        task: Task<TC, P>,
        delay: Option<Duration>,
    ) -> DisposeOnDrop<Self::Disposal> {
        // No test schedules a delayed task here yet; the timer keeps one faithful to the runtime
        // schedulers rather than silently parking forever.
        let (future, abort_handle) =
            abortable(drive(task, delay, self.clock(), |duration| async move {
                async_io::Timer::after(duration).await;
            }));
        self.spawn(future.map(drop).boxed());
        DisposeOnDrop::new(ThreadCheckerDisposal(abort_handle))
    }
}
