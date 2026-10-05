//! A multicast subject that forwards what it receives to every observer.
//!
//! It is [`SerializedMulticast`] with nothing of its own to guard: every method here is one call
//! into it. The subject is terminated as soon as [`Observer::on_termination`] is *called*, not when
//! the termination reaches the observers — see the multicast's module documentation for why, and
//! for everything else that governs the ordering here.

use super::Subject;
use crate::delegate_disposal;
use crate::disposable::DisposableExt;
use crate::disposable::option_disposal::OptionDisposal;
use crate::observable::Subscription;
use crate::observer::boxed_observer::{IntoBoxedObserver, ObserverMode};
use crate::thread_mode::{Local, Shared};
use crate::utils::pending_events::EventBatch;
use crate::utils::serialized_multicast::{MulticastDisposal, SerializedMulticast};
use crate::{
    observable::{Observable, ObservableTypes},
    observer::{Flow, Observer, Termination},
};
use educe::Educe;

/// The plain multicast subject: forwards what it receives to every current observer.
///
/// A late subscriber receives only what is emitted after it subscribed, or the termination at
/// once if the subject has already terminated. Observers are notified in subscription order.
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt,
///     observer::{Observer, Termination},
///     subject::publish_subject::PublishSubject,
/// };
///
/// let mut seen = Vec::new();
/// let mut sender = PublishSubject::<i32, std::convert::Infallible, _>::local();
/// let _ = sender.on_next(1); // Nobody is subscribed yet: dropped.
///
/// let subscription = sender.clone().subscribe_with_callback(|value| seen.push(value), |_| {});
/// let _ = sender.on_next(2);
/// sender.on_termination(Termination::Completed);
/// drop(subscription);
/// assert_eq!(seen, [2]);
/// ```
///
/// The thread mode is declared when it is created: [`local`](Self::local) for a subject that is
/// fed and subscribed to on one thread, [`shared`](Self::shared) for one that crosses threads, whose
/// observers must then be `Send`.
#[derive(Educe)]
#[educe(Debug, Clone(bound()))]
pub struct PublishSubject<'or, T, E, M: ObserverMode>(
    #[educe(Debug(ignore))] SerializedMulticast<'or, T, E, M>,
);

impl<T, E, M: ObserverMode> PublishSubject<'_, T, E, M> {
    /// Creates a subject in the mode `M`, for code that is generic over the mode, such as an operator
    /// that creates it in the mode of its source; [`local`](Self::local) and [`shared`](Self::shared)
    /// name the mode instead.
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self(SerializedMulticast::idle(()))
    }
}

impl<T, E> PublishSubject<'_, T, E, Local> {
    /// Creates a subject that stays on one thread.
    pub fn local() -> Self {
        Self::new()
    }
}

impl<T, E> PublishSubject<'_, T, E, Shared> {
    /// Creates a subject that can be fed and subscribed to from any thread.
    pub fn shared() -> Self {
        Self::new()
    }
}

delegate_disposal!(
    Disposal<'or, T, E, M>,
    OptionDisposal<MulticastDisposal<'or, T, E, M, ()>>,
    where M: ObserverMode
);

impl<'or, T, E, M: ObserverMode> ObservableTypes for PublishSubject<'or, T, E, M>
where
    T: Clone,
    E: Clone,
{
    type Item = T;
    type Error = E;
    type Mode = M;
    type D = Disposal<'or, T, E, M>;
}

impl<'or, T, E, M, OR> Observable<OR> for PublishSubject<'or, T, E, M>
where
    M: ObserverMode,
    OR: IntoBoxedObserver<'or, T, E, M>,
    T: Clone,
    E: Clone,
{
    fn subscribe(self, observer: OR) -> Subscription<Self::D> {
        match self.0.subscribe(observer) {
            Some(disposal) => OptionDisposal::some(disposal),
            None => OptionDisposal::none(),
        }
        .into_subscription()
    }
}

impl<T, E, M: ObserverMode> Observer<T, E> for PublishSubject<'_, T, E, M>
where
    T: Clone,
    E: Clone,
{
    fn on_next(&mut self, value: T) -> Flow {
        // A value that arrives after the termination is dropped outside the lock.
        self.0.send(EventBatch::Next(value))
    }

    fn on_termination(self, termination: Termination<E>) {
        // Only the first termination is ever queued, and nothing joins the subject after it.
        let _ = self.0.send(EventBatch::Termination(termination));
    }
}

impl<T, E, M: ObserverMode> Subject<T, E> for PublishSubject<'_, T, E, M>
where
    T: Clone,
    E: Clone,
{
    fn terminated(&self) -> Option<Termination<E>> {
        self.0.terminated()
    }
}
