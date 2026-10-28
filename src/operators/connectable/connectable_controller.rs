//! The [`ConnectableController`], behind
//! [`ObservableExt::multicast`](crate::observable::ObservableExt::multicast),
//! [`ObservableExt::publish`](crate::observable::ObservableExt::publish),
//! [`ObservableExt::publish_behavior`](crate::observable::ObservableExt::publish_behavior),
//! [`ObservableExt::publish_last`](crate::observable::ObservableExt::publish_last),
//! [`ObservableExt::replay`](crate::observable::ObservableExt::replay).

use super::ref_count::RefCount;
use crate::{
    disposable::{Disposable, dispose_on_drop::DisposeOnDrop},
    observable::{Observable, ObservableTypes},
    observer::{Flow, Observer, Termination},
    subject::Subject,
    thread_mode::{ThreadMode, mutable::MutableHelper},
    utils::{
        id_generator::{Id, IdGenerator},
        on_panic::OnPanic,
    },
};
use educe::Educe;

/// Marker for a connectable controller that is not connected to its source.
#[derive(Debug, Clone, Copy, Default)]
pub struct Disconnected;

/// State carried by a connected controller. Dropping it disconnects the source.
#[derive(Educe)]
#[educe(Debug)]
pub struct Connected<D: Disposable> {
    connection: DisposeOnDrop<D>,
    /// The generation of the subject the source is connected to.
    generation: Id,
}

/// The state shared by a controller, the observables it hands out and the observer its source is
/// subscribed with. It is private to this module: [`RefCount`] reaches it only through the
/// `pub(super)` methods of [`ConnectableController`] and [`MulticastObservable`], each of which
/// takes its lock once, after the ref count's own.
type SubjectLifecyclePtr<S, C> =
    <<S as ObservableTypes>::Mode as ThreadMode>::Ptr<SubjectLifecycle<S, C>>;

/// The subject of the current connection, and what decides when it is replaced.
///
/// Each subject has a generation, an id handed out when the subject is created and dropped with it
/// when it is reset, so that a connection, a subscriber or a [`RefCount`] that is still holding on
/// to an older one can tell. The subject itself is created lazily, by the first subscriber or
/// connection that needs it.
struct SubjectLifecycle<S, C> {
    context: C,
    subject_factory: fn(&mut C) -> S,
    reset_on_error: bool,
    reset_on_complete: bool,
    generations: IdGenerator,
    /// `None` until a subscriber or a connection needs a subject, and again after each reset.
    current: Option<Current<S>>,
}

/// The current subject with its generation, and whether its source has terminated, in which case
/// the subject is kept for late subscribers.
#[derive(Educe)]
#[educe(Clone)]
pub(super) struct Current<S> {
    pub(super) subject: S,
    pub(super) generation: Id,
    pub(super) finished: bool,
}

impl<S, C> SubjectLifecycle<S, C> {
    /// The current subject, created by the factory if there is none yet.
    fn current_or_create(&mut self) -> Current<S>
    where
        S: Clone,
    {
        let Self {
            context,
            subject_factory,
            generations,
            current,
            ..
        } = self;
        current
            .get_or_insert_with(|| Current {
                subject: subject_factory(context),
                generation: generations.next_id(),
                finished: false,
            })
            .clone()
    }

    /// The current subject, if it is the one of `generation`.
    fn current_of(&mut self, generation: Id) -> Option<&mut Current<S>> {
        self.current
            .as_mut()
            .filter(|current| current.generation == generation)
    }

    /// Whether `generation` is the current one and its source has not terminated.
    fn is_live(&self, generation: Id) -> bool {
        self.current
            .as_ref()
            .is_some_and(|current| current.generation == generation && !current.finished)
    }

    /// Resets the subject of `generation` if it is still the current one, whatever happened to its
    /// source. The replaced subject is returned, to be dropped outside the lock.
    fn reset(&mut self, generation: Id) -> Option<S> {
        self.current
            .take_if(|current| current.generation == generation)
            .map(|current| current.subject)
    }

    /// Resets the subject of `generation` if it is still the current one and its source has not
    /// terminated: a subject kept after its source terminated outlives its subscribers. The
    /// replaced subject is returned, to be dropped outside the lock.
    fn reset_live(&mut self, generation: Id) -> Option<S> {
        self.current
            .take_if(|current| current.generation == generation && !current.finished)
            .map(|current| current.subject)
    }

    /// Records that the source connected to `generation` has terminated, which resets the subject
    /// or keeps it as the configuration says. The replaced subject is returned, to be dropped
    /// outside the lock.
    fn terminate<E>(&mut self, generation: Id, termination: &Termination<E>) -> Option<S> {
        let reset = match termination {
            Termination::Completed => self.reset_on_complete,
            Termination::Error(_) => self.reset_on_error,
        };
        if reset {
            return self.reset(generation);
        }
        if let Some(current) = self.current_of(generation) {
            current.finished = true;
        }
        None
    }
}

/// Multicasts a source `Observable` through a `Subject`, but waits until its
/// [`connect`](ConnectableController::connect) method is called before subscribing to the source
/// and emitting items to its observers. Subscribe to the multicast output through
/// [`observable`](ConnectableController::observable). See
/// <https://reactivex.io/documentation/operators/connect.html>
///
/// The subject is created by a subject factory, called with the controller's context, and replaced
/// by a new one when it is reset: when the controller is
/// [disconnected](ConnectableController::disconnect), when the source terminates, unless
/// [`reset_on_error`](ConnectableController::reset_on_error) or
/// [`reset_on_complete`](ConnectableController::reset_on_complete) keeps it, and when the last
/// subscriber of a [`RefCount`] leaves, unless
/// [`reset_on_ref_count_zero`](RefCount::reset_on_ref_count_zero) keeps it. A subscriber always
/// joins the current subject, so one that arrives after a reset waits for the next connection
/// instead of receiving what the previous one left behind.
///
/// The factory is a function pointer, so that the controller's type can be written out: what it
/// needs goes in the context — `()` when nothing, the initial value of a `BehaviorSubject`, the
/// buffer size of a `ReplaySubject` — and a factory returning a clone of a subject kept in the
/// context keeps that subject across every connection. It is called lazily, when a subscriber or a
/// connection first needs a subject, with the controller's lock held — and the [`RefCount`]'s too,
/// when one of its subscribers needs it — so it must not subscribe to, connect or disconnect that
/// controller, nor subscribe to or dispose a subscription of a [`RefCount`] of it. Whatever it
/// runs is bound by that: the `Clone` of a value it copies from the context, such as the initial
/// value of a `BehaviorSubject`. So is the subject's own `Clone`, which runs under the same locks
/// each time a subscriber or a connection joins the subject.
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt,
///     operators::{
///         connectable::connectable_controller::ConnectableController,
///         creating::from_iter::FromIter,
///     },
///     subject::publish_subject::PublishSubject,
/// };
/// use std::cell::RefCell;
///
/// let values = RefCell::new(Vec::new());
/// let controller = ConnectableController::new(FromIter::new(vec![1, 2]), (), |_| {
///     PublishSubject::local()
/// });
/// let observable = controller.observable();
/// let _subscription_1 = observable
///     .clone()
///     .subscribe_with_callback(|value| values.borrow_mut().push(("first", value)), |_| {});
/// let _subscription_2 = observable
///     .subscribe_with_callback(|value| values.borrow_mut().push(("second", value)), |_| {});
/// assert!(values.borrow().is_empty()); // Nothing is emitted until the source is connected.
///
/// // Dropping the connected controller would disconnect the source.
/// let _connected = controller.connect();
/// assert_eq!(
///     *values.borrow(),
///     [("first", 1), ("second", 1), ("first", 2), ("second", 2)]
/// );
/// ```
#[derive(Educe)]
#[educe(Debug)]
pub struct ConnectableController<OE, S, C = (), State = Disconnected>
where
    S: ObservableTypes,
{
    source: OE,
    #[educe(Debug(ignore))]
    lifecycle: SubjectLifecyclePtr<S, C>,
    state: State,
}

impl<OE, S, C> ConnectableController<OE, S, C, Disconnected>
where
    S: ObservableTypes,
{
    /// Creates a disconnected controller that will multicast `source` through the subjects
    /// `subject_factory` creates from `context`;
    /// [`ObservableExt::multicast`](crate::observable::ObservableExt::multicast) is the fluent form.
    ///
    /// Both [`reset_on_error`](Self::reset_on_error) and
    /// [`reset_on_complete`](Self::reset_on_complete) start out `true`.
    pub fn new(source: OE, context: C, subject_factory: fn(&mut C) -> S) -> Self {
        Self {
            source,
            lifecycle: S::Mode::ptr(SubjectLifecycle {
                context,
                subject_factory,
                reset_on_error: true,
                reset_on_complete: true,
                generations: IdGenerator::default(),
                current: None,
            }),
            state: Disconnected,
        }
    }

    /// Whether the subject is reset when the source fails: `true`, the default, makes the next
    /// subscriber wait for a new connection; `false` keeps the failed subject, so that late
    /// subscribers receive what it replays and the error, until the controller is disconnected.
    pub fn reset_on_error(self, reset: bool) -> Self {
        self.lifecycle
            .with_mut(|lifecycle| lifecycle.reset_on_error = reset);
        self
    }

    /// Whether the subject is reset when the source completes: `true`, the default, makes the next
    /// subscriber wait for a new connection; `false` keeps the completed subject, so that late
    /// subscribers receive what it replays and the completion, until the controller is
    /// disconnected.
    pub fn reset_on_complete(self, reset: bool) -> Self {
        self.lifecycle
            .with_mut(|lifecycle| lifecycle.reset_on_complete = reset);
        self
    }

    /// Connects to the source. The returned controller owns the connection and
    /// disconnects it when dropped.
    ///
    /// Ignoring the returned controller would disconnect at the end of the
    /// statement, so the compiler warns about it:
    ///
    /// ```compile_fail
    /// #![deny(unused_must_use)]
    /// use rx_rust::{
    ///     observable::ObservableExt,
    ///     operators::creating::from_iter::FromIter,
    /// };
    ///
    /// FromIter::new([1_i32]).publish().connect();
    /// ```
    ///
    /// Keep it for as long as the connection should last:
    ///
    /// ```rust
    /// #![deny(unused_must_use)]
    /// use rx_rust::{
    ///     observable::ObservableExt,
    ///     operators::creating::from_iter::FromIter,
    /// };
    ///
    /// let _controller = FromIter::new([1_i32]).publish().connect();
    /// ```
    #[must_use = "the returned controller owns the source connection"]
    pub fn connect<T, E>(self) -> ConnectableController<OE, S, C, Connected<OE::Disposal>>
    where
        OE: Observable<ConnectionObserver<S, C>, Item = T, Error = E> + Clone,
        S: Subject<T, E> + Clone,
    {
        self.connect_or_else(drop)
    }

    /// Connects to the source, or hands the controller to `on_panic`, still disconnected, if
    /// connecting unwinds: the subject factory, cloning the source and subscribing to it all run
    /// the user's code.
    pub(super) fn connect_or_else<T, E>(
        self,
        on_panic: impl FnOnce(Self),
    ) -> ConnectableController<OE, S, C, Connected<OE::Disposal>>
    where
        OE: Observable<ConnectionObserver<S, C>, Item = T, Error = E> + Clone,
        S: Subject<T, E> + Clone,
    {
        // The source is cloned and subscribed while the guard keeps the controller whole.
        let guard = OnPanic::new(self, on_panic);
        let controller = guard.state();
        let Current {
            subject,
            generation,
            ..
        } = controller
            .lifecycle
            .with_mut(SubjectLifecycle::current_or_create);
        let observer = ConnectionObserver {
            subject,
            lifecycle: S::Mode::downgrade(&controller.lifecycle),
            generation,
        };
        let connection = controller.source.clone().subscribe(observer);
        let Self {
            source, lifecycle, ..
        } = guard.disarm();
        ConnectableController {
            source,
            lifecycle,
            state: Connected {
                connection,
                generation,
            },
        }
    }

    /// Connects while there is at least one subscriber, and disconnects when the last one leaves.
    pub fn ref_count(self) -> RefCount<OE, S, C>
    where
        OE: ObservableTypes,
    {
        RefCount::new(self)
    }

    /// Whether `generation` is the current one and its source has not terminated.
    pub(super) fn is_live(&self, generation: Id) -> bool {
        self.lifecycle
            .with_ref(|lifecycle| lifecycle.is_live(generation))
    }
}

impl<OE, S, C, D> ConnectableController<OE, S, C, Connected<D>>
where
    S: ObservableTypes,
    D: Disposable,
{
    /// Disconnects the source and returns the controller in its disconnected state.
    ///
    /// The subject of this connection is reset, even one kept after its source terminated, so the
    /// next subscriber joins a new subject; one that was subscribed before receives nothing more.
    pub fn disconnect(self) -> ConnectableController<OE, S, C, Disconnected> {
        let generation = self.state.generation;
        drop(
            self.lifecycle
                .with_mut(|lifecycle| lifecycle.reset(generation)),
        );
        let (controller, connection) = self.take_connection();
        drop(connection);
        controller
    }

    /// Takes the connection out, leaving the subject to the [`RefCount`], which resets it itself.
    /// Dropping the connection disconnects the source, which runs the user's code: the caller drops
    /// it once it has made sure the controller survives that unwinding.
    pub(super) fn take_connection(
        self,
    ) -> (
        ConnectableController<OE, S, C, Disconnected>,
        DisposeOnDrop<D>,
    ) {
        let Self {
            source,
            lifecycle,
            state: Connected { connection, .. },
        } = self;
        let controller = ConnectableController {
            source,
            lifecycle,
            state: Disconnected,
        };
        (controller, connection)
    }

    /// The generation of the subject the source is connected to.
    pub(super) fn generation(&self) -> Id {
        self.state.generation
    }

    /// Resets the subject of this connection if it is still the current one and its source has not
    /// terminated. The replaced subject is returned, to be dropped outside the caller's lock.
    pub(super) fn reset_live(&self) -> Option<S> {
        let generation = self.state.generation;
        self.lifecycle
            .with_mut(|lifecycle| lifecycle.reset_live(generation))
    }
}

impl<OE, S, C, State> ConnectableController<OE, S, C, State>
where
    S: ObservableTypes,
{
    /// The observable side of the subject, for subscribers: each subscription joins the subject of
    /// the current connection.
    pub fn observable(&self) -> MulticastObservable<S, C> {
        MulticastObservable {
            lifecycle: self.lifecycle.clone(),
        }
    }
}

/// The observable side of a [`ConnectableController`], which
/// [`observable`](ConnectableController::observable) hands out: each subscription joins the subject
/// of the controller's current connection.
#[derive(Educe)]
#[educe(Debug, Clone(bound()))]
pub struct MulticastObservable<S, C = ()>
where
    S: ObservableTypes,
{
    #[educe(Debug(ignore))]
    lifecycle: SubjectLifecyclePtr<S, C>,
}

impl<S, C> ObservableTypes for MulticastObservable<S, C>
where
    S: ObservableTypes + Clone,
{
    type Item = S::Item;
    type Error = S::Error;
    type Mode = S::Mode;
    type Disposal = S::Disposal;
}

impl<S, C, OR> Observable<OR> for MulticastObservable<S, C>
where
    OR: Observer<S::Item, S::Error>,
    S: Observable<OR> + Clone,
{
    fn subscribe(self, observer: OR) -> DisposeOnDrop<Self::Disposal> {
        self.join().subject.subscribe(observer)
    }
}

impl<S, C> MulticastObservable<S, C>
where
    S: ObservableTypes,
{
    /// The subject a subscriber joins — the current one, created by the factory if there is none
    /// yet — with its generation, and whether its source has terminated.
    pub(super) fn join(&self) -> Current<S>
    where
        S: Clone,
    {
        self.lifecycle.with_mut(SubjectLifecycle::current_or_create)
    }
}

/// The observer a [`ConnectableController`] subscribes its source with: it forwards to the subject
/// of its connection, and resets that subject when the source terminates.
#[doc(hidden)]
pub struct ConnectionObserver<S, C>
where
    S: ObservableTypes,
{
    subject: S,
    /// Weak, so that a source holding on to this observer does not keep the controller's state,
    /// and the context in it, alive once the controller and its observables are gone.
    lifecycle: <<S as ObservableTypes>::Mode as ThreadMode>::Weak<SubjectLifecycle<S, C>>,
    generation: Id,
}

impl<T, E, S, C> Observer<T, E> for ConnectionObserver<S, C>
where
    S: Subject<T, E>,
{
    fn on_next(&mut self, value: T) -> Flow {
        self.subject.on_next(value)
    }

    fn on_termination(self, termination: Termination<E>) {
        // The subject is reset before it is terminated, so that a subscriber which re-subscribes
        // from its termination joins the next subject rather than this one.
        let replaced = S::Mode::upgrade(&self.lifecycle).and_then(|lifecycle| {
            lifecycle.with_mut(|lifecycle| lifecycle.terminate(self.generation, &termination))
        });
        drop(replaced);
        self.subject.on_termination(termination);
    }
}
