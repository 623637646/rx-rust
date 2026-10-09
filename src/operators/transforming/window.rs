//! The [`Window`] operator, behind
//! [`ObservableExt::window`](crate::observable::ObservableExt::window).

use crate::delegate_disposal;
use crate::disposable::chain_disposal::ChainDisposal;
use crate::observer::boxed_observer::ObserverMode;
use crate::thread_mode::{Joined, ThreadMode};
use crate::utils::MarkerType;
use crate::{
    disposable::{Disposable, dispose_on_drop::DisposeOnDrop},
    observable::{Observable, ObservableTypes},
    observer::{EventBatch, Flow, Observer, Termination},
    subject::unicast_subject::{self, BoxedUnicastObservable, BoxedUnicastSender},
    utils::subscribe_with_context::{self, SubscriptionContext, subscribe_with_context},
};
use educe::Educe;
use std::marker::PhantomData;

/// Subdivides the items of an Observable into Observable windows, opening a new one whenever the
/// `boundary` Observable emits an item.
/// See <https://reactivex.io/documentation/operators/window.html>
///
/// Completing the `boundary` stops future window rotation without terminating the current window
/// or the outer Observable. An error from the `boundary` terminates the current window and the
/// outer Observable.
///
/// Each window is a single-consumer pipe: it can be subscribed to once, it buffers the items that
/// arrive while it has no subscriber, and dropping it without subscribing discards its items. A
/// window is serialized on its own rather than together with the outer Observable, so the events
/// of a window keep their order among themselves, but they are not ordered against the emission of
/// a later window. Disposing the outer subscription ends the open window without a termination:
/// what it had buffered is still delivered to its subscriber, even a later one, which is then
/// dropped without being notified.
///
/// Disposing the subscription of a single window does not necessarily release its observer where
/// it happens: the window releases it on its next item, when it ends, or when the outer
/// subscription is disposed, whichever comes first. See
/// [the unicast subject](crate::subject::unicast_subject) each window is built on.
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt,
///     observer::{Observer, Termination},
///     operators::transforming::window::Window,
///     subject::publish_subject::PublishSubject,
/// };
/// use std::{convert::Infallible, sync::{Arc, Mutex}};
///
/// let windows = Arc::new(Mutex::new(Vec::<Vec<i32>>::new()));
/// let terminations = Arc::new(Mutex::new(Vec::new()));
/// let inner_subscriptions = Arc::new(Mutex::new(Vec::new()));
///
/// let mut source: PublishSubject<'_, i32, Infallible, rx_rust::thread_mode::Local> = PublishSubject::local();
/// let mut boundary: PublishSubject<'_, (), Infallible, rx_rust::thread_mode::Local> = PublishSubject::local();
/// let windows_observer = Arc::clone(&windows);
/// let terminations_observer = Arc::clone(&terminations);
/// let inner_subscriptions_observer = Arc::clone(&inner_subscriptions);
///
/// let subscription = Window::new(source.clone(), boundary.clone()).subscribe_with_callback(
///     move |window| {
///         let index = {
///             let mut windows = windows_observer.lock().unwrap();
///             windows.push(Vec::new());
///             windows.len() - 1
///         };
///         let windows_for_values = Arc::clone(&windows_observer);
///         let sub = window.subscribe_with_callback(
///             move |value| {
///                 windows_for_values.lock().unwrap()[index].push(value);
///             },
///             |_| {},
///         );
///         inner_subscriptions_observer.lock().unwrap().push(sub);
///     },
///     move |termination| terminations_observer
///         .lock()
///         .unwrap()
///         .push(termination),
/// );
///
/// source.on_next(1);
/// source.on_next(2);
/// boundary.on_next(());
/// source.on_next(3);
/// source.on_termination(Termination::Completed);
/// drop(subscription);
/// inner_subscriptions.lock().unwrap().drain(..).for_each(drop);
///
/// assert_eq!(
///     &*windows.lock().unwrap(),
///     &[vec![1, 2], vec![3]]
/// );
/// assert_eq!(
///     &*terminations.lock().unwrap(),
///     &[Termination::Completed]
/// );
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct Window<'a, OE, OE1> {
    source: OE,
    boundary: OE1,
    /// The observer of a window may borrow for `'a`. Unlike the `_boxed` hooks this cannot be left
    /// to an unboxed default: the window is the `Item`, which [`ObservableTypes`] names without any
    /// observer, and its observer arrives only later, so the window boxes it.
    _marker: MarkerType<&'a ()>,
}

impl<OE, OE1> Window<'_, OE, OE1> {
    /// Creates a [`Window`] over `source`;
    /// [`ObservableExt::window`](crate::observable::ObservableExt::window) is the fluent form.
    pub fn new<T, E>(source: OE, boundary: OE1) -> Self
    where
        OE: ObservableTypes<Item = T, Error = E>,
        OE1: ObservableTypes<Item = (), Error = E>,
    {
        Self {
            source,
            boundary,
            _marker: PhantomData,
        }
    }
}

delegate_disposal!(
    Disposal<M, T, E, D, D1>,
    subscribe_with_context::Disposal<M, DelegateAction<T, E>, E, (), ChainDisposal<D, D1>>,
    where M: ThreadMode, D: Disposable, D1: Disposable
);

impl<'a, T, E, OE, OE1> ObservableTypes for Window<'a, OE, OE1>
where
    Joined<<OE as ObservableTypes>::Mode, <OE1 as ObservableTypes>::Mode>: ObserverMode,
    E: Clone,
    OE: ObservableTypes<Item = T, Error = E>,
    OE1: ObservableTypes<Item = (), Error = E>,
{
    type Item = BoxedUnicastObservable<'a, T, E, Joined<OE::Mode, OE1::Mode>>;
    type Error = E;
    type Mode = Joined<OE::Mode, OE1::Mode>;
    type Disposal = Disposal<Joined<OE::Mode, OE1::Mode>, T, E, OE::Disposal, OE1::Disposal>;
}

impl<'a, T, E, OE, OE1, OR> Observable<OR> for Window<'a, OE, OE1>
where
    Joined<<OE as ObservableTypes>::Mode, <OE1 as ObservableTypes>::Mode>: ObserverMode,
    OR: Observer<
            BoxedUnicastObservable<
                'a,
                T,
                E,
                Joined<<OE as ObservableTypes>::Mode, <OE1 as ObservableTypes>::Mode>,
            >,
            E,
        >,
    E: Clone,
    OE: Observable<
            SourceObserver<
                'a,
                Joined<<OE as ObservableTypes>::Mode, <OE1 as ObservableTypes>::Mode>,
                T,
                E,
                OR,
                ChainDisposal<
                    <OE as ObservableTypes>::Disposal,
                    <OE1 as ObservableTypes>::Disposal,
                >,
            >,
            Item = T,
            Error = E,
        >,
    OE1: Observable<
            BoundaryObserver<
                'a,
                Joined<<OE as ObservableTypes>::Mode, <OE1 as ObservableTypes>::Mode>,
                T,
                E,
                OR,
                ChainDisposal<
                    <OE as ObservableTypes>::Disposal,
                    <OE1 as ObservableTypes>::Disposal,
                >,
            >,
            Item = (),
            Error = E,
        >,
{
    fn subscribe(self, observer: OR) -> DisposeOnDrop<Self::Disposal> {
        let observer = DelegateObserver {
            outer_observer: observer,
            sender: None,
        };
        // The windows own their buffered items, so the context needs no model of its own: it only
        // serializes the actions below and owns the source and boundary subscriptions.
        subscribe_with_context(observer, (), |context| {
            // The first window is opened before subscribing, so that a synchronous source has a
            // window to deliver its values to. An observer that stops on that first window stops
            // the context, which disposes the subscriptions below as soon as they are installed.
            let _ = context.send_next(DelegateAction::EmitWindow);
            let boundary_subscription = self.boundary.subscribe(BoundaryObserver(context.clone()));
            let source_subscription = self.source.subscribe(SourceObserver(context));
            boundary_subscription.preceded_by_wrapped(source_subscription)
        })
        .map_inner_into()
    }
}

type WindowContext<'a, M, T, E, OR, D> =
    SubscriptionContext<M, DelegateAction<T, E>, E, DelegateObserver<'a, M, T, E, OR>, (), D>;

pub struct SourceObserver<'a, M: ObserverMode, T, E, OR, D: Disposable>(
    WindowContext<'a, M, T, E, OR, D>,
);

impl<'a, M, T, E, OR, D> Observer<T, E> for SourceObserver<'a, M, T, E, OR, D>
where
    M: ObserverMode,
    E: Clone,
    OR: Observer<BoxedUnicastObservable<'a, T, E, M>, E>,
    D: Disposable,
{
    fn on_next(&mut self, value: T) -> Flow {
        // The value is queued as an action, so that it reaches the window outside the lock.
        self.0.send_next(DelegateAction::ForwardValue(value))
    }

    fn on_termination(self, termination: Termination<E>) {
        terminate(self.0, termination);
    }
}

/// Terminates the current window, then the outer Observable.
fn terminate<'a, M, T, E, OR, D>(
    context: WindowContext<'a, M, T, E, OR, D>,
    termination: Termination<E>,
) where
    M: ObserverMode,
    E: Clone,
    OR: Observer<BoxedUnicastObservable<'a, T, E, M>, E>,
    D: Disposable,
{
    let _ = context.send(EventBatch::NextAndTermination(
        DelegateAction::TerminateWindow(termination.clone()),
        termination,
    ));
}

pub struct BoundaryObserver<'a, M: ObserverMode, T, E, OR, D: Disposable>(
    WindowContext<'a, M, T, E, OR, D>,
);

impl<'a, M, T, E, OR, D> Observer<(), E> for BoundaryObserver<'a, M, T, E, OR, D>
where
    M: ObserverMode,
    E: Clone,
    OR: Observer<BoxedUnicastObservable<'a, T, E, M>, E>,
    D: Disposable,
{
    fn on_next(&mut self, _: ()) -> Flow {
        // Two events rather than one, so that disposing the outer subscription while the current
        // window is completing suppresses the new window.
        self.0.send(EventBatch::NextBatch(vec![
            DelegateAction::TerminateWindow(Termination::Completed),
            DelegateAction::EmitWindow,
        ]))
    }

    fn on_termination(self, termination: Termination<E>) {
        // Completing the boundary only stops the rotation: the current window and the outer
        // Observable keep going.
        if let error @ Termination::Error(_) = termination {
            terminate(self.0, error);
        }
    }
}

enum DelegateAction<T, E> {
    /// Sends a source value to the current window, if there is one.
    ForwardValue(T),
    /// Terminates the current window, if there is one.
    TerminateWindow(Termination<E>),
    /// Opens a new window, makes it the current one and emits it downstream.
    EmitWindow,
}

/// Owns the state that the actions act on, so that a window is fed and emitted outside the lock of
/// the context that serializes the source against the boundary.
pub struct DelegateObserver<'a, M: ObserverMode, T, E, OR> {
    outer_observer: OR,
    /// The sending end of the window that is currently open, which is the only place where the
    /// windows are fed from. It is `None` until the first window opens and after the last window
    /// ended.
    sender: Option<BoxedUnicastSender<'a, T, E, M>>,
}

impl<'a, M, T, E, OR> Observer<DelegateAction<T, E>, E> for DelegateObserver<'a, M, T, E, OR>
where
    M: ObserverMode,
    OR: Observer<BoxedUnicastObservable<'a, T, E, M>, E>,
{
    fn on_next(&mut self, action: DelegateAction<T, E>) -> Flow {
        match action {
            DelegateAction::ForwardValue(value) => {
                match &mut self.sender {
                    // The consumer of one window stops that window, not the operator: the next
                    // window has its own consumer.
                    Some(sender) => {
                        let _ = sender.on_next(value);
                    }
                    // No window accepts values, so this one has nowhere to go.
                    None => drop(value),
                }
                Flow::Continue
            }
            DelegateAction::TerminateWindow(termination) => {
                if let Some(sender) = self.sender.take() {
                    sender.on_termination(termination);
                }
                Flow::Continue
            }
            DelegateAction::EmitWindow => {
                debug_assert!(self.sender.is_none());
                let (sender, window) = unicast_subject::new_boxed();
                self.sender = Some(sender);
                self.outer_observer.on_next(window)
            }
        }
    }

    fn on_termination(self, termination: Termination<E>) {
        // The current window is terminated by the `TerminateWindow` action that precedes this one.
        debug_assert!(self.sender.is_none());
        self.outer_observer.on_termination(termination);
    }
}
