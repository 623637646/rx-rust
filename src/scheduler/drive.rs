//! [`drive()`]: how a [`Task`] runs on an async executor, the driver every built-in runtime
//! scheduler spawns.

use super::{Clock, Next, Task};
use std::{
    future::{Future, pending, poll_fn},
    pin::{Pin, pin},
    task::{Context, Poll},
    time::{Duration, Instant},
};

/// Runs `task` after `delay` on an async executor: a scheduler spawns the future it returns, as
/// every built-in one does, passing its own [`clock`](super::SchedulerTypes::clock).
/// `sleep` is the runtime's timer (tokio's `time::sleep`, smol's `Timer::after`, …), which must run
/// on that clock.
///
/// A `delay` too long for an [`Instant`] to represent never ends: the task waits, without a timer,
/// until it is disposed, whatever `sleep` would make of that duration (tokio's wakes after some
/// thirty years).
///
/// Every step gets `clock.now()`, and a [`Next::SleepUntil`] is measured against it.
/// Between two steps the task yields through `yield_now`, also for a `SleepUntil` whose instant
/// has passed (tokio's `Sleep` would be ready at once, without yielding). A scheduler that wants
/// another kind of yield drives the task itself through [`Task::split`] and
/// [`Stepper::step`](super::Stepper::step).
pub async fn drive<TC, P, C, SF>(
    task: Task<TC, P>,
    delay: Option<Duration>,
    clock: C,
    sleep: impl Fn(Duration) -> SF,
) where
    C: Clock,
    SF: Future<Output = ()>,
{
    let (mut stepper, pinned) = task.split();
    let mut pinned = pin!(pinned);
    if let Some(delay) = delay {
        if clock.now().checked_add(delay).is_none() {
            return pending().await;
        }
        sleep(delay).await;
    }
    // How long until `at`, or `None` when it has come.
    let delay_until = |at: Instant| {
        at.checked_duration_since(clock.now())
            .filter(|delay| !delay.is_zero())
    };
    loop {
        // The `poll_fn` is only there for the `Context` the step takes; it never waits.
        let next = poll_fn(|cx| Poll::Ready(stepper.step(pinned.as_mut(), cx, clock.now()))).await;
        match next {
            Next::Finished => break,
            Next::Yield => yield_now().await,
            Next::SleepUntil(at) => match delay_until(at) {
                Some(delay) => sleep(delay).await,
                None => yield_now().await,
            },
            Next::Pending => until_woken(pending()).await,
            Next::PendingUntil(at) => match delay_until(at) {
                Some(delay) => until_woken(sleep(delay)).await,
                None => yield_now().await,
            },
        }
    }
}

/// Waits for `sleep`, or for the task to be woken by anything else, whichever comes first: a
/// [`Next::Pending`] waits with a `sleep` that never ends.
///
/// The handler registered the task's waker, and the sleep registers it too: whatever wakes the
/// task — the handler's event, the timer, or a spurious wake — polls this again, which ends the
/// wait. A wake that came while the handler was still running polls it again at once, as the
/// executor re-polls a task woken during its own poll.
async fn until_woken(sleep: impl Future<Output = ()>) {
    let mut sleep = pin!(sleep);
    let mut polled = false;
    poll_fn(|cx| {
        if std::mem::replace(&mut polled, true) {
            return Poll::Ready(());
        }
        sleep.as_mut().poll(cx)
    })
    .await;
}

/// A future that yields to the executor once, then completes: what [`drive`] yields with between
/// two steps. It wakes itself right away, so the task runs again as soon as the other tasks had
/// their turn, without waiting for the runtime's IO or timer driver as a runtime's own
/// `yield_now` may.
pub(crate) fn yield_now() -> impl Future<Output = ()> {
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
