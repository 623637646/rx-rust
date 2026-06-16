use crate::observable::shared_model_observable::{Context, SharedModel, SharedModelObservable};
use crate::utils::types::{ActionAfterLock, MutableHelper, NecessarySend};
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
/// let observable = OnBackpressure::new(subject.clone(), |buffer, value| buffer.push(value));
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
pub struct OnBackpressure<OE, F> {
    source: OE,
    receiving_strategy: F,
}

impl<OE, F> OnBackpressure<OE, F> {
    pub fn new<'or, 'sub, T, E>(source: OE, receiving_strategy: F) -> Self
    where
        OE: Observable<'or, 'sub, T, E>,
        F: FnMut(&mut Vec<T>, T),
    {
        Self {
            source,
            receiving_strategy,
        }
    }
}

impl<'or, 'sub, T, E, OE, F> Observable<'or, 'sub, (Vec<T>, RequestCallbackType<'or>), E>
    for OnBackpressure<OE, F>
where
    'or: 'sub,
    T: NecessarySend + 'or,
    E: NecessarySend + 'or,
    OE: Observable<'or, 'sub, T, E>,
    F: FnMut(&mut Vec<T>, T) + NecessarySend + 'or,
{
    fn subscribe(
        self,
        observer: impl Observer<(Vec<T>, RequestCallbackType<'or>), E> + NecessarySend + 'or,
    ) -> Subscription<'sub> {
        let model = Model {
            buffer: Vec::new(),
            termination: None,
            emit_directly: true,
        };
        self.source
            .subscribe_with_shared_model(observer, model, self.receiving_strategy)
    }
}

struct Model<T, E> {
    buffer: Vec<T>,
    termination: Option<Termination<E>>,
    emit_directly: bool,
}

impl<'cb, T, E, OR, F> SharedModel<T, (Vec<T>, RequestCallbackType<'cb>), E, OR, F> for Model<T, E>
where
    T: NecessarySend + 'cb,
    E: NecessarySend + 'cb,
    OR: Observer<(Vec<T>, RequestCallbackType<'cb>), E> + NecessarySend + 'cb,
    F: FnMut(&mut Vec<T>, T),
{
    fn on_next(
        value: T,
        context: Context<(Vec<T>, RequestCallbackType<'cb>), E, OR, Self>,
        receiving_strategy: &mut F,
    ) {
        let buffer = context.model.safe_lock_mut_with_args(
            (value, receiving_strategy),
            |model, (value, receiving_strategy)| {
                if model.termination.is_some() {
                    return None;
                }
                receiving_strategy(&mut model.buffer, value);
                if model.emit_directly {
                    model.emit_directly = false;
                    let buffer = std::mem::take(&mut model.buffer);
                    Some(buffer)
                } else {
                    None
                }
            },
        );
        if let Some(buffer) = buffer {
            let context_cloned = context.clone();
            let callback: RequestCallbackType = Box::new(move || {
                handle_request(context_cloned);
            });
            context.send_next((buffer, callback));
        }
    }

    fn on_termination(
        termination: Termination<E>,
        context: Context<(Vec<T>, RequestCallbackType<'cb>), E, OR, Self>,
        _extra: F,
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
}

fn handle_request<'cb, T, E, OR>(
    context: Context<(Vec<T>, RequestCallbackType<'cb>), E, OR, Model<T, E>>,
) where
    T: NecessarySend + 'cb,
    E: NecessarySend + 'cb,
    OR: Observer<(Vec<T>, RequestCallbackType<'cb>), E> + NecessarySend + 'cb,
{
    let action = context.model.safe_lock_mut(|model| {
        if !model.buffer.is_empty() {
            let buffer = std::mem::take(&mut model.buffer);
            ActionAfterLock::Next(buffer)
        } else if let Some(termination) = model.termination.take() {
            ActionAfterLock::Termination(termination)
        } else {
            model.emit_directly = true;
            ActionAfterLock::None
        }
    });
    match action {
        ActionAfterLock::Next(buffer) => {
            let context_cloned = context.clone();
            let callback: RequestCallbackType = Box::new(move || {
                handle_request(context_cloned);
            });
            context.send_next((buffer, callback));
        }
        ActionAfterLock::Termination(termination) => {
            context.send_termination(termination);
        }
        ActionAfterLock::None => {}
    }
}
