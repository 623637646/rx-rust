//! The [`SkipUntil`] operator, behind
//! [`ObservableExt::skip_until`](crate::observable::ObservableExt::skip_until).

use crate::{
    delegate_disposal,
    disposable::{Disposable, chain_disposal::ChainDisposal, dispose_on_drop::DisposeOnDrop},
    observable::{ChainDisposalOf, JoinedModeOf, Observable, ObservableTypes},
    observer::{Flow, Observer, Termination},
    thread_mode::ThreadMode,
    utils::{
        serialized_delivery::UpdateOutcome,
        subscribe_with_context::{self, SubscriptionContext, subscribe_with_context_two_steps},
    },
};
use educe::Educe;

/// Discards items emitted by a source observable until a second observable emits an item. A second
/// observable that completes before emitting completes the result.
/// See <https://reactivex.io/documentation/operators/skipuntil.html>
///
/// The values of the second observable are dropped — only their arrival matters — so it can be
/// an observable of any item type.
///
/// The first item of the second observable opens the gate for good, so that observable is answered
/// [`Flow::Stop`] right then, as ReactiveX unsubscribes from it: whatever it sends afterwards, an
/// error included, no longer reaches the result.
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt,
///     observer::{Observer, Termination},
///     operators::conditional_boolean::skip_until::SkipUntil,
///     subject::publish_subject::PublishSubject,
/// };
/// use std::convert::Infallible;
///
/// let mut values = Vec::new();
/// let mut source = PublishSubject::local();
/// let mut gate = PublishSubject::<_, Infallible, _>::local();
///
/// let subscription = SkipUntil::new(source.clone(), gate.clone()).subscribe_with_callback(
///     |value| values.push(value),
///     |termination| assert_eq!(termination, Termination::Completed),
/// );
///
/// let _ = source.on_next(1); // Skipped: the gate has not emitted yet.
/// let _ = gate.on_next(());
/// let _ = source.on_next(2);
/// source.on_termination(Termination::Completed);
///
/// drop((subscription, gate));
/// assert_eq!(values, [2]);
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
    pub fn new<E>(source: OE, start: OE1) -> Self
    where
        OE: ObservableTypes<Error = E>,
        OE1: ObservableTypes<Error = E>,
    {
        Self { source, start }
    }
}

delegate_disposal!(
    Disposal<M, T, E, D, D1>,
    subscribe_with_context::Disposal<M, T, E, Model, ChainDisposal<D, D1>>,
    where M: ThreadMode, D: Disposable, D1: Disposable
);

impl<T, E, OE, OE1> ObservableTypes for SkipUntil<OE, OE1>
where
    OE: ObservableTypes<Item = T, Error = E>,
    OE1: ObservableTypes<Error = E>,
{
    type Item = T;
    type Error = E;
    type Mode = JoinedModeOf<OE, OE1>;
    type Disposal = Disposal<JoinedModeOf<OE, OE1>, T, E, OE::Disposal, OE1::Disposal>;
}

impl<T, E, OE, OE1, OR> Observable<OR> for SkipUntil<OE, OE1>
where
    OR: Observer<T, E>,
    OE: Observable<
            SkipUntilObserver<JoinedModeOf<OE, OE1>, T, E, OR, ChainDisposalOf<OE, OE1>>,
            Item = T,
            Error = E,
        >,
    OE1: Observable<
            StartObserver<JoinedModeOf<OE, OE1>, T, E, OR, ChainDisposalOf<OE, OE1>>,
            Error = E,
        >,
{
    fn subscribe(self, observer: OR) -> DisposeOnDrop<Self::Disposal> {
        let model = Model { started: false };
        subscribe_with_context_two_steps(
            observer,
            model,
            |context| Some(self.start.subscribe(StartObserver(context.clone()))),
            |context| Some(self.source.subscribe(SkipUntilObserver(context.clone()))),
        )
        .map_inner_into()
    }
}

struct Model {
    started: bool,
}

#[doc(hidden)]
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

#[doc(hidden)]
pub struct StartObserver<M: ThreadMode, T, E, OR, D: Disposable>(
    SubscriptionContext<M, T, E, OR, Model, D>,
);

impl<M: ThreadMode, T, T1, E, OR, D> Observer<T1, E> for StartObserver<M, T, E, OR, D>
where
    OR: Observer<T, E>,
    D: Disposable,
{
    fn on_next(&mut self, value: T1) -> Flow {
        // The gate stays open from here on, so nothing the notifier sends later matters: it is told
        // to stop, whether or not the context has stopped already.
        let _ = self.0.update(|model| {
            model.started = true;
            UpdateOutcome::empty()
        });
        // Only the arrival of a notification matters, never its value, which is dropped here,
        // after the update and outside its lock, since dropping it can run arbitrary code.
        drop(value);
        Flow::Stop
    }

    fn on_termination(self, termination: Termination<E>) {
        // Only a notifier that has not emitted gets here, since the first item stops it: completing
        // before the gate opens completes the result, which would skip everything, and an error
        // fails it.
        self.0.send_termination(termination);
    }
}
