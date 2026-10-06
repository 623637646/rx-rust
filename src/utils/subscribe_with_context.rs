//! The helper most stateful operators are written with.
//!
//! A [`SubscriptionContext`] holds the downstream observer and the operator's own model behind one
//! lock (a [`SerializedDelivery`]). The
//! operator's observers update the model and queue events in one step, and the events are
//! delivered outside the lock, in order, whichever thread produced them. That is what makes an
//! operator with several sources, a notifier or a scheduler task correct without any locking of
//! its own.
//!
//! Two entry points differ in who owns the source subscription — see each for when to use it:
//! [`subscribe_with_context`] chains it after the context, [`subscribe_with_context_owning_source`]
//! puts it inside, so that the context disposes it when it terminates on its own.
//!
//! # Examples
//! An operator that emits the running total and completes on its own once it exceeds a limit —
//! which is why it owns its source:
//! ```rust
//! use rx_rust::{
//!     observable::{Observable, ObservableExt, ObservableTypes, Subscription},
//!     observer::{Flow, Observer, Termination},
//!     operators::creating::range::Range,
//!     thread_mode::ThreadMode,
//!     utils::{
//!         serialized_delivery::UpdateOutcome,
//!         subscribe_with_context::{self, subscribe_with_context_owning_source, SubscriptionContext},
//!     },
//!     disposable::Disposable,
//! };
//!
//! struct TotalUntil<OE> { source: OE, limit: i32 }
//!
//! // Nothing here depends on the observer: the disposal names the mode, the events, the model and
//! // the source's disposal, never the observer's type.
//! impl<E, OE: ObservableTypes<Item = i32, Error = E>> ObservableTypes for TotalUntil<OE> {
//!     type Item = i32;
//!     type Error = E;
//!     type Mode = OE::Mode;
//!     type Disposal =
//!         subscribe_with_context::ContextDisposal<OE::Mode, i32, E, i32, OE::Disposal>;
//! }
//!
//! // The source is subscribed with the operator's own observer, which is named here. A disposal
//! // written inside the bound of `OE` is spelled out in full.
//! impl<E, OE, OR> Observable<OR> for TotalUntil<OE>
//! where
//!     OR: Observer<i32, E>,
//!     OE: Observable<
//!             TotalObserver<
//!                 <OE as ObservableTypes>::Mode,
//!                 OR,
//!                 E,
//!                 <OE as ObservableTypes>::Disposal,
//!             >,
//!             Item = i32,
//!             Error = E,
//!         >,
//! {
//!     fn subscribe(self, observer: OR) -> Subscription<Self::Disposal> {
//!         let limit = self.limit;
//!         subscribe_with_context_owning_source(observer, 0, |context| {
//!             self.source.subscribe(TotalObserver { context, limit })
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
//!                 UpdateOutcome::empty().with_next_and_termination_events(*total, Termination::Completed)
//!             } else {
//!                 UpdateOutcome::empty().with_next_event(*total)
//!             }
//!         })
//!     }
//!     fn on_termination(self, termination: Termination<E>) {
//!         self.context.send_termination(termination);
//!     }
//! }
//!
//! let mut seen = Vec::new();
//! TotalUntil { source: Range::new(1..), limit: 5 }
//!     .subscribe_with_callback(|total| seen.push(total), |t| assert_eq!(t, Termination::Completed));
//! assert_eq!(seen, [1, 3, 6]);
//! ```

use crate::utils::serialized_delivery::{DeliveryStopped, UpdateOutcome};
use crate::{
    delegate_disposal,
    disposable::{Disposable, DisposableExt, chain_disposal::ChainDisposal},
    observable::Subscription,
    observer::{Flow, Observer, Termination},
    thread_mode::ThreadMode,
    utils::{
        pending_events::EventBatch,
        serialized_delivery::{DeliveryStop, SerializedDelivery},
        subscribe_with_auto_dispose_on_termination::is_auto_dispose_on_termination_observer,
    },
};
use educe::Educe;

delegate_disposal!(
    /// The disposal of a context that does not own its source subscription: stopping the context,
    /// followed by the caller's own subscription. Returned by [`subscribe_with_context`].
    Disposal<M, T, E, MD, D>,
    ChainDisposal<ContextDisposal<M, T, E, MD, ()>, D>,
    where M: ThreadMode, D: Disposable
);

/// Stops a context: its model and the source subscription it owns are dropped, outside the lock,
/// and every later event is rejected.
///
/// Returned by [`subscribe_with_context_owning_source`], where `D` is the disposal of the source
/// subscription the context owns. [`subscribe_with_context`] uses it with `D = ()`, chained before
/// the caller's own subscription.
///
/// Its type names the thread mode, the events, the model and the source's disposal, but not the
/// observer — the disposal of an observable must not depend on its observer (see
/// [`ObservableTypes`](crate::observable::ObservableTypes)). The observer is released when the
/// sources that hold the context let go of it, which the disposal of the model and the sources
/// makes them do.
#[derive(Educe)]
#[educe(Debug)]
pub struct ContextDisposal<M: ThreadMode, T, E, MD, D: Disposable>(
    #[educe(Debug(ignore))] DeliveryStop<M, T, E, ContextResources<MD, D>>,
);

impl<M: ThreadMode, T, E, MD, D: Disposable> Disposable for ContextDisposal<M, T, E, MD, D> {
    fn dispose(self) {
        self.0.stop();
    }
}

/// Creates a subscription backed by a shared, serialized context containing the downstream
/// observer and a mutable model.
///
/// The context does not own the source subscription: the caller's subscription is chained after
/// the context's disposal, so the source is disposed only once downstream drops the returned
/// subscription. Use this when the context can only terminate from inside the source's own
/// `on_termination` — directly, or in a continuation of it, such as a scheduler task that delivers
/// a termination the source had already parked in the model. The source is then finished by the
/// time the context terminates, so owning it would buy nothing.
///
/// When the context can instead terminate while the source is still active — from a notifier, from
/// a scheduler task, or from another source of a multi-source operator — use
/// [`subscribe_with_context_owning_source`] so that the source is disposed on termination.
///
/// `M` is the thread mode the context's pointers are picked for: the mode of the operator.
pub fn subscribe_with_context<M, T, E, OR, D, MD, F>(
    observer: OR,
    model: MD,
    builder: F,
) -> Subscription<Disposal<M, T, E, MD, D>>
where
    M: ThreadMode,
    D: Disposable,
    F: FnOnce(SubscriptionContext<M, T, E, OR, MD>) -> Subscription<D>,
{
    debug_assert_observer_compatibility::<OR>();
    // This context does not own its source subscription, so its own `D` is `()`: the caller's
    // subscription, of the unrelated type `D`, is chained below instead.
    let context = SubscriptionContext::<M, T, E, OR, MD, ()>::new(observer, model);
    let disposal = context.disposal();
    let subscription = builder(context);
    subscription.preceded_by(disposal).map_into()
}

/// Creates a context subscription whose source subscription is owned by the context.
///
/// Owning the source subscription lets the context dispose it automatically when the observer
/// terminates, including when termination occurs synchronously while `builder` is running.
///
/// Use this whenever the context can terminate while the source is still active — from a notifier,
/// from a scheduler task, or from another source of a multi-source operator. When the context can
/// only terminate from inside the source's own `on_termination`, [`subscribe_with_context`] is
/// enough.
pub fn subscribe_with_context_owning_source<M, T, E, OR, D, MD, F>(
    observer: OR,
    model: MD,
    builder: F,
) -> Subscription<ContextDisposal<M, T, E, MD, D>>
where
    M: ThreadMode,
    OR: Observer<T, E>,
    D: Disposable,
    F: FnOnce(SubscriptionContext<M, T, E, OR, MD, D>) -> Subscription<D>,
{
    debug_assert_observer_compatibility::<OR>();
    let context = SubscriptionContext::<M, T, E, OR, MD, D>::new(observer, model);
    let disposal = context.disposal();
    let subscription = builder(context.clone());
    let previous_subscription = context.install_source_subscription(subscription);
    debug_assert!(
        !matches!(previous_subscription, Ok(Some(_))),
        "the source subscription is installed only once"
    );
    disposal.into_subscription()
}

/// What a context owns besides its observer and its queued events.
///
/// These are dropped together, outside the lock, once the context stops. When the context stops by
/// terminating, that happens after the observer was notified, so the source is disposed only after
/// downstream was told the stream ended.
struct ContextResources<MD, D: Disposable> {
    model: MD,
    /// `None` when the context does not own its source subscription — `D` is then `()` — or
    /// while the builder of an owned source subscription is still running.
    source_subscription: Option<Subscription<D>>,
}

type ContextDelivery<M, T, E, OR, MD, D> = SerializedDelivery<M, T, E, OR, ContextResources<MD, D>>;

/// The shared state of an operator: its model and its downstream observer, behind one lock.
///
/// Handed to the builder of [`subscribe_with_context`] / [`subscribe_with_context_owning_source`],
/// cloned into each of the operator's observers, and driven through [`update`](Self::update),
/// [`update_flow`](Self::update_flow), [`send_next`](Self::send_next) and
/// [`send_termination`](Self::send_termination). `M` is the thread mode, `MD` the model, and `D`
/// the disposal of the source subscription the context owns, `()` when it owns none. See the
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
pub struct SubscriptionContext<M: ThreadMode, T, E, OR, MD, D: Disposable = ()> {
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
    fn disposal(&self) -> ContextDisposal<M, T, E, MD, D> {
        ContextDisposal(self.delivery.stop_handle())
    }
}

impl<M, T, E, OR, MD, D> SubscriptionContext<M, T, E, OR, MD, D>
where
    M: ThreadMode,
    OR: Observer<T, E>,
    D: Disposable,
{
    /// Updates the model and sends the events that update produced, while the context is locked.
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
    /// The flow is what delivering the events the update produced answered, and [`Flow::Stop`]
    /// when the context has stopped, so an operator observer can return it directly.
    pub fn update_flow<DO, const EVENTS_DECIDED: bool>(
        &self,
        callback: impl FnOnce(&mut MD) -> UpdateOutcome<T, E, (), DO, EVENTS_DECIDED>,
    ) -> Flow {
        match self
            .delivery
            .update_with_flow(|resources| callback(&mut resources.model))
        {
            Ok(((), flow)) => flow,
            Err(DeliveryStopped) => Flow::Stop,
        }
    }

    /// Gives the context the source subscription it owns, to be disposed once the context stops.
    ///
    /// The subscription it replaces — none, unless it is installed twice — is handed back so that
    /// it is dropped outside the lock. Once the context has stopped, `subscription` is not
    /// installed but disposed, outside the lock, and [`DeliveryStopped`] is returned.
    fn install_source_subscription(
        &self,
        subscription: Subscription<D>,
    ) -> Result<Option<Subscription<D>>, DeliveryStopped> {
        self.delivery.update(|resources| {
            UpdateOutcome::new(resources.source_subscription.replace(subscription))
        })
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

fn debug_assert_observer_compatibility<OR>() {
    debug_assert!(
        !is_auto_dispose_on_termination_observer::<OR>(),
        "Do not combine subscribe_with_auto_dispose_on_termination with a context subscription. \
         Using subscribe_with_context_owning_source handles \"auto dispose on termination\"."
    );
}
