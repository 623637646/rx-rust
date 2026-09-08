//! Tests of [`SerializedDelivery`] on its own, without any operator around it.
//!
//! They drive the state machine directly, so they can check what only this module promises: the
//! order in which queued and re-entrant events reach the observer, when the observer and the
//! resources are dropped, and that nothing is ever dropped nor notified while the state is locked.
//!
//! The last one is checked with values whose `Drop` re-enters the delivery: a drop that ran under
//! the lock would deadlock, or panic in single-threaded builds, instead of recording its note.

mod tests_utils;

use crate::tests_utils::drop_probe::{DropCallback, DropProbe};
use rx_rust::utils::mutable::MutableExt;
use rx_rust::utils::mutable::MutableHelper;
use rx_rust::{
    observer::{BoxedObserverExt, Observer, Termination, boxed_observer::BoxedObserver},
    utils::{
        mutable::Mutable,
        pending_events::EventBatch,
        serialized_delivery::{
            DeliveryStopped, SerializedDelivery, UpdateOutcome, WeakSerializedDelivery,
        },
        types::{MaybeSend, Shared},
    },
};

type TestError = &'static str;
const ERROR: TestError = "boom";

type TestObserver = BoxedObserver<'static, Value, TestError>;
type TestDelivery = SerializedDelivery<Value, TestError, TestObserver, Resources>;
type WeakTestDelivery = WeakSerializedDelivery<Value, TestError, TestObserver, Resources>;
type TestOutcome<R> = UpdateOutcome<Value, TestError, R>;

// MARK: - The log the tests assert on

/// One thing that happened to the observer, to the resources, or to a value.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Record {
    Next(i32),
    Termination(Termination<TestError>),
    ObserverDropped,
    ResourcesDropped,
    /// Written by a [`DropCallback`], which is how a test observes the drop of a value.
    Note(&'static str),
}

type Log = Shared<Mutable<Vec<Record>>>;

fn new_log() -> Log {
    Shared::new(Mutable::new(Vec::new()))
}

fn record(log: &Log, record: Record) {
    log.with_mut(|values| values.push(record));
}

fn records(log: &Log) -> Vec<Record> {
    log.clone_value()
}

/// The values that reached the observer, which is what most tests assert on.
fn values(log: &Log) -> Vec<i32> {
    records(log)
        .into_iter()
        .filter_map(|record| match record {
            Record::Next(value) => Some(value),
            _ => None,
        })
        .collect()
}

/// A callback that records `record_value` when whatever owns it is dropped.
fn record_on_drop(log: &Log, record_value: Record) -> DropCallback {
    let log = log.clone();
    Box::new(move || record(&log, record_value))
}

/// A callback that only notes that whatever owned it was dropped.
fn note(log: &Log, text: &'static str) -> DropCallback {
    record_on_drop(log, Record::Note(text))
}

/// A callback that notes the drop and then re-enters the delivery, which only works when the drop
/// runs with the state unlocked.
///
/// The value it sends carries no callback of its own, so re-entering never recurses any further.
fn note_and_reenter(handle: &DeliveryHandle, log: &Log, text: &'static str) -> DropCallback {
    let handle = handle.clone();
    let log = log.clone();
    Box::new(move || {
        record(&log, Record::Note(text));
        // Locking here would deadlock, or panic in single-threaded builds, if this drop ran while
        // the state was locked.
        handle.get().send(next(99));
    })
}

// MARK: - What the delivery carries

/// A value of the tested stream, numbered so that the tests can tell one from another.
///
/// The probe it embeds is what reports the drop. A value doubles as the probe handed to
/// [`UpdateOutcome::with_drop_outside`] and as the capture of an update that never runs, so that
/// every drop this module promises is observed the same way.
struct Value {
    number: i32,
    probe: DropProbe,
}

impl Value {
    fn new(number: i32) -> Self {
        Self {
            number,
            probe: DropProbe::new(),
        }
    }

    fn on_drop(mut self, callback: DropCallback) -> Self {
        self.probe = self.probe.on_drop(callback);
        self
    }
}

/// What the host owns alongside the observer: a model, and a probe reporting its drop.
struct Resources {
    model: i32,
    probe: DropProbe,
}

impl Resources {
    fn new(model: i32, log: &Log) -> Self {
        Self {
            model,
            probe: DropProbe::new().on_drop(record_on_drop(log, Record::ResourcesDropped)),
        }
    }

    /// Runs `callback` once the drop of the resources was recorded.
    fn on_drop(&mut self, callback: DropCallback) {
        self.probe.also_on_drop(callback);
    }
}

// MARK: - Reaching the delivery from what it owns

/// A handle a test can hand to a callback before the delivery it points at exists.
///
/// The observer and the values are owned by the delivery, so they can only hold a weak handle to
/// it, filled in once it was built.
#[derive(Clone)]
struct DeliveryHandle(Shared<Mutable<Option<WeakTestDelivery>>>);

impl DeliveryHandle {
    fn new() -> Self {
        Self(Shared::new(Mutable::new(None)))
    }

    fn of(delivery: &TestDelivery) -> Self {
        let handle = Self::new();
        handle.install(delivery);
        handle
    }

    fn install(&self, delivery: &TestDelivery) {
        self.0.replace_value(Some(delivery.downgrade()));
    }

    fn get(&self) -> TestDelivery {
        self.0
            .clone_value()
            .expect("the delivery is installed before anything can reach it")
            .upgrade()
            .expect("the tests keep a handle while their callbacks run")
    }
}

// MARK: - The observer and how a test builds one

/// An observer that records every event and every drop, and runs the hooks of its test.
///
/// Each hook receives the delivery it is running in, so it can send, update, or stop from inside a
/// notification, which is what re-entrancy looks like to this module.
struct RecordingObserver<FN, FT> {
    /// Never read: it is here for its drop, and declared first so that the drop of the observer
    /// is recorded before the hooks it owns are dropped, which is the order a hand-written `Drop`
    /// gave.
    _probe: DropProbe,
    log: Log,
    handle: DeliveryHandle,
    on_next: FN,
    on_termination: Option<FT>,
}

impl<FN, FT> Observer<Value, TestError> for RecordingObserver<FN, FT>
where
    FN: FnMut(&TestDelivery, i32),
    FT: FnOnce(&TestDelivery, &Termination<TestError>),
{
    fn on_next(&mut self, value: Value) {
        let number = value.number;
        record(&self.log, Record::Next(number));
        let delivery = self.handle.get();
        (self.on_next)(&delivery, number);
    }

    fn on_termination(mut self, termination: Termination<TestError>) {
        record(&self.log, Record::Termination(termination.clone()));
        let delivery = self.handle.get();
        let hook = self
            .on_termination
            .take()
            .expect("an observer is terminated at most once");
        hook(&delivery, &termination);
        // `self` is dropped here, which records the drop of the observer after its termination.
    }
}

type NoNextHook = fn(&TestDelivery, i32);
type NoTerminationHook = fn(&TestDelivery, &Termination<TestError>);

/// Builds an idle delivery recording into `log`, with no hook and a model of `0`.
fn builder(log: &Log) -> Builder<NoNextHook, NoTerminationHook> {
    Builder {
        log: log.clone(),
        handle: DeliveryHandle::new(),
        model: 0,
        on_next: |_delivery, _number| {},
        on_termination: |_delivery, _termination| {},
        on_observer_drop: None,
    }
}

struct Builder<FN, FT> {
    log: Log,
    handle: DeliveryHandle,
    model: i32,
    on_next: FN,
    on_termination: FT,
    on_observer_drop: Option<DropCallback>,
}

impl<FN, FT> Builder<FN, FT> {
    /// The handle of the delivery to be built, for a callback that has to be made before it.
    fn handle(&self) -> DeliveryHandle {
        self.handle.clone()
    }

    fn model(mut self, model: i32) -> Self {
        self.model = model;
        self
    }

    fn on_next<F>(self, on_next: F) -> Builder<F, FT>
    where
        F: FnMut(&TestDelivery, i32),
    {
        Builder {
            log: self.log,
            handle: self.handle,
            model: self.model,
            on_next,
            on_termination: self.on_termination,
            on_observer_drop: self.on_observer_drop,
        }
    }

    fn on_termination<F>(self, on_termination: F) -> Builder<FN, F>
    where
        F: FnOnce(&TestDelivery, &Termination<TestError>),
    {
        Builder {
            log: self.log,
            handle: self.handle,
            model: self.model,
            on_next: self.on_next,
            on_termination,
            on_observer_drop: self.on_observer_drop,
        }
    }

    fn on_observer_drop(mut self, callback: DropCallback) -> Self {
        self.on_observer_drop = Some(callback);
        self
    }

    fn build(self) -> TestDelivery
    where
        FN: FnMut(&TestDelivery, i32) + MaybeSend + 'static,
        FT: FnOnce(&TestDelivery, &Termination<TestError>) + MaybeSend + 'static,
    {
        let handle = self.handle.clone();
        let mut probe =
            DropProbe::new().on_drop(record_on_drop(&self.log, Record::ObserverDropped));
        if let Some(callback) = self.on_observer_drop {
            probe.also_on_drop(callback);
        }
        let observer: TestObserver = RecordingObserver {
            _probe: probe,
            log: self.log.clone(),
            handle: self.handle,
            on_next: self.on_next,
            on_termination: Some(self.on_termination),
        }
        .into_boxed();
        let resources = Resources::new(self.model, &self.log);
        let delivery = SerializedDelivery::idle(observer, resources);
        handle.install(&delivery);
        delivery
    }
}

// MARK: - Shorthands for the batches the tests send

fn next(number: i32) -> EventBatch<Value, TestError> {
    EventBatch::Next(Value::new(number))
}

fn next_batch(numbers: impl IntoIterator<Item = i32>) -> EventBatch<Value, TestError> {
    EventBatch::NextBatch(numbers.into_iter().map(Value::new).collect())
}

fn completed() -> EventBatch<Value, TestError> {
    EventBatch::Termination(Termination::Completed)
}

fn errored() -> EventBatch<Value, TestError> {
    EventBatch::Termination(Termination::Error(ERROR))
}

fn next_batch_and_completed(
    numbers: impl IntoIterator<Item = i32>,
) -> EventBatch<Value, TestError> {
    EventBatch::NextBatchAndTermination(
        numbers.into_iter().map(Value::new).collect(),
        Termination::Completed,
    )
}

// MARK: - Delivering events

#[test]
fn send_next_delivers_to_the_parked_observer() {
    let log = new_log();
    let delivery = builder(&log).build();

    assert!(delivery.send(next(1)));
    assert_eq!(records(&log), [Record::Next(1)]);

    // The observer is parked back after the delivery, so the next value starts another one.
    assert!(delivery.send(next(2)));
    assert_eq!(values(&log), [1, 2]);
}

#[test]
fn send_delivers_a_batch_in_order() {
    let log = new_log();
    let delivery = builder(&log).build();

    assert!(delivery.send(next_batch([1, 2, 3])));
    assert_eq!(values(&log), [1, 2, 3]);
}

#[test]
fn a_termination_is_notified_before_the_resources_are_dropped() {
    let log = new_log();
    let delivery = builder(&log).build();

    assert!(delivery.send(completed()));
    assert_eq!(
        records(&log),
        [
            Record::Termination(Termination::Completed),
            Record::ObserverDropped,
            Record::ResourcesDropped,
        ]
    );
}

#[test]
fn a_batch_delivers_its_values_before_its_error_termination() {
    let log = new_log();
    let delivery = builder(&log).build();

    assert!(delivery.send(EventBatch::NextBatchAndTermination(
        vec![Value::new(1), Value::new(2)],
        Termination::Error(ERROR),
    )));
    assert_eq!(
        records(&log),
        [
            Record::Next(1),
            Record::Next(2),
            Record::Termination(Termination::Error(ERROR)),
            Record::ObserverDropped,
            Record::ResourcesDropped,
        ]
    );
}

#[test]
fn a_next_and_termination_batch_delivers_its_value_first() {
    let log = new_log();
    let delivery = builder(&log).build();

    assert!(delivery.send(EventBatch::NextAndTermination(
        Value::new(1),
        Termination::Completed,
    )));
    assert_eq!(
        records(&log),
        [
            Record::Next(1),
            Record::Termination(Termination::Completed),
            Record::ObserverDropped,
            Record::ResourcesDropped,
        ]
    );
}

#[test]
fn an_empty_batch_is_accepted_and_leaves_the_delivery_idle() {
    let log = new_log();
    let delivery = builder(&log).build();

    assert!(delivery.send(EventBatch::NextBatch(vec![])));
    assert_eq!(records(&log), []);

    // The observer was never taken out of the state, so it is still there for the next value.
    assert!(delivery.send(next(1)));
    assert_eq!(values(&log), [1]);
}

#[test]
fn an_empty_batch_sent_from_on_next_is_accepted_as_well() {
    let log = new_log();
    let delivery = builder(&log)
        .on_next(|delivery, number| {
            if number == 1 {
                assert!(delivery.send(EventBatch::NextBatch(vec![])));
            }
        })
        .build();

    assert!(delivery.send(next_batch([1, 2])));
    assert_eq!(values(&log), [1, 2]);
}

// MARK: - Rejecting events

#[test]
fn events_sent_after_a_termination_are_rejected() {
    let log = new_log();
    let delivery = builder(&log).build();

    assert!(delivery.send(completed()));
    assert!(!delivery.send(next(1)));
    assert!(!delivery.send(errored()));
    assert!(!delivery.send(EventBatch::NextBatch(vec![])));
    assert_eq!(
        records(&log),
        [
            Record::Termination(Termination::Completed),
            Record::ObserverDropped,
            Record::ResourcesDropped,
        ]
    );
}

#[test]
fn events_sent_after_stop_are_rejected() {
    let log = new_log();
    let delivery = builder(&log).build();

    delivery.stop();
    assert!(!delivery.send(next(1)));
    assert!(!delivery.send(completed()));
    assert_eq!(
        records(&log),
        [Record::ObserverDropped, Record::ResourcesDropped]
    );
}

#[test]
fn rejected_events_are_dropped_outside_the_lock() {
    let log = new_log();
    let delivery = builder(&log).build();
    assert!(delivery.send(completed()));

    let rejected = Value::new(1).on_drop(note_and_reenter(
        &DeliveryHandle::of(&delivery),
        &log,
        "rejected dropped",
    ));
    assert!(!delivery.send(EventBatch::Next(rejected)));

    // The note proves the rejected value was dropped, and the delivery it re-entered from that drop
    // rejected what it sent instead of deadlocking on the lock of `send`.
    assert_eq!(
        records(&log)[3..],
        [Record::Note("rejected dropped")],
        "the rejected value must be dropped, outside the lock"
    );
}

#[test]
fn events_sent_from_on_termination_are_rejected() {
    let log = new_log();
    let delivery = builder(&log)
        .on_termination(|delivery, _termination| {
            // The state is stopped before the terminal notification, so nothing can follow it.
            assert!(!delivery.send(next(1)));
            assert_eq!(
                delivery.update(|_resources| UpdateOutcome::empty()),
                Err(DeliveryStopped)
            );
        })
        .build();

    assert!(delivery.send(completed()));
    assert_eq!(values(&log), []);
}

// MARK: - Stopping

#[test]
fn stop_drops_the_observer_without_notifying_it_and_is_idempotent() {
    let log = new_log();
    let delivery = builder(&log).build();

    delivery.stop();
    assert_eq!(
        records(&log),
        [Record::ObserverDropped, Record::ResourcesDropped]
    );

    delivery.stop();
    assert_eq!(
        records(&log),
        [Record::ObserverDropped, Record::ResourcesDropped],
        "stopping again must drop nothing more"
    );
}

#[test]
fn stop_drops_the_observer_and_the_resources_outside_the_lock() {
    let log = new_log();
    let builder = builder(&log);
    let handle = builder.handle();
    let delivery = builder
        .on_observer_drop(note_and_reenter(&handle, &log, "observer dropped"))
        .build();
    delivery
        .update(|resources| {
            resources.on_drop(note_and_reenter(&handle, &log, "resources dropped"));
            UpdateOutcome::empty()
        })
        .expect("the delivery is idle");

    delivery.stop();

    // Both drops re-entered the delivery, so neither of them ran while the state was locked.
    assert_eq!(
        records(&log),
        [
            Record::ObserverDropped,
            Record::Note("observer dropped"),
            Record::ResourcesDropped,
            Record::Note("resources dropped"),
        ]
    );
}

#[test]
fn stop_from_on_next_drops_the_values_that_are_still_queued() {
    let log = new_log();
    let delivery = builder(&log)
        .on_next(|delivery, number| {
            if number == 2 {
                delivery.stop();
            }
        })
        .build();

    assert!(delivery.send(EventBatch::NextBatch(vec![
        Value::new(1),
        Value::new(2),
        Value::new(3).on_drop(note(&log, "3 dropped")),
    ])));

    assert_eq!(values(&log), [1, 2]);
    assert_eq!(
        records(&log)[2..],
        [
            Record::Note("3 dropped"),
            Record::ResourcesDropped,
            // The loop drops the observer it holds only once it sees the stopped state.
            Record::ObserverDropped,
        ]
    );
    assert!(!delivery.send(next(4)));
}

#[test]
fn stop_from_on_next_suppresses_the_queued_termination() {
    let log = new_log();
    let delivery = builder(&log)
        .on_next(|delivery, number| {
            if number == 1 {
                delivery.stop();
            }
        })
        .build();

    assert!(delivery.send(next_batch_and_completed([1, 2])));
    assert_eq!(
        records(&log),
        [
            Record::Next(1),
            Record::ResourcesDropped,
            Record::ObserverDropped,
        ],
        "the observer must be dropped instead of being terminated"
    );
}

// MARK: - Re-entrant sends

#[test]
fn a_value_sent_from_on_next_is_delivered_after_the_queued_ones() {
    let log = new_log();
    let log_of_observer = log.clone();
    let delivery = builder(&log)
        .on_next(move |delivery, number| {
            if number == 1 {
                assert!(delivery.send(next(10)));
                // The running loop picks it up: it is not delivered from inside this call.
                assert_eq!(values(&log_of_observer), [1]);
            }
        })
        .build();

    assert!(delivery.send(next_batch([1, 2, 3])));
    assert_eq!(values(&log), [1, 2, 3, 10]);
}

#[test]
fn a_termination_sent_from_on_next_is_delivered_after_the_queued_values() {
    let log = new_log();
    let delivery = builder(&log)
        .on_next(|delivery, number| {
            if number == 1 {
                assert!(delivery.send(completed()));
                // Nothing can be queued after the termination, even before it is delivered.
                assert!(!delivery.send(next(10)));
            }
        })
        .build();

    assert!(delivery.send(next_batch([1, 2, 3])));
    assert_eq!(
        records(&log),
        [
            Record::Next(1),
            Record::Next(2),
            Record::Next(3),
            Record::Termination(Termination::Completed),
            Record::ObserverDropped,
            Record::ResourcesDropped,
        ]
    );
}

// MARK: - `update`

#[test]
fn update_without_events_updates_the_model_and_returns_its_result() {
    let log = new_log();
    let delivery = builder(&log).model(1).build();

    assert_eq!(
        delivery.update(|resources| {
            resources.model += 10;
            UpdateOutcome::new(resources.model)
        }),
        Ok(11)
    );
    assert_eq!(
        delivery.update(|resources| UpdateOutcome::new(resources.model)),
        Ok(11)
    );
    assert_eq!(records(&log), [], "an update alone delivers nothing");
}

#[test]
fn update_runs_while_a_delivery_is_running() {
    let log = new_log();
    let delivery = builder(&log)
        .on_next(|delivery, number| {
            assert_eq!(
                delivery.update(|resources| {
                    resources.model += number;
                    UpdateOutcome::new(resources.model)
                }),
                Ok(number * (number + 1) / 2)
            );
        })
        .build();

    assert!(delivery.send(next_batch([1, 2, 3])));
    assert_eq!(
        delivery.update(|resources| UpdateOutcome::new(resources.model)),
        Ok(6)
    );
}

#[test]
fn update_after_a_termination_returns_delivery_stopped() {
    let log = new_log();
    let delivery = builder(&log).build();

    assert!(delivery.send(completed()));
    assert_eq!(
        delivery.update(|resources| UpdateOutcome::new(resources.model)),
        Err(DeliveryStopped)
    );
}

#[test]
fn a_stopped_update_does_not_run_and_is_dropped_outside_the_lock() {
    let log = new_log();
    let delivery = builder(&log).build();
    delivery.stop();

    let probe = Value::new(0).on_drop(note_and_reenter(
        &DeliveryHandle::of(&delivery),
        &log,
        "update dropped",
    ));
    let result: Result<(), _> = delivery.update(move |_resources| -> TestOutcome<()> {
        drop(probe); // Never runs: it is what makes the update own the probe.
        panic!("the update must not run once the delivery has stopped");
    });

    assert_eq!(result, Err(DeliveryStopped));
    assert_eq!(
        records(&log)[2..],
        [Record::Note("update dropped")],
        "the update must be dropped, outside the lock"
    );
}

#[test]
fn update_delivers_the_events_of_its_outcome() {
    let log = new_log();
    let delivery = builder(&log).build();

    assert_eq!(
        delivery.update(|resources| {
            resources.model = 1;
            UpdateOutcome::new("first").with_next_event(Value::new(1))
        }),
        Ok("first")
    );
    assert_eq!(
        delivery.update(|_resources| {
            UpdateOutcome::empty()
                .with_events(EventBatch::NextBatch(vec![Value::new(2), Value::new(3)]))
        }),
        Ok(())
    );
    assert_eq!(values(&log), [1, 2, 3]);
}

#[test]
fn update_terminates_before_dropping_what_it_asked_to_drop_outside() {
    let log = new_log();
    let delivery = builder(&log).build();

    let result = delivery.update(|resources| {
        resources.model = 1;
        UpdateOutcome::empty()
            .with_drop_outside(Value::new(0).on_drop(note(&log, "dropped outside")))
            .with_next_and_termination_events(Value::new(1), Termination::Error(ERROR))
    });

    assert_eq!(result, Ok(()));
    assert_eq!(
        records(&log),
        [
            Record::Next(1),
            Record::Termination(Termination::Error(ERROR)),
            Record::ObserverDropped,
            Record::ResourcesDropped,
            // Dropped last: after the events it was queued with were delivered.
            Record::Note("dropped outside"),
        ]
    );
}

#[test]
fn what_an_update_drops_outside_is_dropped_with_the_lock_released() {
    let log = new_log();
    let delivery = builder(&log).build();
    let probe = Value::new(0).on_drop(note_and_reenter(
        &DeliveryHandle::of(&delivery),
        &log,
        "dropped outside",
    ));

    let result = delivery.update(move |_resources| {
        UpdateOutcome::empty()
            .with_drop_outside(probe)
            .with_next_event(Value::new(1))
    });

    assert_eq!(result, Ok(()));
    assert_eq!(
        records(&log),
        [
            Record::Next(1),
            Record::Note("dropped outside"),
            // The delivery is parked again by then, so what that drop sent is delivered.
            Record::Next(99),
        ]
    );
}

#[test]
fn update_with_a_termination_event_stops_the_delivery() {
    let log = new_log();
    let delivery = builder(&log).build();

    assert_eq!(
        delivery.update(|_resources| {
            UpdateOutcome::empty().with_termination_event(Termination::Completed)
        }),
        Ok(())
    );
    assert_eq!(
        delivery.update(|_resources| UpdateOutcome::empty()),
        Err(DeliveryStopped)
    );
    assert!(!delivery.send(next(1)));
}

#[test]
fn update_from_on_next_queues_its_events_after_the_pending_ones() {
    let log = new_log();
    let delivery = builder(&log)
        .on_next(|delivery, number| {
            if number == 1 {
                assert_eq!(
                    delivery.update(|resources| {
                        resources.model += 1;
                        UpdateOutcome::new(resources.model).with_next_event(Value::new(10))
                    }),
                    Ok(1)
                );
            }
        })
        .build();

    assert!(delivery.send(next_batch([1, 2])));
    assert_eq!(values(&log), [1, 2, 10]);
}

#[test]
fn update_runs_and_returns_even_when_its_events_are_rejected() {
    let log = new_log();
    let log_of_observer = log.clone();
    let delivery = builder(&log)
        .on_next(move |delivery, number| {
            if number != 1 {
                return;
            }
            assert!(delivery.send(completed()));
            // The update still runs against the resources, but the termination is already queued,
            // so its events are dropped instead of being delivered.
            let dropped = Value::new(10).on_drop(note(&log_of_observer, "10 dropped"));
            assert_eq!(
                delivery.update(move |resources| {
                    resources.model += 1;
                    UpdateOutcome::new(resources.model).with_next_event(dropped)
                }),
                Ok(1)
            );
        })
        .build();

    assert!(delivery.send(next_batch([1, 2])));
    assert_eq!(values(&log), [1, 2]);
    assert!(records(&log).contains(&Record::Note("10 dropped")));
}

#[test]
fn the_arms_of_one_update_share_the_type_of_their_outcome() {
    let log = new_log();
    let delivery = builder(&log).build();

    // Both arms are one expression, so the type state makes the arm that emits nothing say so.
    let update = || {
        delivery.update(|resources| {
            resources.model += 1;
            if resources.model == 1 {
                UpdateOutcome::new(resources.model)
                    .with_drop_outside(Value::new(0).on_drop(note(&log, "dropped outside")))
                    .with_next_event(Value::new(1))
            } else {
                UpdateOutcome::new(resources.model)
                    .without_drop_outside()
                    .without_events()
            }
        })
    };

    assert_eq!(update(), Ok(1));
    assert_eq!(update(), Ok(2));
    assert_eq!(
        records(&log),
        [Record::Next(1), Record::Note("dropped outside")]
    );
}

// MARK: - Handles

#[test]
fn a_clone_shares_one_delivery() {
    let log = new_log();
    let delivery = builder(&log).build();
    let clone = delivery.clone();

    assert!(clone.send(next(1)));
    delivery.stop();
    assert!(!clone.send(next(2)));
    assert_eq!(values(&log), [1]);
}

#[test]
fn a_weak_handle_upgrades_while_a_strong_one_is_alive() {
    let log = new_log();
    let delivery = builder(&log).build();
    let weak = delivery.downgrade();

    assert!(weak.upgrade().is_some());

    // Stopping does not free the shared state: the handles stay valid and reject every event.
    delivery.stop();
    let upgraded = weak.upgrade().expect("a stopped delivery is still there");
    assert!(!upgraded.send(next(1)));

    drop(upgraded);
    drop(delivery);
    assert!(weak.upgrade().is_none());
}

#[test]
fn dropping_the_last_handle_drops_the_observer_without_notifying_it() {
    let log = new_log();
    let delivery = builder(&log).build();
    let clone = delivery.clone();

    drop(delivery);
    assert_eq!(records(&log), [], "another handle is still alive");

    drop(clone);
    assert_eq!(
        records(&log),
        [Record::ObserverDropped, Record::ResourcesDropped]
    );
}

// MARK: - Panicking observers

#[cfg(panic = "unwind")]
#[test]
fn a_panic_from_on_next_stops_the_delivery() {
    use crate::tests_utils::panic::expect_panic_on_drop;

    let log = new_log();
    let token = Shared::new(Mutable::new(None));
    let token_of_observer = token.clone();
    let delivery = builder(&log)
        .on_next(move |_delivery, number| {
            if number == 1 {
                // Dropping the token panics, which unwinds out of this notification.
                drop(token_of_observer.take_value());
            }
        })
        .build();

    expect_panic_on_drop(|panic_on_drop| {
        token.replace_value(Some(panic_on_drop));
        delivery.send(next_batch([1, 2, 3]));
    });

    // The delivery is stopped instead of being stuck in its delivering state: the values that were
    // still queued are gone, and so are the observer and the resources.
    assert_eq!(
        records(&log),
        [
            Record::Next(1),
            Record::ResourcesDropped,
            Record::ObserverDropped,
        ]
    );
    assert!(!delivery.send(next(4)));
}

#[cfg(panic = "unwind")]
#[test]
fn a_panic_from_on_termination_drops_the_resources() {
    use crate::tests_utils::panic::expect_panic_on_drop;

    let log = new_log();
    let token = Shared::new(Mutable::new(None));
    let token_of_observer = token.clone();
    let delivery = builder(&log)
        .on_termination(move |_delivery, _termination| {
            drop(token_of_observer.take_value());
        })
        .build();

    expect_panic_on_drop(|panic_on_drop| {
        token.replace_value(Some(panic_on_drop));
        delivery.send(completed());
    });

    // Unwinding from the terminal notification drops the resources the loop was still holding.
    assert_eq!(
        records(&log),
        [
            Record::Termination(Termination::Completed),
            Record::ObserverDropped,
            Record::ResourcesDropped,
        ]
    );
    assert!(!delivery.send(next(1)));
}

// MARK: - Concurrency

#[cfg(not(feature = "single-threaded"))]
#[test]
fn concurrent_sends_are_delivered_one_at_a_time() {
    use rx_rust::utils::mutable::{MutableBool, MutableBoolHelper};

    const THREADS: i32 = 4;
    const VALUES_PER_THREAD: i32 = 25;

    let log = new_log();
    let delivering = Shared::new(MutableBool::new(false));
    let delivering_of_observer = delivering.clone();
    let delivery = builder(&log)
        .on_next(move |_delivery, _value| {
            assert!(
                delivering_of_observer.change_if_not_equal(true),
                "two values must never be delivered at the same time"
            );
            std::thread::yield_now();
            assert!(delivering_of_observer.change_if_not_equal(false));
        })
        .build();

    std::thread::scope(|scope| {
        for thread in 0..THREADS {
            let delivery = delivery.clone();
            scope.spawn(move || {
                for value in 0..VALUES_PER_THREAD {
                    assert!(delivery.send(next(thread * VALUES_PER_THREAD + value)));
                }
            });
        }
    });

    // Whichever thread sent it, and whichever thread delivered it, every value arrives exactly once.
    let mut delivered = values(&log);
    delivered.sort_unstable();
    assert_eq!(
        delivered,
        (0..THREADS * VALUES_PER_THREAD).collect::<Vec<_>>()
    );
}
