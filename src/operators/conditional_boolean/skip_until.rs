use crate::disposable::Disposable;
use crate::utils::subscribe_with_context::{
    BoundSubscriptionDisposal, ModelUpdate, SubscriptionContext,
    subscribe_with_context_bound_subscription,
};
use crate::utils::types::MaybeSend;
use crate::{
    observable::Observable,
    observable::Subscription,
    observer::{Observer, Termination},
};
use educe::Educe;

/// Discards items emitted by a source Observable until a second Observable emits an item.
/// See <https://reactivex.io/documentation/operators/skipuntil.html>
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt,
///     observer::{Observer, Termination},
///     operators::conditional_boolean::skip_until::SkipUntil,
///     subject::publish_subject::PublishSubject,
/// };
/// use std::{convert::Infallible, sync::{Arc, Mutex}};
///
/// let values = Arc::new(Mutex::new(Vec::new()));
/// let terminations = Arc::new(Mutex::new(Vec::new()));
///
/// let mut source: PublishSubject<'_, i32, Infallible> = PublishSubject::default();
/// let mut gate: PublishSubject<'_, (), Infallible> = PublishSubject::default();
/// let values_observer = Arc::clone(&values);
/// let terminations_observer = Arc::clone(&terminations);
///
/// let subscription = SkipUntil::new(source.clone(), gate.clone()).subscribe_with_callback(
///     move |value| values_observer.lock().unwrap().push(value),
///     move |termination| terminations_observer
///         .lock()
///         .unwrap()
///         .push(termination),
/// );
///
/// source.on_next(1);
/// gate.on_next(());
/// source.on_next(2);
/// source.on_termination(Termination::Completed);
/// drop(subscription);
///
/// assert_eq!(&*values.lock().unwrap(), &[2]);
/// assert_eq!(
///     &*terminations.lock().unwrap(),
///     &[Termination::Completed]
/// );
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct SkipUntil<OE, OE1> {
    source: OE,
    start: OE1,
}

impl<OE, OE1> SkipUntil<OE, OE1> {
    pub fn new<'or, T, E>(source: OE, start: OE1) -> Self
    where
        OE: Observable<'or, T, E>,
        OE1: Observable<'or, (), E>,
    {
        Self { source, start }
    }
}

impl<'or, T, E, OE, OE1> Observable<'or, T, E> for SkipUntil<OE, OE1>
where
    T: MaybeSend + 'or,
    E: MaybeSend + 'or,
    OE: Observable<'or, T, E>,
    OE::D: MaybeSend + 'or,
    OE1: Observable<'or, (), E>,
    OE1::D: MaybeSend + 'or,
{
    type D = BoundSubscriptionDisposal<'or>;

    fn subscribe(self, observer: impl Observer<T, E> + MaybeSend + 'or) -> Subscription<Self::D> {
        let model = Model { started: false };
        subscribe_with_context_bound_subscription(observer, model, |context| {
            let subscription_1 = self.start.subscribe(StartObserver {
                context: context.clone(),
                started: false,
            });
            let subscription_2 = self.source.subscribe(SkipUntilObserver(context));
            subscription_1.preceded_by_bound(subscription_2)
        })
    }
}

struct Model {
    started: bool,
}

struct SkipUntilObserver<T, E, OR, D: Disposable>(SubscriptionContext<T, E, OR, Model, D>);

impl<T, E, OR, D> Observer<T, E> for SkipUntilObserver<T, E, OR, D>
where
    OR: Observer<T, E>,
    D: Disposable,
{
    fn on_next(&mut self, value: T) {
        let _ = self.0.try_update_model(|model| {
            if model.started {
                ModelUpdate::empty().with_next_event(value)
            } else {
                ModelUpdate::empty().without_events()
            }
        });
    }

    fn on_termination(self, termination: Termination<E>) {
        self.0.send_termination(termination);
    }
}

struct StartObserver<T, E, OR, D: Disposable> {
    context: SubscriptionContext<T, E, OR, Model, D>,
    started: bool,
}

impl<T, E, OR, D> Observer<(), E> for StartObserver<T, E, OR, D>
where
    OR: Observer<T, E>,
    D: Disposable,
{
    fn on_next(&mut self, _: ()) {
        if !self.started {
            self.started = true;
            let _ = self.context.try_update_model(|model| {
                model.started = true;
                ModelUpdate::empty()
            });
        }
    }

    fn on_termination(self, termination: Termination<E>) {
        match termination {
            completion @ Termination::Completed => {
                if !self.started {
                    self.context.send_termination(completion);
                }
            }
            error @ Termination::Error(_) => {
                self.context.send_termination(error);
            }
        }
    }
}
