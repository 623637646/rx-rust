#[cfg(feature = "async-std-scheduler")]
pub mod async_std_scheduler;
#[cfg(feature = "local-pool-scheduler")]
pub mod local_pool_scheduler;
#[cfg(feature = "thread-pool-scheduler")]
pub mod thread_pool_scheduler;
#[cfg(feature = "tokio-scheduler")]
pub mod tokio_scheduler;

use crate::{disposable::Disposable, utils::types::NecessarySendSync};
use educe::Educe;
#[cfg(feature = "futures")]
use futures::{Stream, stream::StreamExt};
use std::time::{Duration, Instant};

/// Indicates how a recursive scheduling step should continue.
#[derive(Educe)]
#[educe(Debug, Clone, PartialEq, Eq)]
pub enum RecursionAction {
    ContinueAt(Instant),
    ContinueImmediately,
    Stop,
}

/// Core abstraction for driving asynchronous work across runtimes.
/// See <https://reactivex.io/documentation/scheduler.html>
/// This is why the task must be 'static: <https://stackoverflow.com/a/65287449/9315497>
pub trait Scheduler: Clone + NecessarySendSync + 'static {
    fn schedule_future(
        &self,
        future: impl Future<Output = ()> + NecessarySendSync + 'static,
    ) -> impl Disposable + NecessarySendSync + 'static;

    fn sleep(&self, duration: Duration) -> impl Future + NecessarySendSync + 'static;

    fn schedule(
        &self,
        task: impl FnOnce() + NecessarySendSync + 'static,
        delay: Option<Duration>,
    ) -> impl Disposable + NecessarySendSync + 'static {
        let this = self.clone();
        self.schedule_future(async move {
            if let Some(delay) = delay {
                this.sleep(delay).await;
            }
            task();
        })
    }

    fn schedule_recursively(
        &self,
        mut task: impl FnMut(usize) -> RecursionAction + NecessarySendSync + 'static,
        delay: Option<Duration>,
    ) -> impl Disposable + NecessarySendSync + 'static {
        let this = self.clone();
        self.schedule_future(async move {
            if let Some(delay) = delay {
                this.clone().sleep(delay).await;
            }
            let mut count = 0;
            loop {
                match task(count) {
                    RecursionAction::ContinueAt(at) => {
                        if let Some(delay) = at.checked_duration_since(Instant::now()) {
                            this.clone().sleep(delay).await;
                        }
                    }
                    RecursionAction::ContinueImmediately => {}
                    RecursionAction::Stop => break,
                }
                count += 1;
            }
        })
    }

    fn schedule_periodically(
        &self,
        mut task: impl FnMut(usize) -> bool + NecessarySendSync + 'static,
        period: Duration,
        delay: Option<Duration>,
    ) -> impl Disposable + NecessarySendSync + 'static {
        let first = Instant::now() + delay.unwrap_or_default();
        self.schedule_recursively(
            move |count| {
                let stop = task(count);
                if stop {
                    RecursionAction::Stop
                } else {
                    RecursionAction::ContinueAt(first + period * (count as u32 + 1))
                }
            },
            delay,
        )
    }

    #[cfg(feature = "futures")]
    fn schedule_stream<SM>(
        &self,
        mut stream: SM,
        mut result_callback: impl FnMut(Option<SM::Item>) + NecessarySendSync + 'static,
    ) -> impl Disposable + NecessarySendSync + 'static
    where
        SM: Stream + NecessarySendSync + Unpin + 'static,
    {
        self.schedule_future(async move {
            while let Some(item) = stream.next().await {
                result_callback(Some(item));
            }
            result_callback(None);
        })
    }
}
