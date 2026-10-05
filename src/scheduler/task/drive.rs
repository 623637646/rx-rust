use super::{Task, TaskState};
use std::{
    future::{Future, poll_fn},
    pin::{Pin, pin},
    task::{Context, Poll},
    time::{Duration, Instant},
};

/// Runs `task` after `delay` on an async executor: a scheduler spawns the future it returns, as
/// every built-in one does. `sleep` is the runtime's timer (tokio's `time::sleep`, async-std's
/// `task::sleep`, …).
///
/// Between two steps the task yields through [`yield_now`], also for a [`TaskState::SleepUntil`]
/// whose instant has passed (tokio's `Sleep` would be ready at once, without yielding). A scheduler
/// that wants another kind of yield drives the task itself through [`Task::split`] and
/// [`Stepper::step`](super::Stepper::step).
pub async fn drive<TC, P, SF>(
    task: Task<TC, P>,
    delay: Option<Duration>,
    sleep: impl Fn(Duration) -> SF,
) where
    SF: Future<Output = ()>,
{
    let (mut stepper, pinned) = task.split();
    let mut pinned = pin!(pinned);
    if let Some(delay) = delay {
        sleep(delay).await;
    }
    loop {
        match poll_fn(|cx| stepper.step(pinned.as_mut(), cx)).await {
            TaskState::Finished => break,
            TaskState::Yield => yield_now().await,
            TaskState::SleepUntil(at) => match at.checked_duration_since(Instant::now()) {
                Some(delay) if !delay.is_zero() => sleep(delay).await,
                _ => yield_now().await,
            },
        }
    }
}

/// A future that yields to the executor once, then completes: what [`drive`] yields with between
/// two steps. It wakes itself right away, so the task runs again as soon as the other tasks had
/// their turn, without waiting for the runtime's IO or timer driver as a runtime's own
/// `yield_now` may.
pub fn yield_now() -> impl Future<Output = ()> {
    YieldNow(false)
}

struct YieldNow(bool);

impl Future for YieldNow {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        if self.0 {
            Poll::Ready(())
        } else {
            self.0 = true;
            cx.waker().wake_by_ref();
            Poll::Pending
        }
    }
}
