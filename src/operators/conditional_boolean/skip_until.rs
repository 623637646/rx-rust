//! The [`SkipUntil`] operator, behind
//! [`ObservableExt::skip_until`](crate::observable::ObservableExt::skip_until).

use crate::disposable::Disposable;
use crate::disposable::chain_disposal::ChainDisposal;
use crate::thread_mode::Joined;
use crate::thread_mode::ThreadMode;
use crate::utils::serialized_delivery::UpdateOutcome;
use crate::utils::subscribe_with_context::{
    self, SubscriptionContext, subscribe_with_context_owning_source,
};
use crate::{
    observable::Subscription,
    observable::{Observable, ObservableTypes},
    observer::{Flow, Observer, Termination},
};
use educe::Educe;

/// Discards items emitted by a source Observable until a second Observable emits an item. A second
/// Observable that completes before emitting completes the result.
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
/// let mut source: PublishSubject<'_, i32, Infallible, rx_rust::thread_mode::Local> = PublishSubject::local();
/// let mut gate: PublishSubject<'_, (), Infallible, rx_rust::thread_mode::Local> = PublishSubject::local();
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
    /// Creates a [`SkipUntil`] over `source`;
    /// [`ObservableExt::skip_until`](crate::observable::ObservableExt::skip_until) is the fluent
    /// form.
    pub fn new<T, E>(source: OE, start: OE1) -> Self
    where
        OE: ObservableTypes<Item = T, Error = E>,
        OE1: ObservableTypes<Item = (), Error = E>,
    {
        Self { source, start }
    }
}

impl<T, E, OE, OE1> ObservableTypes for SkipUntil<OE, OE1>
where
    OE: ObservableTypes<Item = T, Error = E>,
    OE1: ObservableTypes<Item = (), Error = E>,
{
    type Item = T;
    type Error = E;
    type Mode = Joined<OE::Mode, OE1::Mode>;
    type D = subscribe_with_context::ContextDisposal<
        Joined<OE::Mode, OE1::Mode>,
        T,
        E,
        Model,
        ChainDisposal<OE::D, OE1::D>,
    >;
}

impl<T, E, OE, OE1, OR> Observable<OR> for SkipUntil<OE, OE1>
where
    OR: Observer<T, E>,
    OE: Observable<
            SkipUntilObserver<
                Joined<<OE as ObservableTypes>::Mode, <OE1 as ObservableTypes>::Mode>,
                T,
                E,
                OR,
                ChainDisposal<<OE as ObservableTypes>::D, <OE1 as ObservableTypes>::D>,
            >,
            Item = T,
            Error = E,
        >,
    OE1: Observable<
            StartObserver<
                Joined<<OE as ObservableTypes>::Mode, <OE1 as ObservableTypes>::Mode>,
                T,
                E,
                OR,
                ChainDisposal<<OE as ObservableTypes>::D, <OE1 as ObservableTypes>::D>,
            >,
            Item = (),
            Error = E,
        >,
{
    fn subscribe(self, observer: OR) -> Subscription<Self::D> {
        let model = Model { started: false };
        subscribe_with_context_owning_source(observer, model, |context| {
            let subscription_1 = self.start.subscribe(StartObserver {
                context: context.clone(),
                started: false,
            });
            let subscription_2 = self.source.subscribe(SkipUntilObserver(context));
            subscription_1.preceded_by_bound(subscription_2)
        })
    }
}

pub struct Model {
    started: bool,
}

pub struct SkipUntilObserver<M: ThreadMode, T, E, OR, D: Disposable>(
    SubscriptionContext<M, T, E, OR, Model, D>,
);

impl<M: ThreadMode, T, E, OR, D> Observer<T, E> for SkipUntilObserver<M, T, E, OR, D>
where
    OR: Observer<T, E>,
    D: Disposable,
{
    fn on_next(&mut self, value: T) -> Flow {
        self.0.update_flow(|model| {
            if model.started {
                UpdateOutcome::empty()
                    .without_drop_outside()
                    .with_next_event(value)
            } else {
                // A skipped value is dropped outside the lock: dropping it can run arbitrary code.
                UpdateOutcome::empty()
                    .with_drop_outside(value)
                    .without_events()
            }
        })
    }

    fn on_termination(self, termination: Termination<E>) {
        self.0.send_termination(termination);
    }
}

pub struct StartObserver<M: ThreadMode, T, E, OR, D: Disposable> {
    context: SubscriptionContext<M, T, E, OR, Model, D>,
    started: bool,
}

impl<M: ThreadMode, T, E, OR, D> Observer<(), E> for StartObserver<M, T, E, OR, D>
where
    OR: Observer<T, E>,
    D: Disposable,
{
    fn on_next(&mut self, _: ()) -> Flow {
        if !self.started {
            self.started = true;
            self.context.update_flow(|model| {
                model.started = true;
                UpdateOutcome::empty()
            })
        } else {
            Flow::Continue
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
