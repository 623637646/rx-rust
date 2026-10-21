//! The [`RefCount`] operator, behind
//! [`ObservableExt::share`](crate::observable::ObservableExt::share),
//! [`ObservableExt::share_behavior`](crate::observable::ObservableExt::share_behavior),
//! [`ObservableExt::share_last`](crate::observable::ObservableExt::share_last),
//! [`ObservableExt::share_replay`](crate::observable::ObservableExt::share_replay).

use super::connectable_controller::{
    ConnectableController, Connected, ConnectionObserver, Current, Disconnected,
    MulticastObservable,
};
use crate::{
    delegate_disposal,
    disposable::{Disposable, chain_disposal::ChainDisposal, dispose_on_drop::DisposeOnDrop},
    observable::{Observable, ObservableTypes},
    observer::Observer,
    subject::Subject,
    thread_mode::{ThreadMode, mutable::MutableHelper},
    utils::{
        id_generator::Id,
        on_panic::{OnPanic, on_panic},
    },
};
use educe::Educe;

/// The controller of a [`RefCount`], in either of its states.
enum Controller<OE, S, C>
where
    OE: ObservableTypes,
    S: ObservableTypes,
{
    Disconnected(ConnectableController<OE, S, C, Disconnected>),
    Connected(ConnectableController<OE, S, C, Connected<<OE as ObservableTypes>::Disposal>>),
}

/// The subscribers of a [`RefCount`] and its controller, shared by its subscriptions.
///
/// Subscribers are counted per generation of the controller's subject. A generation ends when its
/// subject is reset — by the source terminating, or by this ref count when the last subscriber
/// leaves. A subscriber that arrives after that joins the next generation and starts the count
/// over, while the subscriptions of the previous one leave without touching it.
struct State<OE, S, C>
where
    OE: ObservableTypes,
    S: ObservableTypes,
{
    reset_on_ref_count_zero: bool,
    /// The generation the subscribers are counted for, `None` before the first one.
    generation: Option<Id>,
    subscribers: usize,
    /// `None` while a caller holds the controller to switch it over with the lock released. Once
    /// that is done, the holder gives it back and decides again: the subscribers that arrived or
    /// left in the meantime may need it switched once more.
    controller: Option<Controller<OE, S, C>>,
}

/// What [`State::decide`] leaves to do with the lock released.
///
/// A decision that takes the controller out of the state must be carried out or cancelled, which
/// is what gives it back: dropping one strands the controller, so that the ref count would never
/// connect or disconnect again.
#[must_use = "a decision must be carried out or cancelled, which gives the controller back"]
enum Decision<OE, S, C>
where
    OE: ObservableTypes,
    S: ObservableTypes,
{
    /// The controller is where the subscribers need it, or a caller is holding it.
    Keep,
    /// The subscribers' generation is live and waiting for the source: connect it.
    Connect(ConnectableController<OE, S, C, Disconnected>),
    /// The source is connected for a generation the subscribers no longer need: disconnect it,
    /// after dropping the subject reset under the lock, if any.
    Disconnect {
        controller: ConnectableController<OE, S, C, Connected<<OE as ObservableTypes>::Disposal>>,
        replaced: Option<S>,
    },
}

impl<OE, S, C> Decision<OE, S, C>
where
    OE: ObservableTypes,
    S: ObservableTypes,
{
    /// Carries the decision out, and returns the switched controller to give back. It runs with
    /// the lock released: connecting subscribes to the source, which may emit to subscribers that
    /// subscribe to or dispose this ref count from there.
    ///
    /// Connecting and disconnecting run the user's code, which may unwind: the controller is then
    /// given back to `state` disconnected, since no connection is left, and nothing is decided on
    /// the unwinding thread, where connecting again could panic once more. The next subscription
    /// or disposal decides instead.
    fn carry_out<T, E>(self, state: &StatePtr<OE, S, C>) -> Option<Controller<OE, S, C>>
    where
        OE: Observable<ConnectionObserver<S, C>, Item = T, Error = E> + Clone,
        S: Subject<T, E> + Clone,
    {
        let give_back = |controller| {
            state.with_mut(|current| current.give_back(Controller::Disconnected(controller)));
        };
        match self {
            Self::Keep => None,
            Self::Connect(controller) => {
                Some(Controller::Connected(controller.connect_or_else(give_back)))
            }
            Self::Disconnect {
                controller,
                replaced,
            } => {
                let (controller, connection) = controller.take_connection();
                let guard = OnPanic::new(controller, give_back);
                drop(replaced);
                drop(connection);
                Some(Controller::Disconnected(guard.disarm()))
            }
        }
    }

    /// Calls the decision off, for a caller that gives the controller back without switching it:
    /// returns the controller taken out, as it was.
    ///
    /// `replaced` is always `None` here: only the count reaching zero resets a subject, and the
    /// one caller that cancels a decision has just counted a subscriber in (`RefCount::subscribe`,
    /// where a `debug_assert` says so). A subject reset after all is dropped rather than put back,
    /// since the state has moved on from it.
    fn cancel(self) -> Option<Controller<OE, S, C>> {
        match self {
            Self::Keep => None,
            Self::Connect(controller) => Some(Controller::Disconnected(controller)),
            Self::Disconnect {
                controller,
                replaced,
            } => {
                drop(replaced);
                Some(Controller::Connected(controller))
            }
        }
    }
}

impl<OE, S, C> State<OE, S, C>
where
    OE: ObservableTypes,
    S: ObservableTypes,
{
    /// Counts in a subscriber of `generation`, starting the count over if it is a new one.
    fn count_in(&mut self, generation: Id) {
        if self.generation != Some(generation) {
            self.generation = Some(generation);
            self.subscribers = 0;
        }
        self.subscribers = self
            .subscribers
            .checked_add(1)
            .expect("RefCount subscriber count overflowed");
    }

    /// Counts out a subscriber of `generation`, unless its generation is over.
    fn count_out(&mut self, generation: Id) {
        if self.generation == Some(generation) {
            self.subscribers = self
                .subscribers
                .checked_sub(1)
                .expect("RefCount subscriber count underflowed");
        }
    }

    /// Gives back the controller a caller took out to switch over.
    fn give_back(&mut self, controller: Controller<OE, S, C>) {
        debug_assert!(self.controller.is_none(), "the controller is held once");
        self.controller = Some(controller);
    }

    /// Decides what to do with the controller, unless a caller is holding it: leaves it in place if
    /// it is where the subscribers need it, and takes it out to be switched over otherwise.
    fn decide(&mut self) -> Decision<OE, S, C> {
        match self.controller.take() {
            Some(Controller::Disconnected(controller))
                if self.subscribers > 0
                    && self
                        .generation
                        .is_some_and(|generation| controller.is_live(generation)) =>
            {
                Decision::Connect(controller)
            }
            // A newer generation has begun, and is waiting for the source.
            Some(Controller::Connected(controller))
                if self.generation != Some(controller.generation()) =>
            {
                Decision::Disconnect {
                    controller,
                    replaced: None,
                }
            }
            // The subject is reset under this lock, so that a subscriber that arrives from now on
            // joins a new generation instead of a subject that is about to lose its source.
            Some(Controller::Connected(controller))
                if self.subscribers == 0 && self.reset_on_ref_count_zero =>
            {
                let replaced = controller.reset_live();
                Decision::Disconnect {
                    controller,
                    replaced,
                }
            }
            kept => {
                self.controller = kept;
                Decision::Keep
            }
        }
    }
}

/// Settles `decision` with the lock released — dropping the reset subject and switching the
/// controller over — then gives the controller back and settles whatever the state decides next,
/// until it keeps it.
///
/// Its callers change the state and decide under one lock (`state.with_mut(|current| { …;
/// current.decide() })`), so that nothing can join a subject between the change and the decision
/// it leads to, and hand the decision here to be carried out with the lock released.
fn settle<T, E, OE, S, C>(state: &StatePtr<OE, S, C>, mut decision: Decision<OE, S, C>)
where
    OE: Observable<ConnectionObserver<S, C>, Item = T, Error = E> + Clone,
    S: Subject<T, E> + Clone,
{
    while let Some(switched) = decision.carry_out(state) {
        decision = state.with_mut(|current| {
            current.give_back(switched);
            current.decide()
        });
    }
}

/// Makes a [`ConnectableController`] behave like an ordinary `Observable` that automatically
/// connects on the first subscription and disconnects when the last subscription is disposed. See
/// <https://reactivex.io/documentation/operators/refcount.html>
///
/// A subscriber joins the controller's current subject. Once that subject is reset — when the
/// source terminates, as the controller's
/// [`reset_on_error`](ConnectableController::reset_on_error) and
/// [`reset_on_complete`](ConnectableController::reset_on_complete) say, or when the last subscriber
/// leaves, as [`reset_on_ref_count_zero`](Self::reset_on_ref_count_zero) says — the next
/// subscriber starts over with a new subject and a new connection.
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt,
///     observer::Termination,
///     operators::{
///         connectable::connectable_controller::ConnectableController,
///         creating::from_iter::FromIter,
///     },
///     subject::publish_subject::PublishSubject,
/// };
/// use std::{convert::Infallible, sync::{Arc, Mutex}};
///
/// let values = Arc::new(Mutex::new(Vec::new()));
/// let terminations = Arc::new(Mutex::new(Vec::new()));
///
/// let controller = ConnectableController::new(FromIter::new(vec![1, 2]), (), |_| {
///     PublishSubject::<i32, Infallible, _>::local()
/// });
/// let observable = controller.ref_count();
///
/// for _ in 0..2 {
///     let values_observer = Arc::clone(&values);
///     let terminations_observer = Arc::clone(&terminations);
///     observable.clone().subscribe_with_callback(
///         move |value| values_observer.lock().unwrap().push(value),
///         move |termination| terminations_observer.lock().unwrap().push(termination),
///     );
/// }
///
/// // The source completed, so the second subscriber started over with a new subject.
/// assert_eq!(&*values.lock().unwrap(), &[1, 2, 1, 2]);
/// assert_eq!(
///     &*terminations.lock().unwrap(),
///     &[Termination::Completed, Termination::Completed],
/// );
/// ```
#[derive(Educe)]
#[educe(Debug, Clone(bound()))]
pub struct RefCount<OE, S, C = ()>
where
    OE: ObservableTypes,
    S: ObservableTypes,
{
    #[educe(Debug(ignore))]
    state: StatePtr<OE, S, C>,
    /// The controller's observable side, through which subscribers join its current subject even
    /// while a subscription holds the controller. Its lock is always taken after the state's.
    #[educe(Debug(ignore))]
    observable: MulticastObservable<S, C>,
}

/// The shared state of a [`RefCount`], behind the pointer the subject's thread mode picks — the
/// mode of the [`RefCount`] itself: it is reached from wherever subscribers subscribe and dispose,
/// and the source is connected and disconnected from there too.
///
/// The source's mode does not enter it. A `Local` value source such as `FromIter` is still `Send`,
/// and multicast through a `Shared` subject the `RefCount` must be `Send` as its mode says; a
/// `Shared` source multicast through a `Local` subject is confined to one thread by the subject.
type StatePtr<OE, S, C> = <<S as ObservableTypes>::Mode as ThreadMode>::Ptr<State<OE, S, C>>;

impl<OE, S, C> RefCount<OE, S, C>
where
    OE: ObservableTypes,
    S: ObservableTypes,
{
    /// Creates a [`RefCount`] over a disconnected controller;
    /// [`ConnectableController::ref_count`] is the fluent form.
    ///
    /// [`reset_on_ref_count_zero`](Self::reset_on_ref_count_zero) starts out `true`.
    pub fn new(controller: ConnectableController<OE, S, C, Disconnected>) -> Self {
        Self {
            observable: controller.observable(),
            state: S::Mode::ptr(State {
                reset_on_ref_count_zero: true,
                generation: None,
                subscribers: 0,
                controller: Some(Controller::Disconnected(controller)),
            }),
        }
    }

    /// What happens when the last subscriber leaves before the source terminates: `true`, the
    /// default, disconnects the source and resets the subject, so the next subscriber starts over;
    /// `false` keeps the source connected and the subject alive, so the next subscriber joins it.
    ///
    /// A subject kept after its source terminated is not reset by this either way: whether it is
    /// is what the controller's [`reset_on_error`](ConnectableController::reset_on_error) and
    /// [`reset_on_complete`](ConnectableController::reset_on_complete) decide.
    pub fn reset_on_ref_count_zero(self, reset: bool) -> Self {
        self.state
            .with_mut(|state| state.reset_on_ref_count_zero = reset);
        self
    }
}

delegate_disposal!(
    Disposal<OE, S, C>,
    ChainDisposal<<S as ObservableTypes>::Disposal, RefCountDisposal<OE, S, C>>,
    where OE: ObservableTypes,
        S: ObservableTypes
);

impl<T, E, OE, S, C> ObservableTypes for RefCount<OE, S, C>
where
    OE: Observable<ConnectionObserver<S, C>, Item = T, Error = E> + Clone,
    S: Subject<T, E> + Clone,
{
    type Item = T;
    type Error = E;
    /// The subscribers are the subject's, so the mode is the subject's.
    type Mode = S::Mode;
    type Disposal = Disposal<OE, S, C>;
}

impl<T, E, OE, S, C, OR> Observable<OR> for RefCount<OE, S, C>
where
    OR: Observer<T, E>,
    OE: Observable<ConnectionObserver<S, C>, Item = T, Error = E> + Clone,
    S: Subject<T, E> + Observable<OR> + Clone,
{
    fn subscribe(self, observer: OR) -> DisposeOnDrop<Self::Disposal> {
        let (subject, generation, decision) = self.state.with_mut(|state| {
            let Current {
                subject,
                generation,
                finished,
            } = self.observable.join();
            if finished {
                // The source has terminated and its subject is kept: there is nothing to connect,
                // and the subject only replays what it has to this subscriber, so it is not counted.
                return (subject, None, Decision::Keep);
            }
            state.count_in(generation);
            // Deciding under the same lock reserves the connection for this subscriber: while it
            // holds the controller, no other caller can connect the source before this subscriber
            // has joined the subject, and make it miss what the source sends as it is connected.
            let decision = state.decide();
            // Counting a subscriber in never brings the count to zero, so nothing is reset here.
            debug_assert!(
                !matches!(
                    decision,
                    Decision::Disconnect {
                        replaced: Some(_),
                        ..
                    }
                ),
                "counting a subscriber in resets nothing"
            );
            (subject, Some(generation), decision)
        });
        // Subscribing notifies the observer inline when the subject has something to replay, so it
        // can unwind while this subscriber is counted in, and maybe holding the controller: the
        // guard counts it out and gives the controller back. Without it, the count would stay
        // inflated forever, so the last subscription would no longer disconnect, and the controller
        // would be lost. Disarming the guard is what ends that scope.
        let state = self.state.clone();
        let guard = OnPanic::new(decision, move |decision| {
            let controller = decision.cancel();
            let decision = state.with_mut(|current| {
                if let Some(generation) = generation {
                    current.count_out(generation);
                }
                if let Some(controller) = controller {
                    current.give_back(controller);
                }
                current.decide()
            });
            settle(&state, decision);
        });
        let subscription = subject.subscribe(observer);
        let decision = guard.disarm();
        // Connecting can unwind too, once `settle` has given the controller back: this guard then
        // counts the subscriber out, without deciding again, as `Decision::carry_out` explains.
        let guard = on_panic(|| {
            if let Some(generation) = generation {
                self.state.with_mut(|current| current.count_out(generation));
            }
        });
        settle(&self.state, decision);
        drop(guard);
        subscription
            .then(RefCountDisposal {
                state: self.state,
                generation,
            })
            .map_inner_into()
    }
}

struct RefCountDisposal<OE, S, C>
where
    OE: ObservableTypes,
    S: ObservableTypes,
{
    state: StatePtr<OE, S, C>,
    /// The generation this subscriber was counted for, `None` if it was not counted.
    generation: Option<Id>,
}

impl<T, E, OE, S, C> Disposable for RefCountDisposal<OE, S, C>
where
    OE: Observable<ConnectionObserver<S, C>, Item = T, Error = E> + Clone,
    S: Subject<T, E> + Clone,
{
    fn dispose(self) {
        let Some(generation) = self.generation else {
            return;
        };
        let decision = self.state.with_mut(|current| {
            current.count_out(generation);
            current.decide()
        });
        settle(&self.state, decision);
    }
}
