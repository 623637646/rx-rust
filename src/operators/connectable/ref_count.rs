//! The [`RefCount`] operator, behind
//! [`ObservableExt::share`](crate::observable::ObservableExt::share),
//! [`ObservableExt::share_last`](crate::observable::ObservableExt::share_last),
//! [`ObservableExt::share_replay`](crate::observable::ObservableExt::share_replay).

use crate::delegate_disposal;
use crate::disposable::{Disposable, chain_disposal::ChainDisposal};
use crate::observer::Observer;
use crate::operators::connectable::connectable_controller::{
    ConnectableController, Connected, Disconnected,
};
use crate::subject::Subject;
use crate::subject::SubjectObservable;
use crate::thread_mode::ThreadMode;
use crate::thread_mode::mutable::MutableHelper;
use crate::utils::on_panic::OnPanic;
use crate::{
    disposable::dispose_on_drop::DisposeOnDrop,
    observable::{Observable, ObservableTypes},
};
use educe::Educe;
use std::num::NonZeroUsize;

/// The connection of a [`RefCount`] and its number of subscribers, shared by its subscriptions.
enum State<OE, S, D>
where
    D: Disposable,
{
    /// No subscriber, and the source is not connected.
    Disconnected {
        controller: ConnectableController<OE, S, Disconnected>,
    },
    /// A subscription holds the controller while it connects or disconnects the source with the
    /// lock released.
    ConnectingOrDisconnecting {
        /// The subscribers so far, possibly none: once it is done, the holder of the controller
        /// disconnects if there are none and connects if there are some.
        subscribers: usize,
    },
    /// Connected, with at least one subscriber.
    Connected {
        subscribers: NonZeroUsize,
        controller: ConnectableController<OE, S, Connected<D>>,
    },
}

/// Makes a [`ConnectableController`] behave like an ordinary `Observable` that automatically
/// connects on the first subscription and disconnects when the last subscription is disposed. See
/// <https://reactivex.io/documentation/operators/refcount.html>
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
/// let subject: PublishSubject<'_, i32, Infallible, rx_rust::thread_mode::Local> = PublishSubject::local();
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
#[educe(Debug, Clone(bound(S: Clone)))]
pub struct RefCount<OE, S>
where
    OE: ObservableTypes,
{
    #[educe(Debug(ignore))]
    observable: SubjectObservable<S>,
    #[educe(Debug(ignore))]
    state: StatePtr<OE, S>,
}

/// The shared state of a [`RefCount`], behind the pointer the source's thread mode picks: it is
/// reached from wherever subscribers subscribe and dispose, which is where the subject the source
/// feeds is reached from too.
type StatePtr<OE, S> = <<OE as ObservableTypes>::Mode as ThreadMode>::Ptr<
    State<OE, S, <OE as ObservableTypes>::Disposal>,
>;

impl<OE, S> RefCount<OE, S>
where
    OE: ObservableTypes,
    S: Clone,
{
    /// Creates a [`RefCount`] over a disconnected controller;
    /// [`ConnectableController::ref_count`] is the fluent form.
    pub fn new(controller: ConnectableController<OE, S, Disconnected>) -> Self {
        Self {
            observable: controller.observable(),
            state: OE::Mode::ptr(State::Disconnected { controller }),
        }
    }
}

delegate_disposal!(
    Disposal<OE, S>,
    ChainDisposal<S::Disposal, RefCountDisposal<OE, S>>,
    where OE: ObservableTypes,
        S: ObservableTypes
);

impl<T, E, OE, S> ObservableTypes for RefCount<OE, S>
where
    OE: Observable<S, Item = T, Error = E> + Clone,
    S: Subject<T, E> + Clone,
{
    type Item = T;
    type Error = E;
    /// The subscribers are the subject's, so the mode is the subject's.
    type Mode = S::Mode;
    type Disposal = Disposal<OE, S>;
}

impl<T, E, OE, S, OR> Observable<OR> for RefCount<OE, S>
where
    OR: Observer<T, E>,
    OE: Observable<S, Item = T, Error = E> + Clone,
    S: Subject<T, E> + Observable<OR> + Clone,
{
    fn subscribe(self, observer: OR) -> DisposeOnDrop<Self::Disposal> {
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
        sub.then(RefCountDisposal { state: self.state })
            .map_inner_into()
    }
}

/// Puts back what [`RefCount::subscribe`] took, when subscribing the observer unwinds.
///
/// The subscriber is counted in — and the controller of a first subscription is taken out of the
/// state — before the observer is subscribed to the subject. Without this, a panic from there
/// would leave the count inflated forever, so the last subscription would no longer disconnect,
/// and would lose the controller with the state stuck in
/// [`ConnectingOrDisconnecting`](State::ConnectingOrDisconnecting).
fn restore_subscriber<T, E, OE, S>(
    state: &StatePtr<OE, S>,
    controller: Option<ConnectableController<OE, S, Disconnected>>,
) where
    OE: Observable<S, Item = T, Error = E> + Clone,
    S: Subject<T, E> + Clone,
{
    let Some(controller) = controller else {
        // Only the count was changed, so removing this subscriber is what disposing the
        // subscription it never got would have done.
        RefCountDisposal::<OE, S> {
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

struct RefCountDisposal<OE, S>
where
    OE: ObservableTypes,
{
    state: StatePtr<OE, S>,
}

impl<T, E, OE, S> Disposable for RefCountDisposal<OE, S>
where
    OE: Observable<S, Item = T, Error = E> + Clone,
    S: Subject<T, E> + Clone,
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

enum Purpose<OE, S>
where
    OE: ObservableTypes,
{
    Connect(ConnectableController<OE, S, Disconnected>),
    Disconnect(ConnectableController<OE, S, Connected<OE::Disposal>>),
}

fn handle_connecting_or_disconnecting<T, E, OE, S>(
    state: StatePtr<OE, S>,
    mut purpose: Purpose<OE, S>,
) where
    OE: Observable<S, Item = T, Error = E> + Clone,
    S: Subject<T, E> + Clone,
{
    loop {
        let next = match purpose {
            Purpose::Connect(controller) => {
                let controller = controller.connect();
                state.with_mut(|current| match &mut *current {
                    State::Disconnected { .. } => unreachable!(),
                    State::ConnectingOrDisconnecting { subscribers } => {
                        if *subscribers == 0 {
                            // Every subscriber left while connecting: disconnect again.
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
                            // Subscribers arrived while disconnecting: connect again.
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
