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
//!     observable::{Observable, ObservableExt, Subscription},
//!     observer::{Flow, Observer, Termination},
//!     operators::creating::range::Range,
//!     utils::{
//!         serialized_delivery::UpdateOutcome,
//!         subscribe_with_context::{self, subscribe_with_context_owning_source, SubscriptionContext},
//!         types::MaybeSend,
//!     },
//!     disposable::Disposable,
//! };
//!
//! struct TotalUntil<OE> { source: OE, limit: i32 }
//!
//! impl<'or, E: MaybeSend + 'or, OE: Observable<'or, i32, E>> Observable<'or, i32, E> for TotalUntil<OE>
//! where
//!     OE::D: MaybeSend + 'or,
//! {
//!     type D = subscribe_with_context::OwningDisposal<'or>;
//!
//!     fn subscribe(self, observer: impl Observer<i32, E> + MaybeSend + 'or) -> Subscription<Self::D> {
//!         let limit = self.limit;
//!         subscribe_with_context_owning_source(observer, 0, |context| {
//!             self.source.subscribe(TotalObserver { context, limit })
//!         })
//!     }
//! }
//!
//! struct TotalObserver<OR, E, D: Disposable> { context: SubscriptionContext<i32, E, OR, i32, D>, limit: i32 }
//!
//! impl<OR: Observer<i32, E>, E, D: Disposable> Observer<i32, E> for TotalObserver<OR, E, D> {
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
    disposable::{
        Disposable, DisposableExt, boxed_disposal::BoxedDisposal, chain_disposal::ChainDisposal,
    },
    observable::Subscription,
    observer::{Flow, Observer, Termination},
    utils::{
        pending_events::EventBatch,
        serialized_delivery::{SerializedDelivery, WeakSerializedDelivery},
        subscribe_with_auto_dispose_on_termination::is_auto_dispose_on_termination_observer,
        types::MaybeSend,
    },
};
use educe::Educe;

// The disposal of a context that does not own its source subscription: stopping the context,
// followed by the caller's own subscription. Returned by `subscribe_with_context`.
delegate_disposal!(
    Disposal<'or_sub, D>,
    ChainDisposal<BoxedDisposal<'or_sub>, D>,
    where D: Disposable
);

/// The type-erased disposal of a context that owns its source subscription. Returned by
/// [`subscribe_with_context_owning_source`].
pub type OwningDisposal<'or_sub> = BoxedDisposal<'or_sub>;

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
pub fn subscribe_with_context<'or_sub, T, E, OR, D, M, F>(
    observer: OR,
    model: M,
    builder: F,
) -> Subscription<Disposal<'or_sub, D>>
where
    T: MaybeSend + 'or_sub,
    E: MaybeSend + 'or_sub,
    OR: MaybeSend + 'or_sub,
    D: Disposable,
    M: MaybeSend + 'or_sub,
    F: FnOnce(SubscriptionContext<T, E, OR, M>) -> Subscription<D>,
{
    debug_assert_observer_compatibility::<OR>();
    // This context does not own its source subscription, so its own `D` is `()`: the caller's
    // subscription, of the unrelated type `D`, is chained below instead.
    let context = SubscriptionContext::<T, E, OR, M, ()>::new(observer, model);
    let disposal = context.disposal();
    let subscription = builder(context);
    subscription.preceded_by(disposal.into_boxed()).map_into()
}

/// Creates a context subscription whose source subscription is owned by the context.
///
/// Owning the source subscription lets the context dispose it automatically when the observer
/// terminates, including when termination occurs synchronously while `builder` is running.
///
/// Use this whenever the context can terminate while the source is still active — from a notifier,
/// from a scheduler task, or from another source of a multi-source operator. Owning the source
/// costs `D: MaybeSend + 'or_sub` and erases the disposal into [`OwningDisposal`], so when
/// the context can only terminate from inside the source's own `on_termination` prefer
/// [`subscribe_with_context`], which keeps `D` concrete.
pub fn subscribe_with_context_owning_source<'or_sub, T, E, OR, D, M, F>(
    observer: OR,
    model: M,
    builder: F,
) -> Subscription<OwningDisposal<'or_sub>>
where
    T: MaybeSend + 'or_sub,
    E: MaybeSend + 'or_sub,
    OR: Observer<T, E> + MaybeSend + 'or_sub,
    D: Disposable + MaybeSend + 'or_sub,
    M: MaybeSend + 'or_sub,
    F: FnOnce(SubscriptionContext<T, E, OR, M, D>) -> Subscription<D>,
{
    debug_assert_observer_compatibility::<OR>();
    let context = SubscriptionContext::<T, E, OR, M, D>::new(observer, model);
    let disposal = context.disposal();
    let subscription = builder(context.clone());
    let previous_subscription = context.install_source_subscription(subscription);
    debug_assert!(
        !matches!(previous_subscription, Ok(Some(_))),
        "the source subscription is installed only once"
    );
    disposal.into_boxed().into_subscription()
}

/// What a context owns besides its observer and its queued events.
///
/// These are dropped together, outside the lock, once the context stops. When the context stops by
/// terminating, that happens after the observer was notified, so the source is disposed only after
/// downstream was told the stream ended.
#[derive(Educe)]
#[educe(Debug)]
struct ContextResources<M, D: Disposable> {
    model: M,
    /// `None` when the context does not own its source subscription — `D` is then `()` — or
    /// while the builder of an owned source subscription is still running.
    source_subscription: Option<Subscription<D>>,
}

type ContextDelivery<T, E, OR, M, D> = SerializedDelivery<T, E, OR, ContextResources<M, D>>;
type WeakContextDelivery<T, E, OR, M, D> = WeakSerializedDelivery<T, E, OR, ContextResources<M, D>>;

/// The shared state of an operator: its model and its downstream observer, behind one lock.
///
/// Handed to the builder of [`subscribe_with_context`] / [`subscribe_with_context_owning_source`],
/// cloned into each of the operator's observers, and driven through [`update`](Self::update),
/// [`update_flow`](Self::update_flow), [`send_next`](Self::send_next) and
/// [`send_termination`](Self::send_termination). `D` is the disposal of the source subscription
/// the context owns, and `()` when it owns none. See the [module documentation](self) for an
/// example.
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct SubscriptionContext<T, E, OR, M, D: Disposable = ()> {
    delivery: ContextDelivery<T, E, OR, M, D>,
}

impl<T, E, OR, M, D: Disposable> SubscriptionContext<T, E, OR, M, D> {
    /// Creates a context holding `observer` and `model`, owning no source subscription yet.
    fn new(observer: OR, model: M) -> Self {
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
    fn disposal(&self) -> SubscriptionContextDisposal<T, E, OR, M, D> {
        SubscriptionContextDisposal {
            delivery: self.delivery.clone(),
        }
    }

    /// Creates a non-owning reference to this context.
    pub fn downgrade(&self) -> WeakSubscriptionContext<T, E, OR, M, D> {
        WeakSubscriptionContext {
            delivery: self.delivery.downgrade(),
        }
    }
}

impl<T, E, OR, M, D> SubscriptionContext<T, E, OR, M, D>
where
    OR: Observer<T, E>,
    D: Disposable,
{
    /// Updates the model and sends the events that update produced, while the context is locked.
    ///
    /// An update that emits nothing simply decides no events, and then nothing is sent here.
    ///
    /// The callback must not call external APIs or drop values that can re-enter this context.
    /// Return such values through [`UpdateOutcome::with_drop_outside`] instead.
    /// If the context's delivery has stopped, the callback is not invoked and [`DeliveryStopped`]
    /// is returned.
    pub fn update<R, DO, const EVENTS_DECIDED: bool>(
        &self,
        callback: impl FnOnce(&mut M) -> UpdateOutcome<T, E, R, DO, EVENTS_DECIDED>,
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
        callback: impl FnOnce(&mut M) -> UpdateOutcome<T, E, (), DO, EVENTS_DECIDED>,
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

struct SubscriptionContextDisposal<T, E, OR, M, D: Disposable> {
    delivery: ContextDelivery<T, E, OR, M, D>,
}

impl<T, E, OR, M, D: Disposable> Disposable for SubscriptionContextDisposal<T, E, OR, M, D> {
    /// Stops the context, so that every later event is dropped.
    fn dispose(self) {
        self.delivery.stop();
    }
}

/// A non-owning reference to a [`SubscriptionContext`].
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct WeakSubscriptionContext<T, E, OR, M, D: Disposable = ()> {
    delivery: WeakContextDelivery<T, E, OR, M, D>,
}

impl<T, E, OR, M, D: Disposable> WeakSubscriptionContext<T, E, OR, M, D> {
    /// Returns the context, or `None` once every strong reference to it is gone.
    pub fn upgrade(&self) -> Option<SubscriptionContext<T, E, OR, M, D>> {
        self.delivery
            .upgrade()
            .map(|delivery| SubscriptionContext { delivery })
    }
}

fn debug_assert_observer_compatibility<OR>() {
    debug_assert!(
        !is_auto_dispose_on_termination_observer::<OR>(),
        "Do not combine subscribe_with_auto_dispose_on_termination with a context subscription. \
         Using subscribe_with_context_owning_source handles \"auto dispose on termination\"."
    );
}
