use crate::observable::shared_model_observable::{Context, SharedModel, SharedModelObservable};
use crate::utils::types::{ActionAfterLock, MarkerType, MutableHelper, NecessarySend};
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
        pub type RequestCallbackType<'cb> = Box<dyn FnOnce() + 'cb>;
    } else {
        /// Callback handed to the downstream observer so it can request the next chunk
        /// once it finishes processing the current batch.
        pub type RequestCallbackType<'cb> = Box<dyn FnOnce() + Send + Sync + 'cb>;
    }
}

pub trait BackpressureCollection<T0, T> {
    fn extend_one(&mut self, item: T0);
    fn take_next_value(&mut self) -> Option<T>;
}

/// Low-level primitive that converts a fast upstream into demand-driven chunks by
/// accumulating values with a custom `receiving_strategy` and emitting them alongside a
/// [`RequestCallbackType`]. Downstream observers must invoke the callback to resume the
/// upstream flow. See <https://reactivex.io/documentation/operators/backpressure.html>.
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     disposable::Disposable,
///     observable::observable_ext::ObservableExt,
///     observer::Observer,
///     operators::backpressure::on_backpressure::OnBackpressure,
///     subject::publish_subject::PublishSubject,
/// };
/// use std::convert::Infallible;
///
/// let mut received = Vec::new();
/// let mut subject = PublishSubject::<_, Infallible>::new();
/// let observable = OnBackpressure::new(subject.clone(), |collection, value| collection.push(value));
///
/// let subscription = observable.subscribe_with_callback(
///     |(values, request_callback)| {
///         received.push(values);
///         request_callback(); // ready for the next batch
///     },
///     |_| {},
/// );
///
/// subject.on_next(1);
/// subject.on_next(2);
/// subject.on_next(3);
///
/// subscription.dispose();
/// drop(subject);
///
/// assert_eq!(received, vec![vec![1], vec![2], vec![3]]);
/// ```
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
    T0: 'sub,
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
        self.source.subscribe_with_shared_model(observer, model)
    }
}

struct Model<E, C> {
    collection: C,
    termination: Option<Termination<E>>,
    emit_directly: bool,
}

impl<'cb, T0, T, E, OR, C> SharedModel<T0, (T, RequestCallbackType<'cb>), E, OR> for Model<E, C>
where
    T: NecessarySend + 'cb,
    E: NecessarySend + 'cb,
    OR: Observer<(T, RequestCallbackType<'cb>), E> + NecessarySend + 'cb,
    C: BackpressureCollection<T0, T> + NecessarySend + 'cb,
{
    fn on_next(context: Context<(T, RequestCallbackType<'cb>), E, OR, Self>, value: T0) {
        let next = context
            .model
            .safe_lock_mut_with_args(value, |model, value| {
                if model.termination.is_some() {
                    return None;
                }
                model.collection.extend_one(value);
                if model.emit_directly {
                    model.emit_directly = false;
                    let next = model.collection.take_next_value().expect("cannot be empty");
                    Some(next)
                } else {
                    None
                }
            });
        if let Some(next) = next {
            let context_cloned = context.clone();
            let callback: RequestCallbackType = Box::new(move || {
                handle_request(context_cloned);
            });
            context.send_next((next, callback));
        }
    }

    fn on_termination(
        context: Context<(T, RequestCallbackType<'cb>), E, OR, Self>,
        termination: Termination<E>,
    ) {
        let termination =
            context
                .model
                .safe_lock_mut_with_args(termination, |model, termination| {
                    if model.emit_directly {
                        Some(termination)
                    } else {
                        model.termination = Some(termination);
                        None
                    }
                });
        if let Some(termination) = termination {
            context.send_termination(termination);
        }
    }

    fn on_dispose(context: Context<(T, RequestCallbackType<'cb>), E, OR, Self>) {
        // Clean up the collection
        let _ = context
            .model
            .safe_lock_mut(|model| model.collection.take_next_value());
    }
}

fn handle_request<'cb, T0, T, E, OR, C>(
    context: Context<(T, RequestCallbackType<'cb>), E, OR, Model<E, C>>,
) where
    T: NecessarySend + 'cb,
    E: NecessarySend + 'cb,
    OR: Observer<(T, RequestCallbackType<'cb>), E> + NecessarySend + 'cb,
    C: BackpressureCollection<T0, T> + NecessarySend + 'cb,
{
    let action = context.model.safe_lock_mut(|model| {
        if let Some(next) = model.collection.take_next_value() {
            ActionAfterLock::Next(next)
        } else if let Some(termination) = model.termination.take() {
            ActionAfterLock::Termination(termination)
        } else {
            model.emit_directly = true;
            ActionAfterLock::None
        }
    });
    match action {
        ActionAfterLock::Next(next) => {
            let context_cloned = context.clone();
            let callback: RequestCallbackType = Box::new(move || {
                handle_request(context_cloned);
            });
            context.send_next((next, callback));
        }
        ActionAfterLock::Termination(termination) => {
            context.send_termination(termination);
        }
        ActionAfterLock::None => {}
    }
}
