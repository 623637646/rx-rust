//! The helper most stateful operators are written with.
//!
//! A [`SubscriptionContext`] holds the downstream observer and the operator's own model behind one
//! lock (a [`SerializedDelivery`]). The
//! operator's observers update the model and queue events in one step, and the events are
//! delivered outside the lock, in order, whichever thread produced them. That is what makes an
//! operator with several sources, a notifier or a scheduler task correct without any locking of
//! its own.
//!
//! The context owns the source subscription, so that it disposes the source as soon as it stops,
//! whatever stopped it: a disposal, a termination — from the source, a notifier, a scheduler task
//! or another source — or a downstream that answered [`Flow::Stop`].
//!
//! An operator with two of them — two sources, or a source and a scheduler task — is written
//! with [`subscribe_with_context_two_steps`], which subscribes them in order, skips the second
//! one once the stream is already over, and disposes them in the opposite order.
//!
//! # Examples
//! An operator that emits the running total of two sources and completes on its own once it
//! exceeds a limit, which disposes both. The observers of the two sources share the total and the
//! downstream observer through the context:
//! ```rust
//! use rx_rust::{
//!     disposable::{Disposable, dispose_on_drop::DisposeOnDrop},
//!     observable::{ChainDisposalOf, JoinedModeOf, Observable, ObservableExt, ObservableTypes},
//!     observer::{Flow, Observer, Termination},
//!     subject::publish_subject::PublishSubject,
//!     thread_mode::ThreadMode,
//!     utils::{
//!         serialized_delivery::UpdateOutcome,
//!         subscribe_with_context::{self, SubscriptionContext, subscribe_with_context},
//!     },
//! };
//! use std::convert::Infallible;
//!
//! struct TotalUntil<A, B> { first: A, second: B, limit: i32 }
//!
//! // Nothing here depends on the observer: the disposal names the mode, the events, the model and
//! // the sources' disposals, never the observer's type.
//! impl<E, A, B> ObservableTypes for TotalUntil<A, B>
//! where
//!     A: ObservableTypes<Item = i32, Error = E>,
//!     B: ObservableTypes<Item = i32, Error = E>,
//! {
//!     type Item = i32;
//!     type Error = E;
//!     type Mode = JoinedModeOf<A, B>;
//!     type Disposal =
//!         subscribe_with_context::Disposal<JoinedModeOf<A, B>, i32, E, i32, ChainDisposalOf<A, B>>;
//! }
//!
//! // Both sources are subscribed with the operator's own observer, which is named here.
//! impl<E, A, B, OR> Observable<OR> for TotalUntil<A, B>
//! where
//!     OR: Observer<i32, E>,
//!     A: Observable<
//!             TotalObserver<JoinedModeOf<A, B>, OR, E, ChainDisposalOf<A, B>>,
//!             Item = i32,
//!             Error = E,
//!         >,
//!     B: Observable<
//!             TotalObserver<JoinedModeOf<A, B>, OR, E, ChainDisposalOf<A, B>>,
//!             Item = i32,
//!             Error = E,
//!         >,
//! {
//!     fn subscribe(self, observer: OR) -> DisposeOnDrop<Self::Disposal> {
//!         let limit = self.limit;
//!         subscribe_with_context(observer, 0, |context| {
//!             let first = self.first.subscribe(TotalObserver { context: context.clone(), limit });
//!             let second = self.second.subscribe(TotalObserver { context, limit });
//!             second.preceded_by_wrapped(first)
//!         })
//!     }
//! }
//!
//! struct TotalObserver<M: ThreadMode, OR, E, D: Disposable> {
//!     context: SubscriptionContext<M, i32, E, OR, i32, D>,
//!     limit: i32,
//! }
//!
//! impl<M: ThreadMode, OR: Observer<i32, E>, E, D: Disposable> Observer<i32, E>
//!     for TotalObserver<M, OR, E, D>
//! {
//!     fn on_next(&mut self, value: i32) -> Flow {
//!         // The model is read and written, and the events queued, under one lock.
//!         self.context.update_flow(|total| {
//!             *total += value;
//!             if *total > self.limit {
//!                 UpdateOutcome::empty()
//!                     .with_next_and_termination_events(*total, Termination::Completed)
//!             } else {
//!                 UpdateOutcome::empty().with_next_event(*total)
//!             }
//!         })
//!     }
//!     fn on_termination(self, termination: Termination<E>) {
//!         // The first termination, from either source, ends the stream.
//!         self.context.send_termination(termination);
//!     }
//! }
//!
//! let mut seen = Vec::new();
//! let mut first = PublishSubject::local();
//! let mut second = PublishSubject::<_, Infallible, _>::local();
//! let subscription = TotalUntil { first: first.clone(), second: second.clone(), limit: 5 }
//!     .subscribe_with_callback(
//!         |total| seen.push(total),
//!         |t| assert_eq!(t, Termination::Completed),
//!     );
//!
//! let _ = first.on_next(1);
//! let _ = second.on_next(2);
//! let _ = first.on_next(3); // Exceeds the limit: completes, and disposes both sources.
//! let _ = second.on_next(4);
//!
//! drop((subscription, first, second));
//! assert_eq!(seen, [1, 3, 6]);
//! ```

#[cfg(debug_assertions)]
use crate::utils::subscribe_with_auto_dispose_on_termination::is_auto_dispose_on_termination_observer;
use crate::{
    disposable::{
        Disposable, DisposableExt, chain_disposal::ChainDisposal, dispose_on_drop::DisposeOnDrop,
    },
    observer::{EventBatch, Flow, IntoFlow, Observer, Termination},
    thread_mode::ThreadMode,
    utils::serialized_delivery::{
        DeliveryStop, DeliveryStopped, SerializedDelivery, UpdateOutcome,
    },
};
use educe::Educe;

/// Stops a context: its model and the source subscription it owns are dropped, outside the lock,
/// and every later event is rejected.
///
/// Returned by [`subscribe_with_context`], where `D` is the disposal of the source subscription the
/// context owns.
///
/// Its type names the thread mode, the events, the model and the source's disposal, but not the
/// observer — the disposal of an observable must not depend on its observer (see
/// [`ObservableTypes`](crate::observable::ObservableTypes)). So it cannot release the observer
/// itself: the first clone of the context dropped after the stop does — typically the source's,
/// as the source subscription is disposed, even while a scheduler task still holds another. See
/// [`DeliveryStop`].
#[derive(Educe)]
#[educe(Debug)]
pub struct Disposal<M: ThreadMode, T, E, MD, D: Disposable>(
    #[educe(Debug(ignore))] DeliveryStop<M, T, E, ContextResources<MD, D>>,
);

impl<M: ThreadMode, T, E, MD, D: Disposable> Disposable for Disposal<M, T, E, MD, D> {
    fn dispose(self) {
        self.0.stop();
    }
}

/// Creates a subscription backed by a shared, serialized context containing the downstream
/// observer and a mutable model.
///
/// `builder` subscribes the sources with observers holding clones of the context, and returns
/// their subscription, which the context then owns: it is disposed once the context stops — by a
/// disposal, by a termination, or because downstream answered [`Flow::Stop`] — including when
/// that happens synchronously while `builder` is running. So the source is not left running once
/// the operator has ended its stream, whichever thread ended it.
///
/// `M` is the thread mode the context's pointers are picked for: the mode of the operator.
pub fn subscribe_with_context<M, T, E, OR, D, MD, F>(
    observer: OR,
    model: MD,
    builder: F,
) -> DisposeOnDrop<Disposal<M, T, E, MD, D>>
where
    M: ThreadMode,
    OR: Observer<T, E>,
    D: Disposable,
    F: FnOnce(SubscriptionContext<M, T, E, OR, MD, D>) -> DisposeOnDrop<D>,
{
    subscribe_with_context_opt(observer, model, |context| Some(builder(context.clone())))
}

/// The context handed to each step of [`subscribe_with_context_two_steps`]: it owns both steps'
/// subscriptions, so its disposal type names them both, whichever step is being assembled.
pub type TwoStepsContext<M, T, E, OR, MD, D1, D2> =
    SubscriptionContext<M, T, E, OR, MD, ChainDisposal<D2, D1>>;

/// The disposal of a [`subscribe_with_context_two_steps`] subscription: the second step is
/// disposed first, the last subscribed first.
pub type TwoStepsDisposal<M, T, E, MD, D1, D2> = Disposal<M, T, E, MD, ChainDisposal<D2, D1>>;

/// Creates a subscription whose sources are assembled in two steps, either of which may decide
/// that nothing more is to be subscribed.
///
/// `builder_1` runs first, and `builder_2` only while the context still accepts events, so a
/// source that ends the stream as it is subscribed — synchronously, or through a notifier — keeps
/// the step after it from starting its own subscription at all. The sources are subscribed in
/// that order and disposed in the opposite one, the last subscribed first.
///
/// A builder returns `None` to end the assembly: the step after it does not run, whatever the
/// steps before it subscribed is disposed at once, outside the lock, and the context ends up
/// owning no source subscription at all. So `None` says "this stream is over", not "I have
/// nothing of my own to dispose" — a step that installs nothing yet must leave the stream running
/// returns an empty disposal instead, as `timeout` does for a timeout that never fires.
///
/// Returning `None` does not stop the context by itself. It does leave no step holding a clone of
/// it, though, so the observer is released as this function returns — without a termination, the
/// way a source letting go of it would end the stream. A builder that means downstream to be told
/// sends a termination first, and the context stops as that is delivered.
///
/// Hence the `Option` the builders answer with, where the builder of [`subscribe_with_context`]
/// hands over a subscription and nothing else: that one is a free assembly, of whatever its
/// operator needs in whatever order, while these two are the fixed steps of one that may end part
/// way through.
///
/// `M` is the thread mode the context's pointers are picked for: the mode of the operator.
pub fn subscribe_with_context_two_steps<M, T, E, OR, D1, D2, MD, F1, F2>(
    observer: OR,
    model: MD,
    builder_1: F1,
    builder_2: F2,
) -> DisposeOnDrop<TwoStepsDisposal<M, T, E, MD, D1, D2>>
where
    M: ThreadMode,
    OR: Observer<T, E>,
    D1: Disposable,
    D2: Disposable,
    F1: FnOnce(&TwoStepsContext<M, T, E, OR, MD, D1, D2>) -> Option<DisposeOnDrop<D1>>,
    F2: FnOnce(&TwoStepsContext<M, T, E, OR, MD, D1, D2>) -> Option<DisposeOnDrop<D2>>,
{
    subscribe_with_context_opt(observer, model, |context| {
        // The first step needs no check of its own: nothing can have stopped a context that was
        // only just created, whose disposal is still held here alone and whose observer has not
        // been handed a single event.
        let subscription_1 = builder_1(context)?;
        if !context.is_active() {
            return None;
        }
        let subscription_2 = builder_2(context)?;
        Some(subscription_1.preceded_by_wrapped(subscription_2))
    })
}

/// [`subscribe_with_context`] whose builder may leave the context owning no source subscription,
/// which is what every step of it being skipped comes down to.
fn subscribe_with_context_opt<M, T, E, OR, D, MD, F>(
    observer: OR,
    model: MD,
    builder: F,
) -> DisposeOnDrop<Disposal<M, T, E, MD, D>>
where
    M: ThreadMode,
    OR: Observer<T, E>,
    D: Disposable,
    F: FnOnce(&SubscriptionContext<M, T, E, OR, MD, D>) -> Option<DisposeOnDrop<D>>,
{
    // A best-effort check of a misuse, so debug builds only: see
    // `is_auto_dispose_on_termination_observer`.
    #[cfg(debug_assertions)]
    assert!(
        !is_auto_dispose_on_termination_observer::<OR>(),
        "Do not combine subscribe_with_auto_dispose_on_termination with a context subscription. \
         subscribe_with_context handles \"auto dispose on termination\" itself."
    );
    let context = SubscriptionContext::new(observer, model);
    let disposal = context.disposal();
    if let Some(subscription) = builder(&context) {
        // The context owns the source subscription from here on. The field is private and set only
        // here, so it is still `None` and nothing is dropped under the lock. A context that already
        // stopped while `builder` ran does not run this update: it drops `subscription`, which
        // disposes the source, outside the lock.
        let _ = context.delivery.update(|resources| {
            resources.source_subscription = Some(subscription);
            UpdateOutcome::new(())
        });
    }
    disposal.into_dispose_on_drop()
}

/// What a context owns besides its observer and its queued events.
///
/// These are dropped together, outside the lock, once the context stops. When the context stops by
/// terminating, that happens after the observer was notified, so the source is disposed only after
/// downstream was told the stream ended.
struct ContextResources<MD, D: Disposable> {
    // Declared first so that what the model holds — the inner subscriptions of `switch`,
    // `merge_all` and `concat_all` — is disposed before the source subscription, the last
    // subscribed first.
    model: MD,
    /// `None` while the builder is still running.
    source_subscription: Option<DisposeOnDrop<D>>,
}

type ContextDelivery<M, T, E, OR, MD, D> = SerializedDelivery<M, T, E, OR, ContextResources<MD, D>>;

/// The shared state of an operator: its model and its downstream observer, behind one lock.
///
/// Handed to the builder of [`subscribe_with_context`], cloned into each of the operator's
/// observers, and driven through [`update`](Self::update),
/// [`update_flow`](Self::update_flow), [`send_next`](Self::send_next) and
/// [`send_termination`](Self::send_termination). `M` is the thread mode, `MD` the model, and `D`
/// the disposal of the source subscription the context owns. See the
/// [module documentation](self) for an example.
///
/// A scheduler task of the operator holds a clone too, so that the work the operator has accepted
/// runs its course even after the source has let go of its observer without a termination. That
/// forms no cycle as long as the context holds only the task's disposal, never the task, which its
/// runtime owns. Nor does it delay the release of the observer by a disposal: the source drops its
/// own clone as it is disposed, and a clone dropped once the context has stopped releases the
/// observer, whoever else still holds one.
#[derive(Educe)]
#[educe(Debug, Clone(bound()))]
pub struct SubscriptionContext<M: ThreadMode, T, E, OR, MD, D: Disposable> {
    #[educe(Debug(ignore))]
    delivery: ContextDelivery<M, T, E, OR, MD, D>,
}

impl<M: ThreadMode, T, E, OR, MD, D: Disposable> SubscriptionContext<M, T, E, OR, MD, D> {
    /// Creates a context holding `observer` and `model`, owning no source subscription yet.
    fn new(observer: OR, model: MD) -> Self {
        Self {
            delivery: SerializedDelivery::idle(
                observer,
                ContextResources {
                    model,
                    source_subscription: None,
                },
            ),
        }
    }

    /// Creates the disposal that stops this context.
    fn disposal(&self) -> Disposal<M, T, E, MD, D> {
        Disposal(self.delivery.stop_handle())
    }

    /// Whether the stream is still running: the context has not stopped, and no termination is
    /// queued behind a delivery running elsewhere.
    ///
    /// An operator that assembles its subscription in several steps asks this before a step whose
    /// side effects a stream that is already over has no use for — subscribing another source,
    /// starting a scheduler task, opening a file — as
    /// [`subscribe_with_context_two_steps`] does between its two. A stop concurrent with the
    /// check may land right after it, so it says what is pointless to start, never that what is
    /// started next outlives the check: that is what the context owning the subscription is for,
    /// as it disposes whatever it was handed once it has stopped.
    pub fn is_active(&self) -> bool {
        self.delivery.is_active()
    }
}

impl<M, T, E, OR, MD, D> SubscriptionContext<M, T, E, OR, MD, D>
where
    M: ThreadMode,
    OR: Observer<T, E>,
    D: Disposable,
{
    /// Updates the model and queues its events under one lock, then delivers them outside it.
    ///
    /// An update that emits nothing simply decides no events, and then nothing is sent here.
    ///
    /// The callback must not call external APIs or drop values that can re-enter this context.
    /// Return such values through [`UpdateOutcome::with_drop_outside`] instead.
    ///
    /// If the context's delivery has stopped, the callback is not invoked and [`DeliveryStopped`]
    /// is returned.
    pub fn update<R, DO, const EVENTS_DECIDED: bool>(
        &self,
        callback: impl FnOnce(&mut MD) -> UpdateOutcome<T, E, R, DO, EVENTS_DECIDED>,
    ) -> Result<R, DeliveryStopped> {
        self.delivery
            .update(|resources| callback(&mut resources.model))
    }

    /// [`Self::update`] for an operator's `on_next`, reporting the flow instead of a result.
    ///
    /// The flow is [`Flow::Stop`] when the result of the update says so, when delivering the events
    /// the update produced answered it, or when the context has stopped, so an operator observer
    /// can return it directly. The result is `()` when only downstream decides, or a [`Flow`] when
    /// the model decides too: under the same lock that queued the events.
    pub fn update_flow<R: IntoFlow, DO, const EVENTS_DECIDED: bool>(
        &self,
        callback: impl FnOnce(&mut MD) -> UpdateOutcome<T, E, R, DO, EVENTS_DECIDED>,
    ) -> Flow {
        match self
            .delivery
            .update_with_flow(|resources| callback(&mut resources.model))
        {
            Ok((result, flow)) => match result.into_flow() {
                Flow::Continue => flow,
                Flow::Stop => Flow::Stop,
            },
            Err(DeliveryStopped) => Flow::Stop,
        }
    }

    /// Sends `value` downstream. Returns the flow of the delivery, as [`Self::send`] does.
    pub fn send_next(&self, value: T) -> Flow {
        self.send(EventBatch::Next(value))
    }

    /// Sends `termination` downstream.
    ///
    /// Nothing is answered: a termination is the last event, so the caller is done whether it
    /// was delivered, queued behind a running delivery, or rejected by a context that had already
    /// stopped — [`Self::send`] would say [`Flow::Stop`] in every case.
    pub fn send_termination(&self, termination: Termination<E>) {
        let _ = self.send(EventBatch::Termination(termination));
    }

    /// Sends `events` downstream, delivering them now or queueing them behind a running delivery.
    ///
    /// Returns whether downstream still accepts events. [`Flow::Stop`] means the events were
    /// rejected and dropped, because the context has stopped or a termination is already queued,
    /// or that the stream is over: `events` carried a termination, or delivering them ended it.
    pub fn send(&self, events: EventBatch<T, E>) -> Flow {
        self.delivery.send(events)
    }
}

#[cfg(test)]
mod tests {
    use super::subscribe_with_context_two_steps;
    use crate::{
        disposable::{
            Disposable, callback_disposal::CallbackDisposal, dispose_on_drop::DisposeOnDrop,
        },
        observer::{Termination, callback_observer::CallbackObserver},
        thread_mode::Shared,
    };
    use std::{
        convert::Infallible,
        sync::{
            Arc, Barrier,
            atomic::{AtomicBool, Ordering},
        },
        thread,
    };

    // A stop concurrent with a step, which needs the context's own disposal: the public surface
    // only hands one out once the assembly has returned. The rest is covered from outside, in
    // `tests/subscribe_with_context.rs`.
    #[test]
    fn test_disposes_steps_started_before_a_concurrent_stop() {
        let disposed = Arc::new(AtomicBool::new(false));
        let disposed_source = disposed.clone();
        let observer = CallbackObserver::new(
            |_: i32| {},
            |_: Termination<Infallible>| {
                panic!("disposal must not terminate the observer");
            },
        );
        let _subscription = subscribe_with_context_two_steps::<Shared, _, _, _, _, _, _, _, _>(
            observer,
            (),
            |_| Some(DisposeOnDrop::new(())),
            |context| {
                let barrier = Barrier::new(2);
                thread::scope(|scope| {
                    let stop = context.disposal();
                    let barrier_thread = &barrier;
                    scope.spawn(move || {
                        barrier_thread.wait();
                        stop.dispose();
                        barrier_thread.wait();
                    });
                    barrier.wait();
                    barrier.wait();
                    Some(DisposeOnDrop::new(CallbackDisposal::new(move || {
                        disposed_source.store(true, Ordering::SeqCst)
                    })))
                })
            },
        );
        assert!(disposed.load(Ordering::SeqCst));
    }
}
