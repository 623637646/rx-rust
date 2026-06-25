use crate::utils::subscribe_with_shared_model::{Action, Context, subscribe_with_shared_model};
use crate::utils::types::{MarkerType, NecessarySend};
use crate::{
    disposable::subscription::Subscription,
    observable::Observable,
    observer::{Observer, Termination},
};
use educe::Educe;

cfg_if::cfg_if! {
    if #[cfg(feature = "single-threaded")] {
        /// Callback handed to the downstream observer so it can request the next chunk
        /// once it finishes processing the current batch.
        pub type RequestCallbackType<'or> = Box<dyn FnOnce() + 'or>;
    } else {
        /// Callback handed to the downstream observer so it can request the next chunk
        /// once it finishes processing the current batch.
        pub type RequestCallbackType<'or> = Box<dyn FnOnce() + Send + Sync + 'or>;
    }
}

pub trait BackpressureCollection<T0, T> {
    fn extend_one(&mut self, item: T0);
    fn take_next_value(&mut self) -> Option<T>;
}

/// Low-level primitive that converts a fast upstream into demand-driven chunks by
/// accumulating values with a custom `collection` and emitting them alongside a
/// [`RequestCallbackType`]. Downstream observers must invoke the callback to resume the
/// upstream flow. See <https://reactivex.io/documentation/operators/backpressure.html>.
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct OnBackpressure<T0, OE, C> {
    source: OE,
    collection: C,
    _marker: MarkerType<T0>,
}

impl<T0, OE, C> OnBackpressure<T0, OE, C> {
    pub fn new<'or, 'sub, E>(source: OE, collection: C) -> Self
    where
        OE: Observable<'or, 'sub, T0, E>,
    {
        Self {
            source,
            collection,
            _marker: Default::default(),
        }
    }
}

impl<'or, 'sub, T0, T, E, OE, C> Observable<'or, 'sub, (T, RequestCallbackType<'or>), E>
    for OnBackpressure<T0, OE, C>
where
    'or: 'sub,
    T: NecessarySend + 'or,
    E: NecessarySend + 'or,
    OE: Observable<'or, 'sub, T0, E>,
    C: BackpressureCollection<T0, T> + NecessarySend + 'or,
{
    fn subscribe(
        self,
        observer: impl Observer<(T, RequestCallbackType<'or>), E> + NecessarySend + 'or,
    ) -> Subscription<'sub> {
        let model = Model {
            collection: self.collection,
            termination: None,
            emit_directly: true,
        };
        subscribe_with_shared_model(observer, model, |context| {
            self.source.subscribe(ObserverImpl(context))
        })
    }
}

struct Model<E, C> {
    collection: C,
    termination: Option<Termination<E>>,
    emit_directly: bool,
}

struct ObserverImpl<'or, T, E, OR, C>(Context<(T, RequestCallbackType<'or>), E, OR, Model<E, C>>);

impl<'or, T0, T, E, OR, C> Observer<T0, E> for ObserverImpl<'or, T, E, OR, C>
where
    T: NecessarySend + 'or,
    E: NecessarySend + 'or,
    OR: Observer<(T, RequestCallbackType<'or>), E> + NecessarySend + 'or,
    C: BackpressureCollection<T0, T> + NecessarySend + 'or,
{
    fn on_next(&mut self, value: T0) {
        self.0.modify_model_with_action(|model| {
            if model.termination.is_some() {
                return Action::None;
            }
            model.collection.extend_one(value);
            if model.emit_directly {
                model.emit_directly = false;
                let next = model.collection.take_next_value().expect("cannot be empty");
                let context = self.0.clone();
                let callback: RequestCallbackType = Box::new(move || {
                    handle_request(context);
                });
                Action::SendNext((next, callback))
            } else {
                Action::None
            }
        });
    }

    fn on_termination(self, termination: Termination<E>) {
        self.0.modify_model_with_action(|model| {
            if model.emit_directly {
                Action::SendTermination(termination)
            } else {
                model.termination = Some(termination);
                Action::None
            }
        });
    }
}

fn handle_request<'or, T0, T, E, OR, C>(
    context: Context<(T, RequestCallbackType<'or>), E, OR, Model<E, C>>,
) where
    T: NecessarySend + 'or,
    E: NecessarySend + 'or,
    OR: Observer<(T, RequestCallbackType<'or>), E> + NecessarySend + 'or,
    C: BackpressureCollection<T0, T> + NecessarySend + 'or,
{
    context.modify_model_with_action(|model| {
        if let Some(next) = model.collection.take_next_value() {
            let context = context.clone();
            let callback: RequestCallbackType = Box::new(move || {
                handle_request(context);
            });
            Action::SendNext((next, callback))
        } else if let Some(termination) = model.termination.take() {
            Action::SendTermination(termination)
        } else {
            model.emit_directly = true;
            Action::None
        }
    });
}
