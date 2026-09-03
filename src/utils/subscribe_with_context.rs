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
        types::MaybeSend,
    },
};
use educe::Educe;

delegate_disposal!(
    Disposal<'or, D>,
    ChainDisposal<BoxedDisposal<'or>, D>,
    where D: Disposable
);

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
/// [`subscribe_with_context_bound_subscription`] so that the source is disposed on termination.
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
    // This context does not own its source subscription: the caller chains it below instead.
    let delivery = SerializedDelivery::idle(
        observer,
        ContextResources {
            model,
            subscription: None,
        },
    );
    let context = SubscriptionContext {
        delivery: delivery.clone(),
    };
    let disposable = SubscriptionContextDisposal { delivery };
    let subscription = builder(context);
    subscription.preceded_by(disposable.into_boxed()).map_into()
}

/// Type-erased disposal returned when the context owns the source subscription.
pub type BoundSubscriptionDisposal<'or> = BoxedDisposal<'or>;

/// Creates a context subscription whose source subscription is owned by the context.
///
/// Owning the source subscription lets the context dispose it automatically when the observer
/// terminates, including when termination occurs synchronously while `builder` is running.
///
/// Use this whenever the context can terminate while the source is still active — from a notifier,
/// from a scheduler task, or from another source of a multi-source operator. Owning the source
/// costs `D: MaybeSend + 'or` and erases the disposal into [`BoundSubscriptionDisposal`], so when
/// the context can only terminate from inside the source's own `on_termination` prefer
/// [`subscribe_with_context`], which keeps `D` concrete.
pub fn subscribe_with_context_bound_subscription<'or, T, E, OR, D, M, F>(
    observer: OR,
    model: M,
    builder: F,
) -> Subscription<BoundSubscriptionDisposal<'or>>
where
    T: MaybeSend + 'or,
    E: MaybeSend + 'or,
    OR: Observer<T, E> + MaybeSend + 'or,
    D: Disposable + MaybeSend + 'or,
    M: MaybeSend + 'or,
    F: FnOnce(SubscriptionContext<T, E, OR, M, D>) -> Subscription<D>,
{
    debug_assert_observer_compatibility::<OR>();
    let delivery = SerializedDelivery::idle(
        observer,
        ContextResources {
            model,
            subscription: None,
        },
    );
    let context = SubscriptionContext {
        delivery: delivery.clone(),
    };
    let disposable = SubscriptionContextDisposal {
        delivery: delivery.clone(),
    };
    let subscription = builder(context);
    // If the context stopped while the builder was running, the update never runs and the
    // subscription is dropped with this closure, outside the lock.
    let _ = delivery.update(|resources| {
        debug_assert!(
            resources.subscription.is_none(),
            "the bound subscription is installed only once"
        );
        resources.subscription = Some(subscription);
        UpdateOutcome::empty()
    });
    disposable.into_boxed().into_subscription()
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
    /// `None` when the context does not own its source subscription, or while a bound
    /// subscription's builder is still running.
    subscription: Option<Subscription<D>>,
}

type ContextDelivery<T, E, OR, M, D> = SerializedDelivery<T, E, OR, ContextResources<M, D>>;
type WeakContextDelivery<T, E, OR, M, D> = WeakSerializedDelivery<T, E, OR, ContextResources<M, D>>;

/// Context used by operator observers to serialize model updates and downstream events.
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct SubscriptionContext<T, E, OR, M, D: Disposable = ()> {
    delivery: ContextDelivery<T, E, OR, M, D>,
}

impl<T, E, OR, M, D> SubscriptionContext<T, E, OR, M, D>
where
    OR: Observer<T, E>,
    D: Disposable,
{
    /// Updates the model and sends the events that update produced, while the context is locked.
    ///
    /// The callback must not call external APIs or drop values that can re-enter this context.
    /// Return such values through [`UpdateOutcome::with_drop_outside`] instead.
    /// If the context's delivery has stopped, the callback is not invoked and [`DeliveryStopped`]
    /// is returned.
    pub fn update_model_and_send<R, DO, const EVENTS_DECIDED: bool>(
        &self,
        callback: impl FnOnce(&mut M) -> UpdateOutcome<T, E, R, DO, EVENTS_DECIDED>,
    ) -> Result<R, DeliveryStopped> {
        self.delivery
            .update(|resources| callback(&mut resources.model))
    }

    pub fn send_next(&self, value: T) {
        self.send(EventBatch::Next(value));
    }

    pub fn send_termination(&self, termination: Termination<E>) {
        self.send(EventBatch::Termination(termination));
    }

    pub fn send(&self, events: EventBatch<T, E>) {
        self.delivery.send(events);
    }

    pub fn downgrade(&self) -> WeakSubscriptionContext<T, E, OR, M, D> {
        WeakSubscriptionContext {
            delivery: self.delivery.downgrade(),
        }
    }
}

struct SubscriptionContextDisposal<T, E, OR, M, D: Disposable> {
    delivery: ContextDelivery<T, E, OR, M, D>,
}

impl<T, E, OR, M, D: Disposable> Disposable for SubscriptionContextDisposal<T, E, OR, M, D> {
    fn dispose(self) {
        self.delivery.stop(); // Stops the context, so that every later event is dropped
    }
}

/// A non-owning reference to a [`SubscriptionContext`].
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct WeakSubscriptionContext<T, E, OR, M, D: Disposable = ()> {
    delivery: WeakContextDelivery<T, E, OR, M, D>,
}

impl<T, E, OR, M, D: Disposable> WeakSubscriptionContext<T, E, OR, M, D> {
    pub fn upgrade(&self) -> Option<SubscriptionContext<T, E, OR, M, D>> {
        self.delivery
            .upgrade()
            .map(|delivery| SubscriptionContext { delivery })
    }
}

fn debug_assert_observer_compatibility<OR>() {
    debug_assert!(
        {
            fn type_name_without_generics<T>() -> &'static str {
                std::any::type_name::<T>()
                    .split_once('<')
                    .map_or_else(|| std::any::type_name::<T>(), |(name, _)| name)
            }

            type_name_without_generics::<OR>()
                != type_name_without_generics::<
                    crate::utils::subscribe_with_auto_dispose_on_termination::AutoDisposeOnTerminationObserver<
                        (),
                        (),
                    >,
                >()
        },
        "Do not combine subscribe_with_auto_dispose_on_termination with a context subscription. Using subscribe_with_context_bound_subscription handles \"auto dispose on termination\"."
    );
}
