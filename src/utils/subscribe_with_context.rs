use crate::utils::serialized_delivery::{DeliveryStopped, UpdateOutcome};
use crate::{
    delegate_disposal,
    disposable::{
        Disposable, DisposableExt, boxed_disposal::BoxedDisposal, chain_disposal::ChainDisposal,
    },
    observable::Subscription,
    observer::{Observer, Termination},
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
    Disposal<'or, D>,
    ChainDisposal<BoxedDisposal<'or>, D>,
    where D: Disposable
);

/// The type-erased disposal of a context that owns its source subscription. Returned by
/// [`subscribe_with_context_owning_source`].
pub type OwningDisposal<'or> = BoxedDisposal<'or>;

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
pub fn subscribe_with_context<'or, T, E, OR, D, M, F>(
    observer: OR,
    model: M,
    builder: F,
) -> Subscription<Disposal<'or, D>>
where
    T: MaybeSend + 'or,
    E: MaybeSend + 'or,
    OR: MaybeSend + 'or,
    D: Disposable,
    M: MaybeSend + 'or,
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
/// costs `D: MaybeSend + 'or` and erases the disposal into [`OwningDisposal`], so when
/// the context can only terminate from inside the source's own `on_termination` prefer
/// [`subscribe_with_context`], which keeps `D` concrete.
pub fn subscribe_with_context_owning_source<'or, T, E, OR, D, M, F>(
    observer: OR,
    model: M,
    builder: F,
) -> Subscription<OwningDisposal<'or>>
where
    T: MaybeSend + 'or,
    E: MaybeSend + 'or,
    OR: Observer<T, E> + MaybeSend + 'or,
    D: Disposable + MaybeSend + 'or,
    M: MaybeSend + 'or,
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

/// Context used by operator observers to serialize model updates and downstream events.
///
/// `D` is the disposal of the source subscription the context owns, and `()` when it owns none.
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

    /// Sends `value` downstream. Returns whether it was accepted, as [`Self::send`] does.
    pub fn send_next(&self, value: T) -> bool {
        self.send(EventBatch::Next(value))
    }

    /// Sends `termination` downstream. Returns whether it was accepted, as [`Self::send`] does.
    pub fn send_termination(&self, termination: Termination<E>) -> bool {
        self.send(EventBatch::Termination(termination))
    }

    /// Sends `events` downstream, delivering them now or queueing them behind a running delivery.
    ///
    /// Returns whether the events were accepted. They are rejected, and dropped, once the context
    /// has stopped or a termination is already queued.
    pub fn send(&self, events: EventBatch<T, E>) -> bool {
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
