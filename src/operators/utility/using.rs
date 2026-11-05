//! The [`Using`] source.

use crate::{
    delegate_disposal,
    disposable::{
        Disposable, DisposableExt, dispose_on_drop::DisposeOnDrop, option_disposal::OptionDisposal,
    },
    observable::{Observable, ObservableTypes},
    observer::{Flow, Observer, Termination},
};
use educe::Educe;

/// Creates a resource for each subscription, builds the observable to subscribe to from it, and
/// drops the resource when that subscription ends: before the termination is delivered, when the
/// subscription is disposed, or when the observable lets go of its observer without a termination.
/// See <https://reactivex.io/documentation/operators/using.html>
///
/// `resource_factory` is called once a subscription starts. An `Err` from it ends that
/// subscription with the error, and `observable_factory` is not called. Otherwise
/// `observable_factory` gets the resource by `&mut` and returns the observable to subscribe to.
/// That observable cannot borrow the resource, which moves into the observer it is subscribed
/// with once it is built: it takes what it needs out of the resource instead — a handle, a path,
/// a cloned sender.
///
/// The resource lives in the observer given to that observable, so it is dropped wherever and
/// whenever the observable drops that observer: at once on a disposal for most observables, when
/// its scheduler drops its task for one whose task owns the observer (`observe_on`).
///
/// The resource is dropped *before* the termination goes downstream. An operator that subscribes
/// again from that termination — [`retry`](crate::observable::ObservableExt::retry) on an error
/// that arrives after the subscription was made — then creates the next resource only once the
/// previous one is gone, which an exclusive resource such as a file lock or a bound port needs; and
/// whatever downstream does on the termination sees the resource released.
/// [`hold`](crate::observable::ObservableExt::hold) attaches a value that already exists instead
/// of creating one, and drops it *after* the termination, which is what an acknowledgement such as
/// a `StreamAck` needs.
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt,
///     observer::{Observer, Termination},
///     operators::utility::using::Using,
///     subject::publish_subject::PublishSubject,
/// };
/// use std::{cell::RefCell, convert::Infallible, rc::Rc};
///
/// type Events = Rc<RefCell<Vec<&'static str>>>;
///
/// // A resource that records when it is closed.
/// struct Connection(Events);
/// impl Drop for Connection {
///     fn drop(&mut self) {
///         self.0.borrow_mut().push("closed");
///     }
/// }
///
/// let events = Events::default();
/// let mut subject = PublishSubject::<_, Infallible, _>::local();
/// let observable = Using::new(
///     {
///         let events = events.clone();
///         move || {
///             events.borrow_mut().push("opened");
///             Ok(Connection(events))
///         }
///     },
///     {
///         let subject = subject.clone();
///         move |_connection| subject
///     },
/// );
/// assert!(events.borrow().is_empty()); // Nothing is opened before a subscription.
///
/// let _subscription = observable.subscribe_with_callback(|_: i32| {}, {
///     let events = events.clone();
///     move |_| events.borrow_mut().push("completed")
/// });
/// assert_eq!(*events.borrow(), ["opened"]);
///
/// subject.on_termination(Termination::Completed);
/// assert_eq!(*events.borrow(), ["opened", "closed", "completed"]);
/// ```
///
/// A resource that cannot be created ends the subscription with its error:
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt,
///     observer::Termination,
///     operators::{creating::just::Just, utility::using::Using},
/// };
///
/// struct Connection;
///
/// fn connect() -> Result<Connection, &'static str> {
///     Err("refused")
/// }
///
/// let observable = Using::new(connect, |_| Just::new(1).with_error_type());
/// observable.subscribe_with_callback(
///     |_| -> () { unreachable!() },
///     |termination| assert_eq!(termination, Termination::Error("refused")),
/// );
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct Using<FR, FO> {
    resource_factory: FR,
    observable_factory: FO,
}

impl<FR, FO> Using<FR, FO> {
    /// Creates a [`Using`].
    pub fn new<R, E, OE>(resource_factory: FR, observable_factory: FO) -> Self
    where
        FR: FnOnce() -> Result<R, E>,
        FO: FnOnce(&mut R) -> OE,
        OE: ObservableTypes<Error = E>,
    {
        Self {
            resource_factory,
            observable_factory,
        }
    }
}

delegate_disposal!(
    Disposal<D>,
    OptionDisposal<D>,
    where D: Disposable
);

impl<T, E, R, OE, FR, FO> ObservableTypes for Using<FR, FO>
where
    FR: FnOnce() -> Result<R, E>,
    FO: FnOnce(&mut R) -> OE,
    OE: ObservableTypes<Item = T, Error = E>,
{
    type Item = T;
    type Error = E;
    type Mode = OE::Mode;
    type Disposal = Disposal<OE::Disposal>;
}

impl<T, E, R, OE, FR, FO, OR> Observable<OR> for Using<FR, FO>
where
    OR: Observer<T, E>,
    FR: FnOnce() -> Result<R, E>,
    FO: FnOnce(&mut R) -> OE,
    OE: Observable<UsingObserver<OR, R>, Item = T, Error = E>,
{
    fn subscribe(self, observer: OR) -> DisposeOnDrop<Self::Disposal> {
        match (self.resource_factory)() {
            Ok(mut resource) => {
                let observable = (self.observable_factory)(&mut resource);
                observable
                    .subscribe(UsingObserver { observer, resource })
                    .map_inner(OptionDisposal::some)
                    .map_inner_into()
            }
            Err(error) => {
                observer.on_termination(Termination::Error(error));
                OptionDisposal::none().into_dispose_on_drop()
            }
        }
    }
}

#[doc(hidden)]
pub struct UsingObserver<OR, R> {
    observer: OR,
    resource: R,
}

impl<T, E, OR, R> Observer<T, E> for UsingObserver<OR, R>
where
    OR: Observer<T, E>,
{
    fn on_next(&mut self, value: T) -> Flow {
        self.observer.on_next(value)
    }

    fn on_termination(self, termination: Termination<E>) {
        let Self { observer, resource } = self;
        // Released before the termination goes downstream, so that whatever the termination
        // starts — a new subscription, and with it a new resource — finds this one gone.
        drop(resource);
        observer.on_termination(termination)
    }
}
