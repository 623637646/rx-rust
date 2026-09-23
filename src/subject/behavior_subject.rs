//! A subject that keeps a current value and hands it to every new subscriber.
//!
//! The current value lives in the [`SerializedMulticast`]'s resources, so replacing it and
//! forwarding it are one atomic step, and so are reading it and joining the multicast: a
//! subscriber can neither miss a value emitted while it was subscribing nor observe one twice.

use super::Subject;
use crate::delegate_disposal;
use crate::disposable::DisposableExt;
use crate::disposable::option_disposal::OptionDisposal;
use crate::observable::Subscription;
use crate::utils::pending_events::EventBatch;
use crate::utils::serialized_delivery::UpdateOutcome;
use crate::utils::serialized_multicast::{Admission, MulticastDisposal, SerializedMulticast};
use crate::utils::types::MaybeSend;
use crate::{
    observable::Observable,
    observer::{Flow, Observer, Termination},
};
use educe::Educe;

/// A subject with a current value, which every new subscriber receives first.
///
/// The current value is replaced by each value the subject receives; [`value`](Self::value)
/// reads it. Once the subject has terminated, a late subscriber receives the termination only.
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt,
///     observer::Observer,
///     subject::behavior_subject::BehaviorSubject,
/// };
///
/// let mut seen = Vec::new();
/// let mut sender = BehaviorSubject::<i32, std::convert::Infallible>::new(0);
/// let _ = sender.on_next(1);
/// assert_eq!(sender.value(), 1);
///
/// let subscription = sender.clone().subscribe_with_callback(|value| seen.push(value), |_| {});
/// let _ = sender.on_next(2);
/// drop((subscription, sender));
/// assert_eq!(seen, [1, 2]); // The current value first, then what follows.
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct BehaviorSubject<'or, T, E>(SerializedMulticast<'or, T, E, T>);

impl<T, E> BehaviorSubject<'_, T, E> {
    /// Creates a subject whose current value is `value`.
    pub fn new(value: T) -> Self {
        Self(SerializedMulticast::idle(value))
    }
}

impl<T, E> BehaviorSubject<'_, T, E>
where
    T: Clone,
    E: Clone,
{
    /// The current value.
    ///
    /// # Panics
    ///
    /// Panics once an observer's callback has panicked, which takes the subject's whole state with
    /// it and leaves nothing to return.
    pub fn value(&self) -> T {
        self.0
            .read(|value, _| value.clone())
            .expect("the subject is dead because an observer panicked")
    }
}

delegate_disposal!(
    Disposal<'or, T, E>,
    OptionDisposal<MulticastDisposal<'or, T, E, T>>,
);

impl<'or, T, E> Observable<'or, T, E> for BehaviorSubject<'or, T, E>
where
    T: Clone,
    E: Clone,
{
    type D = Disposal<'or, T, E>;

    fn subscribe(self, observer: impl Observer<T, E> + MaybeSend + 'or) -> Subscription<Self::D> {
        match self
            .0
            .subscribe_with(observer, |value, terminated| match terminated {
                // The current value is snapshotted under the very lock that queues the subscription,
                // so it is followed by exactly the values emitted after it.
                None => Admission::Join(vec![value.clone()]),
                Some(termination) => Admission::Terminated(Vec::new(), termination.clone()),
            }) {
            Some(disposal) => OptionDisposal::some(disposal),
            None => OptionDisposal::none(),
        }
        .into_subscription()
    }
}

impl<T, E> Observer<T, E> for BehaviorSubject<'_, T, E>
where
    T: Clone,
    E: Clone,
{
    fn on_next(&mut self, value: T) -> Flow {
        self.0
            .update(|current, terminated| {
                if terminated.is_some() {
                    return UpdateOutcome::new(Flow::Stop)
                        .with_drop_outside(Some(value))
                        .without_events();
                }
                // Replacing the current value and forwarding it are one step, so the value a
                // subscriber is given always matches the values it then receives. The replaced
                // value is dropped outside the lock: dropping it can run arbitrary code.
                let previous = std::mem::replace(current, value.clone());
                UpdateOutcome::new(Flow::Continue)
                    .with_drop_outside(Some(previous))
                    .with_next_event(value)
            })
            .unwrap_or(Flow::Stop)
    }

    fn on_termination(self, termination: Termination<E>) {
        let _ = self.0.send(EventBatch::Termination(termination));
    }
}

impl<'or, T, E> Subject<'or, T, E> for BehaviorSubject<'or, T, E>
where
    T: Clone,
    E: Clone,
{
    fn terminated(&self) -> Option<Termination<E>>
    where
        E: Clone,
    {
        self.0.terminated()
    }
}
