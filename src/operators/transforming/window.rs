use crate::{
    disposable::{Disposable, chain_disposal::ChainDisposal},
    observable::{Observable, Subscription},
    observer::{BoxedObserverExt, Observer, Termination, boxed_observer::BoxedObserver},
    subject::unicast_subject::{UnicastObservable, UnicastSender, unicast_subject},
    utils::{
        subscribe_with_context::{
            self, EventBatch, SubscriptionContext, subscribe_with_context_bound_subscription,
        },
        types::MaybeSend,
    },
};
use educe::Educe;

/// Periodically subdivides items from an Observable into Observable windows.
///
/// A new window is emitted whenever the `boundary` Observable emits an item.
/// Completing the `boundary` stops future window rotation without terminating the current window
/// or the outer Observable.
/// An error from the `boundary` terminates the current window and the outer Observable.
///
/// Each window is a single-consumer pipe: it can be subscribed to once, it buffers the items that
/// arrive while it has no subscriber, and dropping it without subscribing discards its items. A
/// window is serialized on its own rather than together with the outer Observable, so the events
/// of a window keep their order among themselves, but they are not ordered against the emission of
/// a later window. Disposing the outer subscription drops the observer of the open window without
/// notifying it.
/// See <https://reactivex.io/documentation/operators/window.html>
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
/// let mut source: PublishSubject<'_, i32, Infallible> = PublishSubject::default();
/// let mut boundary: PublishSubject<'_, (), Infallible> = PublishSubject::default();
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
pub struct Window<OE, OE1> {
    source: OE,
    boundary: OE1,
}

impl<OE, OE1> Window<OE, OE1> {
    pub fn new<'or, T, E>(source: OE, boundary: OE1) -> Self
    where
        OE: Observable<'or, T, E>,
        OE1: Observable<'or, (), E>,
    {
        Self { source, boundary }
    }
}

impl<'or, T, E, OE, OE1> Observable<'or, UnicastObservable<'or, T, E>, E> for Window<OE, OE1>
where
    T: MaybeSend + 'or,
    E: Clone + MaybeSend + 'or,
    OE: Observable<'or, T, E>,
    OE::D: MaybeSend + 'or,
    OE1: Observable<'or, (), E>,
    OE1::D: MaybeSend + 'or,
{
    type D = subscribe_with_context::BoundSubscriptionDisposal<'or>;

    fn subscribe(
        self,
        observer: impl Observer<UnicastObservable<'or, T, E>, E> + MaybeSend + 'or,
    ) -> Subscription<Self::D> {
        let observer = DelegateObserver {
            outer_observer: observer.into_boxed(),
            sender: None,
        };
        // The windows own their buffered items, so the context needs no model of its own: it only
        // serializes the actions below and owns the source and boundary subscriptions.
        subscribe_with_context_bound_subscription(observer, (), |context| {
            let mut boundary_observer = BoundaryObserver(context.clone());
            boundary_observer.on_next(());
            let boundary_subscription = self.boundary.subscribe(boundary_observer);
            let source_subscription = self.source.subscribe(SourceObserver(context));
            boundary_subscription.preceded_by_bound(source_subscription)
        })
    }
}

type WindowContext<'or, T, E, D1, D2> = SubscriptionContext<
    DelegateAction<'or, T, E>,
    E,
    DelegateObserver<'or, T, E>,
    (),
    ChainDisposal<D1, D2>,
>;

struct SourceObserver<'or, T, E, D1, D2>(WindowContext<'or, T, E, D1, D2>)
where
    D1: Disposable,
    D2: Disposable;

impl<T, E, D1, D2> Observer<T, E> for SourceObserver<'_, T, E, D1, D2>
where
    E: Clone,
    D1: Disposable,
    D2: Disposable,
{
    fn on_next(&mut self, value: T) {
        // The value is queued as an action, so that it reaches the window outside the lock.
        self.0.send_next(DelegateAction::ForwardValue(value));
    }

    fn on_termination(self, termination: Termination<E>) {
        terminate(self.0, termination);
    }
}

/// Terminates the current window, then the outer Observable.
fn terminate<T, E, D1, D2>(context: WindowContext<'_, T, E, D1, D2>, termination: Termination<E>)
where
    E: Clone,
    D1: Disposable,
    D2: Disposable,
{
    context.send_next_and_termination(
        DelegateAction::TerminateWindow(termination.clone()),
        termination,
    );
}

struct BoundaryObserver<'or, T, E, D1, D2>(WindowContext<'or, T, E, D1, D2>)
where
    D1: Disposable,
    D2: Disposable;

impl<T, E, D1, D2> Observer<(), E> for BoundaryObserver<'_, T, E, D1, D2>
where
    E: Clone,
    D1: Disposable,
    D2: Disposable,
{
    fn on_next(&mut self, _: ()) {
        let (sender, window) = unicast_subject();
        // Two events rather than one, so that disposing the outer subscription while the current
        // window is completing suppresses the new window.
        self.0.send_events(EventBatch::NextBatch(vec![
            DelegateAction::TerminateWindow(Termination::Completed),
            DelegateAction::EmitWindow(sender, window),
        ]));
    }

    fn on_termination(self, termination: Termination<E>) {
        // Completing the boundary only stops the rotation: the current window and the outer
        // Observable keep going.
        if let error @ Termination::Error(_) = termination {
            terminate(self.0, error);
        }
    }
}

enum DelegateAction<'or, T, E> {
    /// Sends a source value to the current window, if there is one.
    ForwardValue(T),
    /// Terminates the current window, if there is one.
    TerminateWindow(Termination<E>),
    /// Makes the new window the current one and emits it downstream.
    EmitWindow(UnicastSender<'or, T, E>, UnicastObservable<'or, T, E>),
}

struct DelegateObserver<'or, T, E> {
    outer_observer: BoxedObserver<'or, UnicastObservable<'or, T, E>, E>,
    /// The sending end of the window that is currently open, which is the only place where the
    /// windows are fed from. It is `None` before the first rotation and after the last window
    /// ended.
    sender: Option<UnicastSender<'or, T, E>>,
}

impl<'or, T, E> Observer<DelegateAction<'or, T, E>, E> for DelegateObserver<'or, T, E> {
    fn on_next(&mut self, action: DelegateAction<'or, T, E>) {
        match action {
            DelegateAction::ForwardValue(value) => match &mut self.sender {
                Some(sender) => sender.on_next(value),
                // No window accepts values, so this one has nowhere to go.
                None => drop(value),
            },
            DelegateAction::TerminateWindow(termination) => {
                if let Some(sender) = self.sender.take() {
                    sender.on_termination(termination);
                }
            }
            DelegateAction::EmitWindow(sender, window) => {
                debug_assert!(self.sender.is_none());
                self.sender = Some(sender);
                self.outer_observer.on_next(window);
            }
        };
    }

    fn on_termination(self, termination: Termination<E>) {
        // The current window is terminated by the `TerminateWindow` action that precedes this one.
        debug_assert!(self.sender.is_none());
        self.outer_observer.on_termination(termination);
    }
}
