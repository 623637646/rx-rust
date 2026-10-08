mod tests_utils;

use crate::tests_utils::DURATION_10_MS;
use crate::tests_utils::DURATION_100_MS;
use crate::tests_utils::drop_probe::DropCount;
use futures::channel::oneshot;
use rx_rust::disposable::Disposable;
use rx_rust::observable::ObservableExt;
use rx_rust::operators::creating::interval::Interval;
use rx_rust::operators::creating::just::Just;
use rx_rust::scheduler::virtual_time::{VirtualTime, VirtualTimeScheduler};
use rx_rust::scheduler::{Scheduler, SchedulerExt, SchedulerTypes, Task, TaskState};
use rx_rust::thread_mode::mutable::{MutableExt, MutableHelper};
use std::panic::{self, AssertUnwindSafe};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::task::{Poll, Waker};
use std::thread;
use std::time::{Duration, Instant};

const DURATION_1_MS: Duration = Duration::from_millis(1);

/// What the tasks of a test saw: a label and the time of the step, relative to the start.
type Log = Arc<Mutex<Vec<(&'static str, Duration)>>>;

fn log_at(log: &Log, label: &'static str, now: Instant, start: Instant) {
    log.with_mut(|log| log.push((label, now - start)));
}

#[test]
fn test_runs_at_the_exact_boundary() {
    let time = VirtualTime::new();
    let scheduler = time.scheduler();
    let ran = Arc::new(Mutex::new(false));
    let ran_task = ran.clone();

    let _disposal = scheduler.schedule(
        move || {
            ran_task.replace_value(true);
        },
        Some(DURATION_100_MS),
    );
    assert_eq!(time.pending_tasks(), 1);

    time.advance_by(DURATION_100_MS - DURATION_1_MS);
    assert!(!ran.clone_value());
    assert_eq!(time.pending_tasks(), 1);

    time.advance_by(DURATION_1_MS);
    assert!(ran.clone_value());
    assert_eq!(time.pending_tasks(), 0);
}

#[test]
fn test_clock() {
    let time = VirtualTime::new();
    let scheduler = time.scheduler();
    let start = time.now();
    assert_eq!(scheduler.now(), start);

    time.advance_by(DURATION_10_MS);
    assert_eq!(time.now(), start + DURATION_10_MS);
    assert_eq!(scheduler.now(), start + DURATION_10_MS);

    // Nothing is due: the clock still lands on the target.
    time.advance_by(DURATION_100_MS);
    assert_eq!(time.now(), start + DURATION_10_MS + DURATION_100_MS);
}

#[test]
fn test_without_delay_waits_for_advance() {
    let time = VirtualTime::new();
    let ran = Arc::new(Mutex::new(false));
    let ran_task = ran.clone();

    let _disposal = time.scheduler().schedule(
        move || {
            ran_task.replace_value(true);
        },
        None,
    );
    // `run_task` only queues.
    assert!(!ran.clone_value());

    time.advance_by(Duration::ZERO);
    assert!(ran.clone_value());
}

#[test]
fn test_same_instant_in_queue_order() {
    let time = VirtualTime::new();
    let scheduler = time.scheduler();
    let order = Arc::new(Mutex::new(Vec::new()));

    let mut disposals = Vec::new();
    for (label, delay) in [
        ("a", DURATION_10_MS),
        ("b", Duration::ZERO),
        ("c", DURATION_10_MS),
    ] {
        let order = order.clone();
        disposals.push(scheduler.schedule(
            move || order.with_mut(|order| order.push(label)),
            Some(delay),
        ));
    }

    time.advance_by(DURATION_10_MS);
    assert_eq!(order.clone_value(), ["b", "a", "c"]);
}

#[test]
fn test_steps_see_the_instant_they_were_due() {
    let time = VirtualTime::new();
    let start = time.now();
    let log: Log = Arc::new(Mutex::new(Vec::new()));
    let log_task = log.clone();

    // Sleeps 10 ms, then 30 ms, then finishes.
    let _disposal = time.scheduler().schedule_recursively(
        move |count, now| {
            log_at(&log_task, "step", now, start);
            match count {
                0 => TaskState::SleepUntil(now + DURATION_10_MS),
                1 => TaskState::SleepUntil(now + DURATION_10_MS * 3),
                _ => TaskState::Finished,
            }
        },
        Some(DURATION_10_MS),
    );

    // One call crosses every step, each at its own instant.
    time.advance_by(DURATION_100_MS);
    assert_eq!(
        log.clone_value(),
        [
            ("step", DURATION_10_MS),
            ("step", DURATION_10_MS * 2),
            ("step", DURATION_10_MS * 5),
        ]
    );
    assert_eq!(time.pending_tasks(), 0);
}

#[test]
fn test_periodic_rate() {
    let time = VirtualTime::new();
    let start = time.now();
    let log: Log = Arc::new(Mutex::new(Vec::new()));
    let log_task = log.clone();
    let scheduler = time.scheduler();
    let scheduler_task = scheduler.clone();

    let _disposal = scheduler.schedule_periodically(
        move |_| {
            log_at(&log_task, "tick", scheduler_task.now(), start);
            true
        },
        DURATION_10_MS,
        Some(DURATION_10_MS),
    );

    time.advance_by(DURATION_10_MS * 4 - DURATION_1_MS);
    assert_eq!(
        log.clone_value(),
        [
            ("tick", DURATION_10_MS),
            ("tick", DURATION_10_MS * 2),
            ("tick", DURATION_10_MS * 3),
        ]
    );
    time.advance_by(DURATION_1_MS);
    assert_eq!(log.with_ref(Vec::len), 4);
}

#[test]
fn test_yield_lets_others_run_first() {
    let time = VirtualTime::new();
    let scheduler = time.scheduler();
    let order = Arc::new(Mutex::new(Vec::new()));

    let order_a = order.clone();
    let _a = scheduler.schedule_recursively(
        move |count, _| {
            order_a.with_mut(|order| order.push("a"));
            if count == 0 {
                TaskState::Yield
            } else {
                TaskState::Finished
            }
        },
        None,
    );
    let order_b = order.clone();
    let _b = scheduler.schedule(move || order_b.with_mut(|order| order.push("b")), None);

    time.advance_by(Duration::ZERO);
    assert_eq!(order.clone_value(), ["a", "b", "a"]);
}

#[test]
fn test_task_queued_while_advancing() {
    let time = VirtualTime::new();
    let start = time.now();
    let scheduler = time.scheduler();
    let log: Log = Arc::new(Mutex::new(Vec::new()));
    let inner_disposals = Arc::new(Mutex::new(Vec::new()));

    let log_outer = log.clone();
    let scheduler_outer = scheduler.clone();
    let inner_disposals_outer = inner_disposals.clone();
    let _outer = scheduler.schedule(
        move || {
            log_at(&log_outer, "outer", scheduler_outer.now(), start);
            for (label, delay) in [("soon", DURATION_10_MS), ("late", DURATION_100_MS)] {
                let log = log_outer.clone();
                let scheduler = scheduler_outer.clone();
                let disposal = scheduler_outer.schedule(
                    move || log_at(&log, label, scheduler.now(), start),
                    Some(delay),
                );
                inner_disposals_outer.with_mut(|disposals| disposals.push(disposal));
            }
        },
        Some(DURATION_10_MS),
    );

    // `soon` is due within this call, `late` is not.
    time.advance_by(DURATION_10_MS * 3);
    assert_eq!(
        log.clone_value(),
        [("outer", DURATION_10_MS), ("soon", DURATION_10_MS * 2)]
    );
    assert_eq!(time.pending_tasks(), 1);

    time.advance_by(DURATION_100_MS);
    assert_eq!(
        log.with_ref(|log| log[2]),
        ("late", DURATION_10_MS + DURATION_100_MS)
    );
}

#[test]
fn test_dispose_queued() {
    let time = VirtualTime::new();
    let drops = DropCount::new();
    let probe = drops.probe();
    let ran = Arc::new(Mutex::new(false));
    let ran_task = ran.clone();

    let disposal = time.scheduler().schedule(
        move || {
            let _probe = probe;
            ran_task.replace_value(true);
        },
        Some(DURATION_10_MS),
    );
    assert_eq!(time.pending_tasks(), 1);

    disposal.dispose();
    assert_eq!(time.pending_tasks(), 0);
    assert_eq!(drops.get(), 1);

    // Its stale queue entry is skipped.
    time.advance_by(DURATION_100_MS);
    assert!(!ran.clone_value());
}

#[test]
fn test_dispose_by_another_task_while_advancing() {
    let time = VirtualTime::new();
    let scheduler = time.scheduler();
    let ran = Arc::new(Mutex::new(false));
    let ran_task = ran.clone();

    let victim = scheduler.schedule(
        move || {
            ran_task.replace_value(true);
        },
        Some(DURATION_10_MS * 2),
    );
    let victim = Arc::new(Mutex::new(Some(victim)));
    let victim_killer = victim.clone();
    let _killer = scheduler.schedule(
        move || {
            if let Some(victim) = victim_killer.take_value() {
                victim.dispose();
            }
        },
        Some(DURATION_10_MS),
    );

    time.advance_by(DURATION_100_MS);
    assert!(!ran.clone_value());
    assert_eq!(time.pending_tasks(), 0);
}

#[test]
fn test_dispose_while_running() {
    let time = VirtualTime::new();
    let scheduler = time.scheduler();
    let drops = DropCount::new();
    let steps = Arc::new(Mutex::new(0));
    // The task disposes itself during its first step: it is dropped once the step returns,
    // although the step asks for more.
    let own_disposal = Arc::new(Mutex::new(None));

    let probe = drops.probe();
    let steps_task = steps.clone();
    let own_disposal_task = own_disposal.clone();
    let disposal = scheduler.schedule_recursively(
        move |_, now| {
            let _ = &probe;
            steps_task.with_mut(|steps| *steps += 1);
            if let Some(disposal) = own_disposal_task.take_value() {
                Disposable::dispose(disposal);
            }
            TaskState::SleepUntil(now + DURATION_10_MS)
        },
        None,
    );
    own_disposal.replace_value(Some(disposal));

    time.advance_by(DURATION_100_MS);
    assert_eq!(steps.clone_value(), 1);
    assert_eq!(drops.get(), 1);
    assert_eq!(time.pending_tasks(), 0);
}

#[test]
fn test_pending_until_woken() {
    let time = VirtualTime::new();
    let (sender, receiver) = oneshot::channel::<i32>();
    let received = Arc::new(Mutex::new(None));
    let received_task = received.clone();

    let _disposal = time.scheduler().spawn_future(async move {
        let _ = received_task.replace_value(receiver.await.ok());
    });
    time.advance_by(DURATION_10_MS);
    assert_eq!(received.clone_value(), None);
    assert_eq!(time.pending_tasks(), 1);

    sender.send(7).unwrap();
    // Woken: it runs at the next call, at the current time.
    assert_eq!(received.clone_value(), None);
    time.advance_by(Duration::ZERO);
    assert_eq!(received.clone_value(), Some(7));
    assert_eq!(time.pending_tasks(), 0);
}

#[test]
fn test_woken_from_another_thread() {
    let time = VirtualTime::new();
    let (sender, receiver) = oneshot::channel::<i32>();
    let received = Arc::new(Mutex::new(None));
    let received_task = received.clone();

    let _disposal = time.scheduler().spawn_future(async move {
        let _ = received_task.replace_value(receiver.await.ok());
    });
    time.advance_by(Duration::ZERO);

    thread::spawn(move || sender.send(7).unwrap())
        .join()
        .unwrap();
    time.advance_by(Duration::ZERO);
    assert_eq!(received.clone_value(), Some(7));
}

#[test]
fn test_time_from_another_thread() {
    let time = VirtualTime::new();
    let scheduler = time.scheduler();
    time.advance_by(DURATION_100_MS);
    let expected = time.now();

    let seen = thread::spawn(move || scheduler.now()).join().unwrap();
    assert_eq!(seen, expected);
}

#[test]
fn test_task_from_another_thread() {
    let time = VirtualTime::new();
    let scheduler = time.scheduler();
    let ran = Arc::new(Mutex::new(false));
    let ran_task = ran.clone();

    let disposal = thread::spawn(move || {
        scheduler.schedule(
            move || {
                ran_task.replace_value(true);
            },
            Some(DURATION_10_MS),
        )
    })
    .join()
    .unwrap();

    time.advance_by(DURATION_10_MS);
    assert!(ran.clone_value());
    drop(disposal);
}

#[test]
fn test_advance_inside_a_task_panics() {
    let time = Arc::new(VirtualTime::new());
    let time_task = time.clone();
    let _disposal = time
        .scheduler()
        .schedule(move || time_task.advance_by(DURATION_10_MS), None);

    let result = panic::catch_unwind(AssertUnwindSafe(|| time.advance_by(Duration::ZERO)));
    assert!(result.is_err());

    // The clock is usable again, and the task that panicked is gone.
    assert_eq!(time.pending_tasks(), 0);
    time.advance_by(DURATION_10_MS);
}

#[test]
fn test_task_panic() {
    let time = VirtualTime::new();
    let scheduler = time.scheduler();
    let drops = DropCount::new();
    let probe = drops.probe();
    let _panicking = scheduler.schedule(
        move || {
            let _probe = probe;
            panic!("task panic");
        },
        None,
    );
    let ran = Arc::new(Mutex::new(false));
    let ran_task = ran.clone();
    let _later = scheduler.schedule(
        move || {
            ran_task.replace_value(true);
        },
        Some(DURATION_10_MS),
    );

    let result = panic::catch_unwind(AssertUnwindSafe(|| time.advance_by(DURATION_10_MS)));
    let payload = result.unwrap_err();
    assert_eq!(payload.downcast_ref::<&str>(), Some(&"task panic"));
    assert_eq!(drops.get(), 1);

    // The clock stopped at the instant of the panic; the other task is still queued, and runs
    // at the next call.
    assert_eq!(time.pending_tasks(), 1);
    assert!(!ran.clone_value());
    time.advance_by(DURATION_10_MS);
    assert!(ran.clone_value());
}

#[test]
fn test_drop_owner_drops_tasks() {
    let time = VirtualTime::new();
    let scheduler = time.scheduler();
    let drops = DropCount::new();

    // Each task holds a handle of its own scheduler, as an operator's would: no cycle keeps it.
    let probe = drops.probe();
    let scheduler_task = scheduler.clone();
    let _queued = scheduler.schedule(
        move || {
            let _ = (&probe, &scheduler_task);
        },
        Some(DURATION_10_MS),
    );
    let probe = drops.probe();
    let scheduler_task = scheduler.clone();
    let _parked = scheduler.spawn_future(async move {
        let _ = (&probe, &scheduler_task);
        std::future::pending::<()>().await;
    });
    time.advance_by(Duration::ZERO);
    assert_eq!(time.pending_tasks(), 2);

    drop(time);
    assert_eq!(drops.get(), 2);
}

#[test]
#[should_panic(expected = "has been dropped")]
fn test_run_task_after_owner_dropped() {
    let time = VirtualTime::new();
    let scheduler = time.scheduler();
    drop(time);
    let _ = scheduler.run_task(Task::once((), |()| {}), None);
}

#[test]
fn test_disposal_after_owner_dropped() {
    let time = VirtualTime::new();
    let disposal = time.scheduler().schedule(|| {}, Some(DURATION_10_MS));
    drop(time);
    disposal.dispose();
}

#[test]
fn test_finished_task_released_while_disposal_lives() {
    let time = VirtualTime::new();
    let drops = DropCount::new();
    let probe = drops.probe();
    let disposal = time.scheduler().schedule(
        move || {
            let _ = &probe;
        },
        None,
    );

    time.advance_by(Duration::ZERO);
    // The disposal is a handle, not the task's owner.
    assert_eq!(drops.get(), 1);
    drop(disposal);
}

#[test]
fn test_scheduler_is_send_and_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<VirtualTimeScheduler>();
    assert_send_sync::<VirtualTime>();
}

#[test]
fn test_handler_gets_virtual_now() {
    let time = VirtualTime::new();
    let start = time.now();
    let seen = Arc::new(Mutex::new(None));
    let seen_task = seen.clone();

    let _disposal = time.scheduler().run_task(
        Task::new(seen_task, |seen, _, _, now| {
            let _ = seen.replace_value(Some(now));
            Poll::Ready(TaskState::Finished)
        }),
        Some(DURATION_100_MS),
    );
    time.advance_by(DURATION_100_MS * 2);
    assert_eq!(seen.clone_value(), Some(start + DURATION_100_MS));
}

#[test]
fn test_with_operators() {
    let time = VirtualTime::new();
    let scheduler = time.scheduler();
    let values = Arc::new(Mutex::new(Vec::new()));

    let values_observer = values.clone();
    let start = time.now();
    let _subscription = Interval::new(DURATION_10_MS, scheduler.clone(), None)
        .take(3)
        .delay(DURATION_100_MS, scheduler.clone())
        .timestamp(scheduler.clone())
        .subscribe_with_callback(
            move |(value, at): (usize, Instant)| {
                values_observer.with_mut(|values| values.push((value, at - start)));
            },
            |_| {},
        );

    // Emitted at 0, 10 and 20 ms, each delayed by 100 ms.
    time.advance_by(DURATION_100_MS - DURATION_1_MS);
    assert!(values.with_ref(Vec::is_empty));
    time.advance_by(DURATION_10_MS * 2 + DURATION_1_MS);
    assert_eq!(
        values.clone_value(),
        [
            (0, DURATION_100_MS),
            (1, DURATION_100_MS + DURATION_10_MS),
            (2, DURATION_100_MS + DURATION_10_MS * 2),
        ]
    );
}

#[test]
fn test_just_delay_exact() {
    let time = VirtualTime::new();
    let values = Arc::new(Mutex::new(Vec::new()));
    let values_observer = values.clone();
    let _subscription = Just::new(1)
        .delay(DURATION_100_MS, time.scheduler())
        .subscribe_with_callback(
            move |value| values_observer.with_mut(|values| values.push(value)),
            |_| {},
        );

    time.advance_by(DURATION_100_MS - DURATION_1_MS);
    assert!(values.with_ref(Vec::is_empty));
    time.advance_by(DURATION_1_MS);
    assert_eq!(values.clone_value(), [1]);
}

#[test]
fn test_sleep_until_passed_instant_a_few_times() {
    let time = VirtualTime::new();
    let steps = Arc::new(Mutex::new(0));
    let steps_task = steps.clone();

    // Asking for an instant that has passed is a yield point, not an error.
    let _disposal = time.scheduler().schedule_recursively(
        move |count, now| {
            steps_task.with_mut(|steps| *steps += 1);
            if count < 100 {
                TaskState::SleepUntil(now)
            } else {
                TaskState::Finished
            }
        },
        None,
    );
    time.advance_by(Duration::ZERO);
    assert_eq!(steps.clone_value(), 101);
}

#[test]
fn test_stalled_task_panics() {
    let time = VirtualTime::new();
    let drops = DropCount::new();
    let probe = drops.probe();

    // On a real clock this waits for time to move; the virtual one would loop forever.
    let _disposal = time.scheduler().schedule_recursively(
        move |_, now| {
            let _ = &probe;
            TaskState::SleepUntil(now)
        },
        None,
    );

    let result = panic::catch_unwind(AssertUnwindSafe(|| time.advance_by(DURATION_10_MS)));
    let payload = result.unwrap_err();
    let message = payload.downcast_ref::<String>().unwrap();
    assert!(message.contains("already passed"), "{message}");
    assert_eq!(drops.get(), 1);
    assert_eq!(time.pending_tasks(), 0);
}

#[test]
fn test_woken_during_advance_runs_in_the_same_call() {
    let time = VirtualTime::new();
    let start = time.now();
    let scheduler = time.scheduler();
    let (sender, receiver) = oneshot::channel::<i32>();
    let received = Arc::new(Mutex::new(None));

    let received_task = received.clone();
    let scheduler_task = scheduler.clone();
    let _waiting = scheduler.spawn_future(async move {
        let value = receiver.await.ok();
        let _ = received_task.replace_value(value.map(|value| (value, scheduler_task.now())));
    });
    let mut sender = Some(sender);
    let _sending = scheduler.schedule(
        move || {
            let _ = sender.take().unwrap().send(7);
        },
        Some(DURATION_10_MS),
    );

    // The send at 10 ms wakes the waiting task, which runs at 10 ms within this call.
    time.advance_by(DURATION_100_MS);
    assert_eq!(received.clone_value(), Some((7, start + DURATION_10_MS)));
    assert_eq!(time.pending_tasks(), 0);
}

#[test]
fn test_repeated_wakes_run_once() {
    let time = VirtualTime::new();
    let steps = Arc::new(Mutex::new(0));
    let waker: Arc<Mutex<Option<Waker>>> = Arc::new(Mutex::new(None));

    let _disposal = time.scheduler().run_task(
        Task::new(
            (steps.clone(), waker.clone()),
            |(steps, waker), _, cx, _| {
                steps.with_mut(|steps| *steps += 1);
                let _ = waker.replace_value(Some(cx.waker().clone()));
                Poll::Pending
            },
        ),
        None,
    );
    time.advance_by(Duration::ZERO);
    assert_eq!(steps.clone_value(), 1);

    let waker = waker.clone_value().unwrap();
    for _ in 0..5 {
        waker.wake_by_ref();
    }
    time.advance_by(Duration::ZERO);
    assert_eq!(steps.clone_value(), 2);

    // No wake left over.
    time.advance_by(Duration::ZERO);
    assert_eq!(steps.clone_value(), 2);
}

#[test]
fn test_woken_before_its_step_returns() {
    let time = VirtualTime::new();
    let steps = Arc::new(Mutex::new(0));

    // The first step wakes itself and returns `Pending`: the wake is not lost.
    let _disposal = time.scheduler().run_task(
        Task::new(steps.clone(), |steps, _, cx, _| {
            let count = steps.with_mut(|steps| {
                *steps += 1;
                *steps
            });
            if count == 1 {
                cx.waker().wake_by_ref();
                Poll::Pending
            } else {
                Poll::Ready(TaskState::Finished)
            }
        }),
        None,
    );

    time.advance_by(Duration::ZERO);
    assert_eq!(steps.clone_value(), 2);
    assert_eq!(time.pending_tasks(), 0);
}

#[test]
fn test_delay_too_long_never_comes_due() {
    let time = VirtualTime::new();
    let ran = Arc::new(AtomicBool::new(false));
    let ran_task = ran.clone();

    let disposal = time.scheduler().schedule(
        move || ran_task.store(true, Ordering::SeqCst),
        Some(Duration::MAX),
    );
    assert_eq!(time.pending_tasks(), 1);

    time.advance_by(Duration::from_secs(60 * 60 * 24 * 365));
    assert!(!ran.load(Ordering::SeqCst));
    assert_eq!(time.pending_tasks(), 1);

    disposal.dispose();
    assert_eq!(time.pending_tasks(), 0);
}

#[test]
fn test_periodic_delay_too_long_never_comes_due() {
    let time = VirtualTime::new();
    let runs = Arc::new(Mutex::new(0));
    let runs_task = runs.clone();

    let disposal = time.scheduler().schedule_periodically(
        move |_| {
            runs_task.with_mut(|runs| *runs += 1);
            true
        },
        DURATION_10_MS,
        Some(Duration::MAX),
    );
    time.advance_by(Duration::from_secs(60 * 60 * 24 * 365));
    assert_eq!(runs.clone_value(), 0);
    assert_eq!(time.pending_tasks(), 1);

    disposal.dispose();
    assert_eq!(time.pending_tasks(), 0);
}

/// The second run would be due later than an `Instant` can represent: it never comes, so the task
/// finishes after the first one and drops its closure.
#[test]
fn test_periodic_period_too_long_finishes() {
    let time = VirtualTime::new();
    let drops = DropCount::new();
    let probe = drops.probe();
    let runs = Arc::new(Mutex::new(0));
    let runs_task = runs.clone();

    let disposal = time.scheduler().schedule_periodically(
        move |_| {
            let _probe = &probe;
            runs_task.with_mut(|runs| *runs += 1);
            true
        },
        Duration::MAX,
        None,
    );
    assert_eq!(time.pending_tasks(), 1);

    time.advance_by(Duration::ZERO);
    assert_eq!(runs.clone_value(), 1);
    assert_eq!(time.pending_tasks(), 0);
    assert_eq!(drops.get(), 1);

    time.advance_by(Duration::from_secs(60 * 60 * 24 * 365));
    disposal.dispose();
    assert_eq!(runs.clone_value(), 1);
    assert_eq!(drops.get(), 1);
}

#[test]
fn test_advance_too_far_panics_and_keeps_the_clock() {
    let time = VirtualTime::new();
    let start = time.now();

    let result = panic::catch_unwind(AssertUnwindSafe(|| time.advance_by(Duration::MAX)));
    let payload = result.unwrap_err();
    let message = payload.downcast_ref::<String>().unwrap();
    assert!(message.contains("too far"), "{message}");
    assert_eq!(time.now(), start);

    // Not left advancing.
    time.advance_by(DURATION_10_MS);
    assert_eq!(time.now(), start + DURATION_10_MS);
}

/// A task queued from another thread while the clock advances is either run by that call or due
/// after it, never left behind: each step sees the time it was queued at, never a later one.
#[test]
fn test_race_condition_queue_while_advancing() {
    const TASKS: usize = 2_000;
    let time = VirtualTime::new();
    let scheduler = time.scheduler();
    // For each task: the clock just before and just after queueing it, and the time its step saw.
    let records = Arc::new(Mutex::new(Vec::with_capacity(TASKS)));
    let done = Arc::new(AtomicBool::new(false));

    let worker = {
        let records = records.clone();
        let done = done.clone();
        thread::spawn(move || {
            let mut disposals = Vec::with_capacity(TASKS);
            for _ in 0..TASKS {
                let seen = Arc::new(Mutex::new(None));
                let seen_task = seen.clone();
                let before = scheduler.now();
                disposals.push(scheduler.schedule_recursively(
                    move |_, now| {
                        let _ = seen_task.replace_value(Some(now));
                        TaskState::Finished
                    },
                    None,
                ));
                let after = scheduler.now();
                records.with_mut(|records| records.push((before, after, seen)));
            }
            done.store(true, Ordering::SeqCst);
            disposals
        })
    };
    while !done.load(Ordering::SeqCst) {
        time.advance_by(Duration::from_micros(1));
    }
    let _disposals = worker.join().unwrap();
    time.advance_by(Duration::ZERO);

    records.with_ref(|records| {
        assert_eq!(records.len(), TASKS);
        for (before, after, seen) in records {
            let seen = seen.clone_value().expect("every task ran");
            assert!(
                *before <= seen && seen <= *after,
                "queued between {before:?} and {after:?}, ran at {seen:?}"
            );
        }
    });
}
