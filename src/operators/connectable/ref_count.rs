use crate::delegate_disposal;
use crate::disposable::{Disposable, chain_disposal::ChainDisposal};
use crate::observable::{Observable, Subscription};
use crate::observer::Observer;
use crate::operators::connectable::connectable_controller::{
    ConnectableController, Connected, Disconnected,
};
use crate::subject::Subject;
use crate::subject::subject_observable::SubjectObservable;
use crate::utils::mutable::{Mutable, MutableHelper};
use crate::utils::on_panic::OnPanic;
use crate::utils::types::{MaybeSend, Shared};
use educe::Educe;
use std::num::NonZeroUsize;

#[derive(Educe)]
#[educe(Debug)]
enum State<OE, S, D>
where
    D: Disposable,
{
    Disconnected {
        controller: ConnectableController<OE, S, Disconnected>,
    },
    ConnectingOrDisconnecting {
        /// Use `usize` instead of `NonZeroUsize`. If it's 0, there are no subscribers and should
        /// disconnect. If it's not 0, there is at least one subscriber and should connect.
        subscribers: usize,
    },
    Connected {
        subscribers: NonZeroUsize,
        controller: ConnectableController<OE, S, Connected<D>>,
    },
}

/// Makes a [`ConnectableController`] behave like an ordinary `Observable` that automatically connects
/// on the first subscription and disconnects when the last subscription is disposed.
/// See <https://reactivex.io/documentation/operators/refcount.html>
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt,
///     observer::Termination,
///     operators::{
///         connectable::{connectable_controller::ConnectableController, ref_count::RefCount},
///         creating::from_iter::FromIter,
///     },
///     subject::publish_subject::PublishSubject,
/// };
/// use std::{convert::Infallible, sync::{Arc, Mutex}};
///
/// let values = Arc::new(Mutex::new(Vec::new()));
/// let terminations = Arc::new(Mutex::new(Vec::new()));
///
/// let subject: PublishSubject<'_, i32, Infallible> = PublishSubject::default();
/// let controller = ConnectableController::new(FromIter::new(vec![1, 2]), subject);
/// let observable = controller.ref_count();
/// let values_observer = Arc::clone(&values);
/// let terminations_observer = Arc::clone(&terminations);
///
/// let subscription = observable.clone().subscribe_with_callback(
///     move |value| values_observer.lock().unwrap().push(value),
///     move |termination| terminations_observer
///         .lock()
///         .unwrap()
///         .push(termination),
/// );
///
/// drop(subscription);
///
/// assert_eq!(&*values.lock().unwrap(), &[1, 2]);
/// assert_eq!(
///     &*terminations.lock().unwrap(),
///     &[Termination::Completed]
/// );
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct RefCount<'or, T, E, OE, S>
where
    OE: Observable<'or, T, E>,
{
    observable: SubjectObservable<S>,
    state: Shared<Mutable<State<OE, S, OE::D>>>,
}

impl<'or, T, E, OE, S> RefCount<'or, T, E, OE, S>
where
    OE: Observable<'or, T, E>,
    S: Clone,
{
    pub fn new(controller: ConnectableController<OE, S, Disconnected>) -> Self {
        Self {
            observable: controller.observable(),
            state: Shared::new(Mutable::new(State::Disconnected { controller })),
        }
    }
}

delegate_disposal!(
    Disposal<'or, T, E, OE, S>,
    ChainDisposal<S::D, RefCountDisposal<'or, T, E, OE, S>>,
    where OE: Observable<'or, T, E>,
        S: Subject<'or, T, E>
);

impl<'or, T, E, OE, S> Observable<'or, T, E> for RefCount<'or, T, E, OE, S>
where
    OE: Observable<'or, T, E> + Clone,
    S: Subject<'or, T, E> + Clone + MaybeSend + 'or,
{
    type D = Disposal<'or, T, E, OE, S>;

    fn subscribe(self, observer: impl Observer<T, E> + MaybeSend + 'or) -> Subscription<Self::D> {
        let controller = self.state.with_mut(|current| match &mut *current {
            state @ State::Disconnected { .. } => {
                let State::Disconnected { controller } =
                    std::mem::replace(state, State::ConnectingOrDisconnecting { subscribers: 1 })
                else {
                    unreachable!()
                };
                Some(controller)
            }
            State::ConnectingOrDisconnecting { subscribers } => {
                *subscribers = subscribers
                    .checked_add(1)
                    .expect("RefCount subscriber count overflowed");
                None
            }
            State::Connected { subscribers, .. } => {
                *subscribers = subscribers
                    .checked_add(1)
                    .expect("RefCount subscriber count overflowed");
                None
            }
        });
        // Subscribing notifies the observer inline when the subject has already terminated, so it
        // can unwind while this subscriber is counted in and the controller is out of the state.
        // Disarming the guard is what ends that scope: the connection that follows is not covered,
        // and cannot be, since the controller is consumed by then.
        let state = self.state.clone();
        let guard = OnPanic::new(controller, move |controller| {
            restore_subscriber(&state, controller);
        });
        let sub = self.observable.subscribe(observer);
        let controller = guard.disarm();
        if let Some(controller) = controller {
            handle_connecting_or_disconnecting(self.state.clone(), Purpose::Connect(controller));
        }
        sub.then(RefCountDisposal { state: self.state }).map_into()
    }
}

/// Puts back what [`RefCount::subscribe`] took, when subscribing the observer unwinds.
///
/// The subscriber is counted in — and the controller of a first subscription is taken out of the
/// state — before the observer is subscribed to the subject. Without this, a panic from there
/// would leave the count inflated forever, so the last subscription would no longer disconnect,
/// and would lose the controller with the state stuck in
/// [`ConnectingOrDisconnecting`](State::ConnectingOrDisconnecting).
fn restore_subscriber<'or, T, E, OE, S>(
    state: &Shared<Mutable<State<OE, S, OE::D>>>,
    controller: Option<ConnectableController<OE, S, Disconnected>>,
) where
    OE: Observable<'or, T, E> + Clone,
    S: Observer<T, E> + Clone + MaybeSend + 'or,
{
    let Some(controller) = controller else {
        // Only the count was changed, so removing this subscriber is what disposing the
        // subscription it never got would have done.
        RefCountDisposal {
            state: state.clone(),
        }
        .dispose();
        return;
    };
    // The controller is held here, so nothing else can leave the connecting state, and the count
    // can only have grown: the state is the one this subscription wrote.
    let controller = state.with_mut(|current| {
        let State::ConnectingOrDisconnecting { subscribers } = current else {
            unreachable!("the controller of the connecting state is held by this guard")
        };
        let remaining = subscribers
            .checked_sub(1)
            .expect("RefCount subscriber count underflowed");
        if remaining == 0 {
            // Nobody is left to connect for, so the state goes back to what it was.
            *current = State::Disconnected { controller };
            None
        } else {
            *subscribers = remaining;
            Some(controller)
        }
    });
    if let Some(controller) = controller {
        // Subscribers that arrived meanwhile are still waiting for the connection this
        // subscription was going to make, so it is made here, as the returning path would.
        handle_connecting_or_disconnecting(state.clone(), Purpose::Connect(controller));
    }
}

struct RefCountDisposal<'or, T, E, OE, S>
where
    OE: Observable<'or, T, E>,
{
    state: Shared<Mutable<State<OE, S, OE::D>>>,
}

impl<'or, T, E, OE, S> Disposable for RefCountDisposal<'or, T, E, OE, S>
where
    OE: Observable<'or, T, E> + Clone,
    S: Observer<T, E> + Clone + MaybeSend + 'or,
{
    fn dispose(self) {
        let controller = self.state.with_mut(|current| match &mut *current {
            State::Disconnected { .. } => unreachable!(),
            State::ConnectingOrDisconnecting { subscribers } => {
                *subscribers = subscribers
                    .checked_sub(1)
                    .expect("RefCount subscriber count underflowed");
                None
            }
            State::Connected { subscribers, .. } if subscribers.get() > 1 => {
                *subscribers = NonZeroUsize::new(subscribers.get() - 1)
                    .expect("decremented subscriber count is non-zero");
                None
            }
            State::Connected { .. } => {
                let State::Connected { controller, .. } =
                    std::mem::replace(current, State::ConnectingOrDisconnecting { subscribers: 0 })
                else {
                    unreachable!()
                };
                Some(controller)
            }
        });
        if let Some(controller) = controller {
            handle_connecting_or_disconnecting(self.state.clone(), Purpose::Disconnect(controller));
        }
    }
}

enum Purpose<'or, T, E, OE, S>
where
    OE: Observable<'or, T, E>,
{
    Connect(ConnectableController<OE, S, Disconnected>),
    Disconnect(ConnectableController<OE, S, Connected<OE::D>>),
}

fn handle_connecting_or_disconnecting<'or, T, E, OE, S>(
    state: Shared<Mutable<State<OE, S, OE::D>>>,
    mut purpose: Purpose<'or, T, E, OE, S>,
) where
    OE: Observable<'or, T, E> + Clone,
    S: Observer<T, E> + Clone + MaybeSend + 'or,
{
    loop {
        let next = match purpose {
            Purpose::Connect(controller) => {
                let controller = controller.connect();
                state.with_mut(|current| match &mut *current {
                    State::Disconnected { .. } => unreachable!(),
                    State::ConnectingOrDisconnecting { subscribers } => {
                        if *subscribers == 0 {
                            // It's unsubscribed, so we should disconnect
                            Some(Purpose::Disconnect(controller))
                        } else {
                            *current = State::Connected {
                                subscribers: NonZeroUsize::new(*subscribers).unwrap(),
                                controller,
                            };
                            None
                        }
                    }
                    State::Connected { .. } => unreachable!(),
                })
            }
            Purpose::Disconnect(controller) => {
                let controller = controller.disconnect();
                state.with_mut(|current| match &mut *current {
                    State::Disconnected { .. } => unreachable!(),
                    State::ConnectingOrDisconnecting { subscribers } => {
                        if *subscribers == 0 {
                            *current = State::Disconnected { controller };
                            None
                        } else {
                            // It's not unsubscribed, so we should reconnect
                            Some(Purpose::Connect(controller))
                        }
                    }
                    State::Connected { .. } => unreachable!(),
                })
            }
        };
        match next {
            Some(next) => purpose = next,
            None => break,
        }
    }
}
