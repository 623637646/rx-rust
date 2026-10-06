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
use crate::observer::boxed_observer::{IntoBoxedObserver, ObserverMode};
use crate::thread_mode::{Local, Shared};
use crate::utils::pending_events::EventBatch;
use crate::utils::serialized_delivery::UpdateOutcome;
use crate::utils::serialized_multicast::{Admission, MulticastDisposal, SerializedMulticast};
use crate::{
    observable::{Observable, ObservableTypes},
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
/// let mut sender = BehaviorSubject::<i32, std::convert::Infallible, rx_rust::thread_mode::Local>::local(0);
/// let _ = sender.on_next(1);
/// assert_eq!(sender.value(), 1);
///
/// let subscription = sender.clone().subscribe_with_callback(|value| seen.push(value), |_| {});
/// let _ = sender.on_next(2);
/// drop((subscription, sender));
/// assert_eq!(seen, [1, 2]); // The current value first, then what follows.
/// ```
#[derive(Educe)]
#[educe(Debug, Clone(bound()))]
pub struct BehaviorSubject<'or, T, E, M: ObserverMode>(
    #[educe(Debug(ignore))] SerializedMulticast<'or, T, E, M, T>,
);

impl<T, E, M: ObserverMode> BehaviorSubject<'_, T, E, M> {
    /// Creates a subject in the mode `M`, for code that is generic over the mode, such as an
    /// operator that creates it in the mode of its source; [`local`](Self::local) and
    /// [`shared`](Self::shared) name the mode instead.
    pub fn new(value: T) -> Self {
        Self(SerializedMulticast::idle(value))
    }
}

impl<T, E, M: ObserverMode> BehaviorSubject<'_, T, E, M>
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

impl<T, E> BehaviorSubject<'_, T, E, Local> {
    /// Creates a subject that stays on one thread.
    pub fn local(value: T) -> Self {
        Self::new(value)
    }
}

impl<T, E> BehaviorSubject<'_, T, E, Shared> {
    /// Creates a subject that can be fed and subscribed to from any thread.
    pub fn shared(value: T) -> Self {
        Self::new(value)
    }
}

delegate_disposal!(
    Disposal<'or, T, E, M>,
    OptionDisposal<MulticastDisposal<'or, T, E, M, T>>,
    where M: ObserverMode
);

impl<'or, T, E, M: ObserverMode> ObservableTypes for BehaviorSubject<'or, T, E, M>
where
    T: Clone,
    E: Clone,
{
    type Item = T;
    type Error = E;
    type Mode = M;
    type Disposal = Disposal<'or, T, E, M>;
}

impl<'or, T, E, M, OR> Observable<OR> for BehaviorSubject<'or, T, E, M>
where
    M: ObserverMode,
    OR: IntoBoxedObserver<'or, T, E, M>,
    T: Clone,
    E: Clone,
{
    fn subscribe(self, observer: OR) -> Subscription<Self::Disposal> {
        match self
            .0
            .subscribe_with(observer, |value, terminated| match terminated {
                // The current value is snapshotted under the very lock that queues the
                // subscription, so it is followed by exactly the values emitted after it.
                None => Admission::Join(vec![value.clone()]),
                Some(termination) => Admission::Terminated(Vec::new(), termination.clone()),
            }) {
            Some(disposal) => OptionDisposal::some(disposal),
            None => OptionDisposal::none(),
        }
        .into_subscription()
    }
}

impl<T, E, M: ObserverMode> Observer<T, E> for BehaviorSubject<'_, T, E, M>
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

impl<T, E, M: ObserverMode> Subject<T, E> for BehaviorSubject<'_, T, E, M>
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
