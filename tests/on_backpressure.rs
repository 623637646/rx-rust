use rx_rust::utils::mutable::MutableExt;
use rx_rust::{
    disposable::Disposable,
    observable::{Observable, ObservableExt, Subscription},
    observer::{Observer, Termination},
    operators::{
        backpressure::on_backpressure::{BackpressureCollection, OnBackpressure, RequestToken},
        creating::{create::Create, empty::Empty, throw::Throw},
    },
    subject::{behavior_subject::BehaviorSubject, publish_subject::PublishSubject},
    utils::{
        mutable::{Mutable, MutableHelper},
        subscribe_with_context,
        types::{MaybeSend, Shared},
    },
};
use std::convert::Infallible;

#[derive(Clone)]
struct ChunksOfThree<T>(Vec<T>);

impl<T> Default for ChunksOfThree<T> {
    fn default() -> Self {
        Self(Vec::new())
    }
}

impl<T> BackpressureCollection for ChunksOfThree<T> {
    type Input = T;
    type Output = Vec<T>;

    fn extend_one(&mut self, item: Self::Input) {
        self.0.push(item);
    }

    fn take_next_value(&mut self) -> Option<Self::Output> {
        if self.0.len() < 3 {
            None
        } else {
            Some(self.0.drain(..3).collect())
        }
    }
}

struct Recorder<'or, T, E> {
    values: Shared<Mutable<Vec<Vec<T>>>>,
    requests: Shared<Mutable<Vec<RequestToken<'or>>>>,
    termination: Shared<Mutable<Option<Termination<E>>>>,
}

impl<'or, T, E> Recorder<'or, T, E> {
    fn new() -> Self {
        Self {
            values: Shared::new(Mutable::new(Vec::new())),
            requests: Shared::new(Mutable::new(Vec::new())),
            termination: Shared::new(Mutable::new(None)),
        }
    }

    fn values(&self) -> Vec<Vec<T>>
    where
        T: Clone,
    {
        self.values.with_ref(|values| values.clone())
    }

    fn request_count(&self) -> usize {
        self.requests.with_ref(|requests| requests.len())
    }

    fn take_request(&self) -> RequestToken<'or> {
        self.requests.with_mut(|requests| requests.remove(0))
    }

    fn termination(&self) -> Option<Termination<E>>
    where
        E: Clone,
    {
        self.termination.with_ref(|termination| termination.clone())
    }
}

fn record_observable<'or, T, E, OE>(observable: OE) -> (Recorder<'or, T, E>, Subscription<OE::D>)
where
    T: MaybeSend + 'or,
    E: MaybeSend + 'or,
    OE: Observable<'or, (Vec<T>, RequestToken<'or>), E>,
{
    let recorder = Recorder::new();
    let values = recorder.values.clone();
    let requests = recorder.requests.clone();
    let termination = recorder.termination.clone();
    let subscription = observable.subscribe_with_callback(
        move |(value, request)| {
            values.with_mut(|values| values.push(value));
            requests.with_mut(|values| values.push(request));
        },
        move |value| {
            termination.replace_value(Some(value));
        },
    );
    (recorder, subscription)
}

fn subscribe_chunks<'or, T, E, OE>(
    source: OE,
) -> (
    Recorder<'or, T, E>,
    Subscription<subscribe_with_context::Disposal<'or, OE::D>>,
)
where
    T: MaybeSend + 'or,
    E: MaybeSend + 'or,
    OE: Observable<'or, T, E>,
{
    record_observable(OnBackpressure::new(source, ChunksOfThree::default()))
}

#[test]
fn waits_until_collection_can_produce_an_output() {
    let mut subject = PublishSubject::<i32, Infallible>::new();
    let (recorder, _subscription) = subscribe_chunks(subject.clone());

    subject.on_next(1);
    subject.on_next(2);
    assert!(recorder.values().is_empty());
    assert_eq!(recorder.request_count(), 0);

    subject.on_next(3);
    assert_eq!(recorder.values(), [vec![1, 2, 3]]);
    assert_eq!(recorder.request_count(), 1);
    assert_eq!(recorder.termination(), None);
}

#[test]
fn buffers_multiple_ready_outputs_until_each_is_requested() {
    let mut subject = PublishSubject::<i32, Infallible>::new();
    let (recorder, _subscription) = subscribe_chunks(subject.clone());

    for value in 1..=9 {
        subject.on_next(value);
    }
    assert_eq!(recorder.values(), [vec![1, 2, 3]]);

    recorder.take_request().request();
    assert_eq!(recorder.values(), [vec![1, 2, 3], vec![4, 5, 6]]);
    assert_eq!(recorder.request_count(), 1);

    recorder.take_request().request();
    assert_eq!(
        recorder.values(),
        [vec![1, 2, 3], vec![4, 5, 6], vec![7, 8, 9]]
    );
    assert_eq!(recorder.request_count(), 1);
}

#[test]
fn request_with_partial_buffer_waits_for_more_input() {
    let mut subject = PublishSubject::<i32, Infallible>::new();
    let (recorder, _subscription) = subscribe_chunks(subject.clone());

    for value in 1..=5 {
        subject.on_next(value);
    }
    assert_eq!(recorder.values(), [vec![1, 2, 3]]);

    recorder.take_request().request();
    assert_eq!(recorder.values(), [vec![1, 2, 3]]);
    assert_eq!(recorder.request_count(), 0);

    subject.on_next(6);
    assert_eq!(recorder.values(), [vec![1, 2, 3], vec![4, 5, 6]]);
    assert_eq!(recorder.request_count(), 1);
}

#[test]
fn request_with_empty_buffer_allows_the_next_output_through() {
    let mut subject = PublishSubject::<i32, Infallible>::new();
    let (recorder, _subscription) = subscribe_chunks(subject.clone());

    for value in 1..=3 {
        subject.on_next(value);
    }
    recorder.take_request().request();
    assert_eq!(recorder.request_count(), 0);

    for value in 4..=6 {
        subject.on_next(value);
    }
    assert_eq!(recorder.values(), [vec![1, 2, 3], vec![4, 5, 6]]);
    assert_eq!(recorder.request_count(), 1);
}

#[test]
fn completion_before_any_output_discards_partial_input() {
    let mut subject = PublishSubject::<i32, Infallible>::new();
    let (recorder, _subscription) = subscribe_chunks(subject.clone());

    subject.on_next(1);
    subject.on_next(2);
    subject.on_termination(Termination::Completed);

    assert!(recorder.values().is_empty());
    assert_eq!(recorder.request_count(), 0);
    assert_eq!(recorder.termination(), Some(Termination::Completed));
}

#[test]
fn pending_completion_discards_partial_input_on_request() {
    let mut subject = PublishSubject::<i32, Infallible>::new();
    let (recorder, _subscription) = subscribe_chunks(subject.clone());

    for value in 1..=5 {
        subject.on_next(value);
    }
    subject.on_termination(Termination::Completed);
    assert_eq!(recorder.termination(), None);

    recorder.take_request().request();
    assert_eq!(recorder.values(), [vec![1, 2, 3]]);
    assert_eq!(recorder.request_count(), 0);
    assert_eq!(recorder.termination(), Some(Termination::Completed));
}

#[test]
fn completion_waits_until_all_complete_outputs_are_requested() {
    let mut subject = PublishSubject::<i32, Infallible>::new();
    let (recorder, _subscription) = subscribe_chunks(subject.clone());

    for value in 1..=9 {
        subject.on_next(value);
    }
    subject.on_termination(Termination::Completed);

    recorder.take_request().request();
    assert_eq!(recorder.termination(), None);
    recorder.take_request().request();
    assert_eq!(recorder.termination(), None);
    recorder.take_request().request();

    assert_eq!(
        recorder.values(),
        [vec![1, 2, 3], vec![4, 5, 6], vec![7, 8, 9]]
    );
    assert_eq!(recorder.request_count(), 0);
    assert_eq!(recorder.termination(), Some(Termination::Completed));
}

#[test]
fn error_before_any_output_discards_partial_input() {
    let mut subject = PublishSubject::<i32, &str>::new();
    let (recorder, _subscription) = subscribe_chunks(subject.clone());

    subject.on_next(1);
    subject.on_next(2);
    subject.on_termination(Termination::Error("error"));

    assert!(recorder.values().is_empty());
    assert_eq!(recorder.request_count(), 0);
    assert_eq!(recorder.termination(), Some(Termination::Error("error")));
}

#[test]
fn pending_error_discards_partial_input_on_request() {
    let mut subject = PublishSubject::<i32, &str>::new();
    let (recorder, _subscription) = subscribe_chunks(subject.clone());

    for value in 1..=5 {
        subject.on_next(value);
    }
    subject.on_termination(Termination::Error("error"));
    assert_eq!(recorder.termination(), None);

    recorder.take_request().request();
    assert_eq!(recorder.values(), [vec![1, 2, 3]]);
    assert_eq!(recorder.request_count(), 0);
    assert_eq!(recorder.termination(), Some(Termination::Error("error")));
}

#[test]
fn error_waits_until_all_complete_outputs_are_requested() {
    let mut subject = PublishSubject::<i32, &str>::new();
    let (recorder, _subscription) = subscribe_chunks(subject.clone());

    for value in 1..=9 {
        subject.on_next(value);
    }
    subject.on_termination(Termination::Error("error"));

    recorder.take_request().request();
    recorder.take_request().request();
    assert_eq!(recorder.termination(), None);
    recorder.take_request().request();

    assert_eq!(
        recorder.values(),
        [vec![1, 2, 3], vec![4, 5, 6], vec![7, 8, 9]]
    );
    assert_eq!(recorder.termination(), Some(Termination::Error("error")));
}

#[test]
fn termination_is_immediate_after_requesting_an_empty_buffer() {
    let mut subject = PublishSubject::<i32, Infallible>::new();
    let (recorder, _subscription) = subscribe_chunks(subject.clone());

    for value in 1..=3 {
        subject.on_next(value);
    }
    recorder.take_request().request();
    subject.on_termination(Termination::Completed);

    assert_eq!(recorder.values(), [vec![1, 2, 3]]);
    assert_eq!(recorder.request_count(), 0);
    assert_eq!(recorder.termination(), Some(Termination::Completed));
}

#[test]
fn termination_is_immediate_after_requesting_a_partial_buffer() {
    let mut subject = PublishSubject::<i32, Infallible>::new();
    let (recorder, _subscription) = subscribe_chunks(subject.clone());

    for value in 1..=5 {
        subject.on_next(value);
    }
    recorder.take_request().request();
    assert_eq!(recorder.request_count(), 0);

    subject.on_termination(Termination::Completed);
    assert_eq!(recorder.values(), [vec![1, 2, 3]]);
    assert_eq!(recorder.termination(), Some(Termination::Completed));
}

#[test]
fn request_after_disposal_is_a_no_op() {
    let mut subject = PublishSubject::<i32, Infallible>::new();
    let (recorder, subscription) = subscribe_chunks(subject.clone());

    for value in 1..=3 {
        subject.on_next(value);
    }
    subscription.dispose();
    recorder.take_request().request();
    for value in 4..=6 {
        subject.on_next(value);
    }

    assert_eq!(recorder.values(), [vec![1, 2, 3]]);
    assert_eq!(recorder.request_count(), 0);
    assert_eq!(recorder.termination(), None);
}

#[test]
fn request_can_be_called_reentrantly_from_on_next() {
    let mut subject = PublishSubject::<i32, Infallible>::new();
    let values = Shared::new(Mutable::new(Vec::new()));
    let values_cloned = values.clone();
    let termination = Shared::new(Mutable::new(None));
    let termination_cloned = termination.clone();
    let _subscription = OnBackpressure::new(subject.clone(), ChunksOfThree::default())
        .subscribe_with_callback(
            move |(value, request)| {
                values_cloned.with_mut(|values| values.push(value));
                request.request();
            },
            move |value| {
                termination_cloned.replace_value(Some(value));
            },
        );

    for value in 1..=9 {
        subject.on_next(value);
    }
    subject.on_termination(Termination::Completed);

    assert_eq!(
        values.with_ref(|values| values.clone()),
        [vec![1, 2, 3], vec![4, 5, 6], vec![7, 8, 9]]
    );
    assert_eq!(
        termination.with_ref(|termination| termination.clone()),
        Some(Termination::Completed)
    );
}

#[test]
fn upstream_can_terminate_reentrantly_from_on_next() {
    let mut subject = PublishSubject::<i32, Infallible>::new();
    let subject_cloned = subject.clone();
    let values = Shared::new(Mutable::new(Vec::new()));
    let values_cloned = values.clone();
    let termination = Shared::new(Mutable::new(None));
    let termination_cloned = termination.clone();
    let _subscription = OnBackpressure::new(subject.clone(), ChunksOfThree::default())
        .subscribe_with_callback(
            move |(value, request)| {
                values_cloned.with_mut(|values| values.push(value));
                request.request();
                subject_cloned
                    .clone()
                    .on_termination(Termination::<Infallible>::Completed);
            },
            move |value| {
                termination_cloned.replace_value(Some(value));
            },
        );

    subject.on_next(1);
    subject.on_next(2);
    subject.on_next(3);

    assert_eq!(values.with_ref(|values| values.clone()), [vec![1, 2, 3]]);
    assert_eq!(
        termination.with_ref(|termination| termination.clone()),
        Some(Termination::Completed)
    );
}

#[test]
fn handles_synchronous_emission_and_termination_during_subscribe() {
    let source = Create::new(|mut observer| {
        for value in 1..=6 {
            observer.on_next(value);
        }
        observer.on_termination(Termination::<Infallible>::Completed);
        Subscription::default()
    });
    let (recorder, _subscription) = subscribe_chunks(source);

    assert_eq!(recorder.values(), [vec![1, 2, 3]]);
    assert_eq!(recorder.termination(), None);
    recorder.take_request().request();
    assert_eq!(recorder.values(), [vec![1, 2, 3], vec![4, 5, 6]]);
    recorder.take_request().request();
    assert_eq!(recorder.termination(), Some(Termination::Completed));
}

#[test]
fn handles_immediate_completion_during_subscribe() {
    let (recorder, _subscription): (Recorder<'_, Infallible, Infallible>, _) =
        subscribe_chunks(Empty);

    assert!(recorder.values().is_empty());
    assert_eq!(recorder.request_count(), 0);
    assert_eq!(recorder.termination(), Some(Termination::Completed));
}

#[test]
fn handles_immediate_error_during_subscribe() {
    let (recorder, _subscription): (Recorder<'_, Infallible, &str>, _) =
        subscribe_chunks(Throw::new("error"));

    assert!(recorder.values().is_empty());
    assert_eq!(recorder.request_count(), 0);
    assert_eq!(recorder.termination(), Some(Termination::Error("error")));
}

#[test]
fn subscriptions_have_independent_collection_and_request_state() {
    let mut subject = PublishSubject::<i32, Infallible>::new();
    let observable = OnBackpressure::new(subject.clone(), ChunksOfThree::default());
    let (recorder_1, _subscription_1) = record_observable(observable.clone());
    let (recorder_2, _subscription_2) = record_observable(observable);

    for value in 1..=6 {
        subject.on_next(value);
    }
    assert_eq!(recorder_1.values(), [vec![1, 2, 3]]);
    assert_eq!(recorder_2.values(), [vec![1, 2, 3]]);

    recorder_1.take_request().request();
    assert_eq!(recorder_1.values(), [vec![1, 2, 3], vec![4, 5, 6]]);
    assert_eq!(recorder_2.values(), [vec![1, 2, 3]]);

    subject.on_termination(Termination::Completed);
    recorder_1.take_request().request();
    recorder_2.take_request().request();
    assert_eq!(recorder_1.termination(), Some(Termination::Completed));
    assert_eq!(recorder_2.values(), [vec![1, 2, 3], vec![4, 5, 6]]);
    assert_eq!(recorder_2.termination(), None);
    recorder_2.take_request().request();
    assert_eq!(recorder_2.termination(), Some(Termination::Completed));
}

#[test]
fn downstream_take_disposes_upstream_and_invalidates_the_token() {
    let mut subject = PublishSubject::<i32, Infallible>::new();
    let observable = OnBackpressure::new(subject.clone(), ChunksOfThree::default()).take(1);
    let (recorder, _subscription) = record_observable(observable);

    for value in 1..=3 {
        subject.on_next(value);
    }
    assert_eq!(recorder.values(), [vec![1, 2, 3]]);
    assert_eq!(recorder.termination(), Some(Termination::Completed));

    recorder.take_request().request();
    for value in 4..=6 {
        subject.on_next(value);
    }
    assert_eq!(recorder.values(), [vec![1, 2, 3]]);
    assert_eq!(recorder.request_count(), 0);
}

#[test]
fn supports_borrowed_values_and_errors() {
    let values = [1, 2, 3];
    let error = "error";
    let mut subject = PublishSubject::<&i32, &str>::new();
    let (recorder, _subscription) = subscribe_chunks(subject.clone());

    for value in &values {
        subject.on_next(value);
    }
    subject.on_termination(Termination::Error(error));
    assert_eq!(
        recorder.values(),
        [vec![&values[0], &values[1], &values[2]]]
    );
    assert_eq!(recorder.termination(), None);
    recorder.take_request().request();
    assert_eq!(recorder.termination(), Some(Termination::Error(error)));
}

#[test]
fn supports_mutably_borrowed_values() {
    let mut value_1 = 1;
    let mut value_2 = 2;
    let mut value_3 = 3;
    let source = Create::new(|mut observer| {
        observer.on_next(&mut value_1);
        observer.on_next(&mut value_2);
        observer.on_next(&mut value_3);
        observer.on_termination(Termination::<Infallible>::Completed);
        Subscription::default()
    });
    let subscription = OnBackpressure::new(source, ChunksOfThree::default())
        .subscribe_with_callback(
            |(values, request)| {
                for value in values {
                    *value *= 2;
                }
                request.request();
            },
            |termination| assert_eq!(termination, Termination::Completed),
        );
    drop(subscription);

    assert_eq!([value_1, value_2, value_3], [2, 4, 6]);
}

#[test]
fn handles_values_emitted_synchronously_on_subscription() {
    let mut subject = BehaviorSubject::<i32, Infallible>::new(1);
    let (recorder, _subscription) = subscribe_chunks(subject.clone());

    assert!(recorder.values().is_empty());
    subject.on_next(2);
    subject.on_next(3);
    assert_eq!(recorder.values(), [vec![1, 2, 3]]);
    assert_eq!(recorder.request_count(), 1);
}

#[test]
fn on_backpressure_is_clone_when_source_and_collection_are_clone() {
    let subject = PublishSubject::<i32, String>::new();
    let observable = OnBackpressure::new(subject, ChunksOfThree::default());
    _ = observable.clone();
}

#[test]
fn convenience_api_preserves_type_inference_without_subscribing() {
    let subject = PublishSubject::<'_, i32, String>::new();
    subject
        .on_backpressure(ChunksOfThree::default())
        .filter(|_| true);
}

#[test]
fn convenience_api_preserves_type_inference_when_subscribing() {
    let subject = PublishSubject::<'_, i32, String>::new();
    let observable = subject
        .on_backpressure(ChunksOfThree::default())
        .filter(|_| true);
    let (_recorder, _subscription) = record_observable(observable);
}

#[cfg(not(feature = "single-threaded"))]
#[test]
fn request_token_can_be_used_from_another_thread() {
    let mut subject = PublishSubject::<'static, i32, Infallible>::new();
    let (recorder, _subscription) = subscribe_chunks(subject.clone());

    for value in 1..=6 {
        subject.on_next(value);
    }
    let request = recorder.take_request();
    std::thread::spawn(move || request.request())
        .join()
        .unwrap();

    assert_eq!(recorder.values(), [vec![1, 2, 3], vec![4, 5, 6]]);
    assert_eq!(recorder.request_count(), 1);
}
