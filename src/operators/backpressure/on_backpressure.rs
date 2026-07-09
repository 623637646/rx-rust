use crate::observable::Subscription;
use crate::utils::subscribe_with_shared_model::{
    self, Context, ModificationResult, WeakContext, subscribe_with_shared_model,
};
use crate::utils::types::{MaybeSend, MaybeSync, Shared};
use crate::{
    observable::Observable,
    observer::{Observer, Termination},
};
use educe::Educe;

trait RequestHandler<'or>: MaybeSend + MaybeSync {
    fn request(self: Shared<Self>);
}

type SharedRequestHandler<'or> = Shared<dyn RequestHandler<'or> + 'or>;

/// A single-use token handed to the downstream observer to request the next chunk.
///
/// Tokens share one request handler allocated when the subscription is created, so
/// requesting additional chunks does not allocate or clone the shared handler.
///
/// The handler holds only a weak reference to the subscription's state, so a token
/// does not keep the subscription alive: calling [`request`](Self::request) after
/// the subscription has been disposed is a no-op.
pub struct RequestToken<'or>(SharedRequestHandler<'or>);

impl RequestToken<'_> {
    pub fn request(self) {
        self.0.request();
    }
}

impl std::fmt::Debug for RequestToken<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(std::any::type_name::<Self>())
    }
}

pub trait BackpressureCollection {
    type Input;
    type Output;

    fn extend_one(&mut self, item: Self::Input);

    /// Returns the next output, or `None` when more input is required.
    fn take_next_value(&mut self) -> Option<Self::Output>;
}

/// Low-level primitive that converts a fast upstream into demand-driven chunks by
/// accumulating values with a custom `collection` and emitting them alongside a
/// [`RequestToken`]. Downstream observers must call [`RequestToken::request`] to resume the
/// upstream flow. See <https://reactivex.io/documentation/operators/backpressure.html>.
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct OnBackpressure<OE, C> {
    source: OE,
    collection: C,
}

impl<OE, C> OnBackpressure<OE, C> {
    pub fn new<'or, E>(source: OE, collection: C) -> Self
    where
        C: BackpressureCollection,
        OE: Observable<'or, C::Input, E>,
    {
        Self { source, collection }
    }
}

impl<'or_sub, E, OE, C> Observable<'or_sub, (C::Output, RequestToken<'or_sub>), E>
    for OnBackpressure<OE, C>
where
    E: MaybeSend + 'or_sub,
    OE: Observable<'or_sub, C::Input, E>,
    OE::D: MaybeSend + 'or_sub,
    C: BackpressureCollection + MaybeSend + 'or_sub,
    C::Output: MaybeSend + 'or_sub,
{
    type D = subscribe_with_shared_model::Disposal<'or_sub>;

    fn subscribe(
        self,
        observer: impl Observer<(C::Output, RequestToken<'or_sub>), E> + MaybeSend + 'or_sub,
    ) -> Subscription<Self::D> {
        let model = Model {
            collection: self.collection,
            termination: None,
            downstream_ready: true,
        };
        subscribe_with_shared_model(observer, model, |context| {
            let request_handler: SharedRequestHandler<'or_sub> = Shared::new(context.downgrade());
            self.source.subscribe(ObserverImpl {
                context,
                request_handler,
            })
        })
    }
}

struct Model<E, C> {
    collection: C,
    termination: Option<Termination<E>>,
    downstream_ready: bool,
}

type BackpressureContext<'or, E, OR, C> =
    Context<(<C as BackpressureCollection>::Output, RequestToken<'or>), E, OR, Model<E, C>>;

impl<'or, E, OR, C> RequestHandler<'or>
    for WeakContext<(C::Output, RequestToken<'or>), E, OR, Model<E, C>>
where
    E: MaybeSend + 'or,
    OR: Observer<(C::Output, RequestToken<'or>), E> + MaybeSend + 'or,
    C: BackpressureCollection + MaybeSend + 'or,
    C::Output: MaybeSend + 'or,
{
    fn request(self: Shared<Self>) {
        let Some(context) = self.upgrade() else {
            return;
        };
        let _ = context.modify_model(|model| {
            if let Some(next) = model.collection.take_next_value() {
                ModificationResult::new_send_next((next, RequestToken(self)))
            } else if let Some(termination) = model.termination.take() {
                ModificationResult::new_send_termination(termination)
            } else {
                model.downstream_ready = true;
                ModificationResult::new_without_result()
            }
        });
    }
}

struct ObserverImpl<'or, E, OR, C>
where
    C: BackpressureCollection,
{
    context: BackpressureContext<'or, E, OR, C>,
    request_handler: SharedRequestHandler<'or>,
}

impl<'or, E, OR, C> Observer<C::Input, E> for ObserverImpl<'or, E, OR, C>
where
    E: MaybeSend + 'or,
    OR: Observer<(C::Output, RequestToken<'or>), E> + MaybeSend + 'or,
    C: BackpressureCollection + MaybeSend + 'or,
    C::Output: MaybeSend + 'or,
{
    fn on_next(&mut self, value: C::Input) {
        let _ = self.context.modify_model(|model| {
            if model.termination.is_some() {
                return ModificationResult::new_without_result();
            }
            model.collection.extend_one(value);
            if model.downstream_ready {
                if let Some(next) = model.collection.take_next_value() {
                    model.downstream_ready = false;
                    let request = RequestToken(self.request_handler.clone());
                    ModificationResult::new_send_next((next, request))
                } else {
                    ModificationResult::new_without_result()
                }
            } else {
                ModificationResult::new_without_result()
            }
        });
    }

    fn on_termination(self, termination: Termination<E>) {
        let _ = self.context.modify_model(|model| {
            if model.downstream_ready {
                ModificationResult::new_send_termination(termination)
            } else {
                model.termination = Some(termination);
                ModificationResult::new_without_result()
            }
        });
    }
}
