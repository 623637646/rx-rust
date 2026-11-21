//! The [`StopOnDisposal`] operator, behind
//! [`ObservableExt::stop_on_disposal`](crate::observable::ObservableExt::stop_on_disposal).

use crate::{
    delegate_disposal,
    disposable::{
        Disposable, chain_disposal::ChainDisposal, dispose_on_drop::DisposeOnDrop,
        flag_disposal::FlagDisposal,
    },
    observable::{Observable, ObservableTypes},
    observer::{Flow, Observer, Termination},
    thread_mode::{ThreadMode, mutable::RaiseOnlyFlag},
};
use educe::Educe;

/// Delivers nothing more to the observer once the subscription is disposed, and answers
/// [`Flow::Stop`] from then on.
///
/// A disposal goes from the subscription straight to the source: the operators in between do not
/// see it. One that sends several notifications for a single event of its source — the buffered
/// values and the completion of
/// [`take_last`](crate::observable::ObservableExt::take_last), the last bundle of
/// [`buffer_with_count`](crate::observable::ObservableExt::buffer_with_count) — is stopped
/// between two of them only by a [`Flow::Stop`]. An observer that disposes its own subscription
/// from `on_next` says so by answering [`Flow::Stop`]; this operator is for what cannot: a
/// callback that returns `()`, or a disposal made by someone else, such as a closure further up
/// the chain ([`do_before_next`](crate::observable::ObservableExt::do_before_next), say). It
/// goes last in the chain, right before the subscription.
///
/// What the source sends from inside its own disposal is still delivered: the subscription counts
/// as disposed once the source's disposal has returned. A disposal made on another thread while a
/// notification is on its way cannot hold that notification back.
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::{Observable, ObservableExt},
///     observer::{Observer, Termination, callback_observer::CallbackObserver},
///     subject::publish_subject::PublishSubject,
///     thread_mode::mutable::{MutableExt, MutableHelper},
/// };
/// use std::{cell::RefCell, convert::Infallible, rc::Rc};
///
/// let subscription = Rc::new(RefCell::new(None));
/// let events = Rc::new(RefCell::new(Vec::new()));
/// let (subscription_cloned, on_next_events, on_termination_events) =
///     (subscription.clone(), events.clone(), events.clone());
///
/// let subject = PublishSubject::<_, Infallible, _>::local();
/// let mut sender = subject.clone();
/// // `take_last` replays its two values when the source completes, then completes itself.
/// let made = subject.take_last(2).stop_on_disposal().subscribe(CallbackObserver::new(
///     move |value| {
///         on_next_events.with_mut(|events| events.push(value));
///         // Disposed on the first of them, by a callback that does not answer `Flow::Stop`.
///         drop(subscription_cloned.take_value());
///     },
///     move |_| on_termination_events.with_mut(|events| events.push(0)),
/// ));
/// subscription.with_mut(|subscription| *subscription = Some(made));
///
/// for value in [1, 2, 3] {
///     let _ = sender.on_next(value);
/// }
/// sender.on_termination(Termination::Completed);
/// // Without the operator: `[2, 3, 0]`.
/// assert_eq!(events.clone_value(), [2]);
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct StopOnDisposal<OE> {
    source: OE,
}

impl<OE> StopOnDisposal<OE> {
    /// Creates a [`StopOnDisposal`] over `source`;
    /// [`ObservableExt::stop_on_disposal`](crate::observable::ObservableExt::stop_on_disposal)
    /// is the fluent form.
    pub fn new(source: OE) -> Self {
        Self { source }
    }
}

impl<OE> ObservableTypes for StopOnDisposal<OE>
where
    OE: ObservableTypes,
{
    type Item = OE::Item;
    type Error = OE::Error;
    type Mode = OE::Mode;
    /// First the source's subscription, then the flag the observer reads.
    type Disposal = Disposal<OE::Mode, OE::Disposal>;
}

impl<OE, OR> Observable<OR> for StopOnDisposal<OE>
where
    OR: Observer<<OE as ObservableTypes>::Item, <OE as ObservableTypes>::Error>,
    OE: Observable<StopOnDisposalObserver<<OE as ObservableTypes>::Mode, OR>>,
{
    fn subscribe(self, observer: OR) -> DisposeOnDrop<Self::Disposal> {
        let disposed = <OE::Mode as ThreadMode>::Flag::default();
        let observer = StopOnDisposalObserver {
            observer,
            disposed: disposed.clone(),
        };
        self.source
            .subscribe(observer)
            .then(FlagDisposal(disposed))
            .map_inner_into()
    }
}

delegate_disposal!(
    Disposal<M, D>,
    ChainDisposal<D, FlagDisposal<<M as ThreadMode>::Flag>>,
    where M: ThreadMode, D: Disposable
);

#[doc(hidden)]
pub struct StopOnDisposalObserver<M: ThreadMode, OR> {
    observer: OR,
    disposed: M::Flag,
}

impl<M, T, E, OR> Observer<T, E> for StopOnDisposalObserver<M, OR>
where
    M: ThreadMode,
    OR: Observer<T, E>,
{
    fn on_next(&mut self, value: T) -> Flow {
        if self.disposed.is_raised() {
            return Flow::Stop;
        }
        let flow = self.observer.on_next(value);
        // The observer may have disposed the subscription while it handled the value.
        if self.disposed.is_raised() {
            Flow::Stop
        } else {
            flow
        }
    }

    fn on_termination(self, termination: Termination<E>) {
        if !self.disposed.is_raised() {
            self.observer.on_termination(termination);
        }
    }
}
