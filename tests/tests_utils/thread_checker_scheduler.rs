use educe::Educe;
use futures::channel::oneshot;
use futures::executor::LocalPool;
use futures::future::{BoxFuture, abortable};
use futures::stream::AbortHandle;
use futures::task::LocalSpawnExt;
use rx_rust::disposable::Disposable;
use rx_rust::disposable::dispose_on_drop::DisposeOnDrop;
use rx_rust::scheduler::{Scheduler, SchedulerTypes, Task, drive};
use rx_rust::thread_mode::Shared;
use std::cell::Cell;
use std::panic::{self, AssertUnwindSafe};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

thread_local! {
    static THREAD_NAME: Cell<Option<&'static str>> = const { Cell::new(None) };
}

/// The name of the [`ThreadCheckerScheduler`] whose thread this is, `None` on any other thread.
pub(crate) fn get_thread_name() -> Option<&'static str> {
    THREAD_NAME.with(|name| name.get())
}

enum Command {
    Spawn(BoxFuture<'static, ()>),
    RunUntilStalled(oneshot::Sender<thread::Result<()>>),
}

/// A scheduler with a thread of its own, named for [`get_thread_name`], that runs its tasks only
/// when the test asks for it.
///
/// [`Scheduler::run_task`] only queues the task. [`Self::run_until_stalled`] then runs, on that
/// thread, every queued task until none can make progress, and returns once they have. So the test
/// decides when the scheduler's thread runs relative to its own, instead of racing it, and waits
/// for exactly the work it started instead of sleeping.
///
/// A disposed task is dropped, on the scheduler's thread, by the next [`Self::run_until_stalled`].
#[derive(Educe)]
#[educe(Debug, Clone)]
pub(crate) struct ThreadCheckerScheduler {
    name: &'static str,
    #[educe(Debug(ignore))]
    commands: mpsc::Sender<Command>,
}

impl ThreadCheckerScheduler {
    pub(crate) fn new(name: &'static str) -> Self {
        let (commands, receiver) = mpsc::channel();
        let thread = thread::Builder::new().name(name.to_owned());
        thread
            .spawn(move || {
                THREAD_NAME.with(|thread_name| thread_name.set(Some(name)));
                let mut pool = LocalPool::new();
                let spawner = pool.spawner();
                // Ends once every handle on the scheduler is gone.
                for command in receiver {
                    match command {
                        Command::Spawn(future) => spawner
                            .spawn_local(future)
                            .expect("the pool lives as long as this thread"),
                        Command::RunUntilStalled(done) => {
                            // A panic of a task is the test's failure: it is handed to the test
                            // thread rather than lost with this one.
                            let result =
                                panic::catch_unwind(AssertUnwindSafe(|| pool.run_until_stalled()));
                            let _ = done.send(result);
                        }
                    }
                }
            })
            .expect("the scheduler's thread starts");
        Self { name, commands }
    }

    /// Runs the queued tasks on the scheduler's thread until none of them can make progress, and
    /// resumes on the calling thread a panic one of them raised.
    pub(crate) async fn run_until_stalled(&self) {
        let (sender, receiver) = oneshot::channel();
        self.send(Command::RunUntilStalled(sender));
        let result = receiver
            .await
            .unwrap_or_else(|_| panic!("the {} thread is gone", self.name));
        if let Err(payload) = result {
            panic::resume_unwind(payload);
        }
    }

    fn send(&self, command: Command) {
        if self.commands.send(command).is_err() {
            panic!("the {} thread is gone", self.name);
        }
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
        let future = drive(task, delay, self.clone(), |duration| async move {
            async_io::Timer::after(duration).await;
        });
        let (abortable, abort_handle) = abortable(future);
        self.send(Command::Spawn(Box::pin(async move {
            let _ = abortable.await;
        })));
        DisposeOnDrop::new(ThreadCheckerDisposal(abort_handle))
    }
}
