#[cfg(feature = "async-std-scheduler")]
pub mod async_std_scheduler;
#[cfg(feature = "local-pool-scheduler")]
pub mod local_pool_scheduler;
#[cfg(feature = "thread-pool-scheduler")]
pub mod thread_pool_scheduler;
#[cfg(feature = "tokio-scheduler")]
pub mod tokio_scheduler;

use crate::{
    disposable::{Disposable, bound_drop_disposal::BoundDropDisposal},
    utils::types::NecessarySend,
};
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
pub trait Scheduler {
    fn spawn_future<F>(
        &self,
        future: F,
    ) -> BoundDropDisposal<impl Disposable + NecessarySend + 'static + use<Self, F>>
    where
        F: Future<Output = ()> + NecessarySend + 'static;

    fn sleep(&self, duration: Duration) -> impl Future + NecessarySend + 'static + use<Self>;

    fn schedule<F>(
        &self,
        task: F,
        delay: Option<Duration>,
    ) -> BoundDropDisposal<impl Disposable + NecessarySend + 'static + use<Self, F>>
    where
        F: FnOnce() + NecessarySend + 'static,
    {
        let delay = delay.map(|duration| self.sleep(duration));
        self.spawn_future(async move {
            if let Some(delay) = delay {
                delay.await;
            }
            task()
        })
    }

    fn schedule_recursively<F>(
        &self,
        mut task: F,
        delay: Option<Duration>,
    ) -> BoundDropDisposal<impl Disposable + NecessarySend + 'static + use<Self, F>>
    where
        F: FnMut(usize) -> RecursionAction + NecessarySend + 'static,
        Self: Clone + NecessarySend + 'static,
    {
        let delay = delay.map(|duration| self.sleep(duration));
        let self_cloned = self.clone();
        self.spawn_future(async move {
            if let Some(delay) = delay {
                delay.await;
            }
            let mut count = 0;
            loop {
                match task(count) {
                    RecursionAction::ContinueAt(at) => {
                        if let Some(delay) = at.checked_duration_since(Instant::now()) {
                            self_cloned.sleep(delay).await;
                        }
                    }
                    RecursionAction::ContinueImmediately => {}
                    RecursionAction::Stop => break,
                }
                count += 1;
            }
        })
    }

    fn schedule_periodically<F>(
        &self,
        mut task: F,
        period: Duration,
        delay: Option<Duration>,
    ) -> BoundDropDisposal<impl Disposable + NecessarySend + 'static + use<Self, F>>
    where
        F: FnMut(usize) -> bool + NecessarySend + 'static,
        Self: Clone + NecessarySend + 'static,
    {
        let mut next_time = Instant::now() + delay.unwrap_or_default();
        self.schedule_recursively(
            move |count| {
                let r#continue = task(count);
                if r#continue {
                    next_time += period;
                    RecursionAction::ContinueAt(next_time)
                } else {
                    RecursionAction::Stop
                }
            },
            delay,
        )
    }

    #[cfg(feature = "futures")]
    fn schedule_stream<SM, F>(
        &self,
        stream: SM,
        mut result_callback: F,
    ) -> BoundDropDisposal<impl Disposable + NecessarySend + 'static + use<Self, SM, F>>
    where
        SM: Stream + NecessarySend + 'static,
        F: FnMut(Option<SM::Item>) + NecessarySend + 'static,
    {
        self.spawn_future(async move {
            let mut stream = std::pin::pin!(stream);
            while let Some(item) = stream.next().await {
                result_callback(Some(item));
            }
            result_callback(None);
        })
    }
}
