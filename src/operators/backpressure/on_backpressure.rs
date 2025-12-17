use crate::disposable::Disposable;
use crate::utils::types::{MutGuard, Mutable, MutableHelper, NecessarySendSync, Shared};
use crate::{
    disposable::subscription::Subscription,
    observable::Observable,
    observer::{Observer, Termination},
};
use crate::{safe_lock_option, safe_lock_option_observer};
use educe::Educe;

cfg_if::cfg_if! {
    if #[cfg(feature = "single-threaded")] {
        /// Callback handed to the downstream observer so it can request the next chunk
        /// once it finishes processing the current batch.
        pub type RequestCallbackType<'cb> = Box<dyn FnOnce() + 'cb>;
    } else {
        /// Callback handed to the downstream observer so it can request the next chunk
        /// once it finishes processing the current batch.
        pub type RequestCallbackType<'cb> = Box<dyn FnOnce() + Send + 'cb>;
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
    T: NecessarySendSync + 'or + 'sub,
    E: NecessarySendSync + 'or + 'sub,
    OE: Observable<'or, 'sub, T, E>,
    F: FnMut(&mut Vec<T>, T) + NecessarySendSync + 'or,
{
    fn subscribe(
        self,
        observer: impl Observer<(Vec<T>, RequestCallbackType<'or>), E> + NecessarySendSync + 'or,
    ) -> Subscription<'sub> {
        let context = Shared::new(Mutable::new(OnBackpressureContext {
            buffer: Some(Vec::new()),
            termination: None,
            emit_directly: true,
        }));
        let observer = OnBackpressureObserver {
            observer: Shared::new(Mutable::new(Some(observer))),
            context: context.clone(),
            receiving_strategy: self.receiving_strategy,
        };
        self.source.subscribe(observer) + context
    }
}

struct OnBackpressureContext<T, E> {
    buffer: Option<Vec<T>>, // None means the subscription is disposed
    termination: Option<Termination<E>>,
    emit_directly: bool,
}

impl<T, E> Disposable for Shared<Mutable<OnBackpressureContext<T, E>>> {
    fn dispose(self) {
        safe_lock_option!(take: self, buffer);
    }
}

struct OnBackpressureObserver<T, E, OR, F> {
    observer: Shared<Mutable<Option<OR>>>,
    context: Shared<Mutable<OnBackpressureContext<T, E>>>,
    receiving_strategy: F,
}

impl<'cb, T, E, OR, F> Observer<T, E> for OnBackpressureObserver<T, E, OR, F>
where
    T: NecessarySendSync + 'cb,
    E: NecessarySendSync + 'cb,
    OR: Observer<(Vec<T>, RequestCallbackType<'cb>), E> + NecessarySendSync + 'cb,
    F: FnMut(&mut Vec<T>, T),
{
    fn on_next(&mut self, value: T) {
        self.context.lock_mut(|mut lock| {
            if let Some(buffer) = &mut lock.buffer {
                (self.receiving_strategy)(buffer, value);
                if lock.emit_directly {
                    emit(Some(lock), self.observer.clone(), self.context.clone());
                }
            }
        });
    }

    fn on_termination(self, termination: Termination<E>) {
        self.context.lock_mut(|mut lock| {
            if lock.buffer.is_none() {
                return;
            }
            lock.termination = Some(termination);
            if lock.emit_directly {
                emit(Some(lock), self.observer.clone(), self.context.clone());
            }
        });
    }
}

fn emit<'cb, T, E, OR>(
    lock: Option<MutGuard<'_, OnBackpressureContext<T, E>>>,
    observer: Shared<Mutable<Option<OR>>>,
    context: Shared<Mutable<OnBackpressureContext<T, E>>>,
) where
    T: NecessarySendSync + 'cb,
    E: NecessarySendSync + 'cb,
    OR: Observer<(Vec<T>, RequestCallbackType<'cb>), E> + NecessarySendSync + 'cb,
{
    let implementation = |mut lock: MutGuard<'_, OnBackpressureContext<T, E>>| {
        if let Some(buffer) = &mut lock.buffer {
            if buffer.is_empty() {
                if let Some(termination) = lock.termination.take() {
                    // OnTermination
                    drop(lock);
                    safe_lock_option_observer!(on_termination: observer, termination);
                } else {
                    // This code is calling from request callback. Emit directly for next value
                    lock.emit_directly = true;
                    drop(lock);
                }
            } else {
                // OnNext
                let values = std::mem::take(buffer);
                lock.emit_directly = false;
                drop(lock);
                let observer_cloned = observer.clone();
                let context_cloned = context.clone();
                let callback: RequestCallbackType = Box::new(move || {
                    emit(None, observer_cloned, context_cloned);
                });
                safe_lock_option_observer!(on_next: observer, (values, callback));
            }
        } else {
            // The subscription is disposed
        }
    };
    if let Some(lock) = lock {
        implementation(lock);
    } else {
        context.lock_mut(implementation);
    }
}
