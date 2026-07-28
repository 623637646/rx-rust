use super::ref_count::RefCount;
use crate::disposable::Disposable;
use crate::observable::{Observable, Subscription};
use crate::observer::Observer;
use crate::subject::subject_observable::SubjectObservable;
use crate::utils::types::MaybeSend;
use educe::Educe;

/// Marker for a connectable controller that is not connected to its source.
#[derive(Debug, Clone, Copy, Default)]
pub struct Disconnected;

/// State carried by a connected controller. Dropping it disconnects the source.
#[derive(Educe)]
#[educe(Debug)]
pub struct Connected<D: Disposable>(Subscription<D>);

/// Multicasts a source `Observable` through a `Subject`, but waits until its [`connect`](ConnectableController::connect)
/// method is called before subscribing to the source and emitting items to its observers.
/// Subscribe to the multicast output through [`observable`](ConnectableController::observable).
/// See <https://reactivex.io/documentation/operators/connect.html>
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
///
/// use std::{convert::Infallible, sync::{Arc, Mutex}};
///
/// let values_1 = Arc::new(Mutex::new(Vec::new()));
/// let values_2 = Arc::new(Mutex::new(Vec::new()));
/// let terminations = Arc::new(Mutex::new(Vec::new()));
///
/// let subject: PublishSubject<'_, i32, Infallible> = PublishSubject::default();
/// let controller = ConnectableController::new(FromIter::new(vec![1, 2]), subject);
/// let observable = controller.observable();
/// let values_1_observer = Arc::clone(&values_1);
/// let values_2_observer = Arc::clone(&values_2);
/// let terminations_observer = Arc::clone(&terminations);
///
/// let subscription_1 = observable.clone().subscribe_with_callback(
///     move |value| values_1_observer.lock().unwrap().push(value),
///     |_| {},
/// );
/// let subscription_2 = observable.subscribe_with_callback(
///     move |value| values_2_observer.lock().unwrap().push(value),
///     move |termination| terminations_observer
///         .lock()
///         .unwrap()
///         .push(termination),
/// );
///
/// // Nothing is emitted until the source is connected.
/// let connected = controller.connect();
/// // Dropping the connected controller disconnects the source.
/// drop(connected);
/// drop(subscription_1);
/// drop(subscription_2);
///
/// assert_eq!(&*values_1.lock().unwrap(), &[1, 2]);
/// assert_eq!(&*values_2.lock().unwrap(), &[1, 2]);
/// assert_eq!(
///     &*terminations.lock().unwrap(),
///     &[Termination::Completed]
/// );
/// ```
#[derive(Educe)]
#[educe(Debug)]
pub struct ConnectableController<OE, S, State = Disconnected> {
    source: OE,
    subject: S,
    state: State,
}

impl<OE, S> ConnectableController<OE, S, Disconnected> {
    pub fn new(source: OE, subject: S) -> Self {
        Self {
            source,
            subject,
            state: Disconnected,
        }
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
    #[must_use = "the returned controller owns the source connection"]
    pub fn connect<'or, T, E>(self) -> ConnectableController<OE, S, Connected<OE::D>>
    where
        OE: Observable<'or, T, E> + Clone,
        S: Observer<T, E> + Clone + MaybeSend + 'or,
    {
        let sub = self.source.clone().subscribe(self.subject.clone());
        ConnectableController {
            source: self.source,
            subject: self.subject,
            state: Connected(sub),
        }
    }

    pub fn ref_count<'or, T, E>(self) -> RefCount<'or, T, E, OE, S>
    where
        OE: Observable<'or, T, E>,
        S: Clone,
    {
        RefCount::new(self)
    }
}

impl<OE, S, D> ConnectableController<OE, S, Connected<D>>
where
    D: Disposable,
{
    /// Disconnects the source and returns the controller in its disconnected state.
    pub fn disconnect(self) -> ConnectableController<OE, S, Disconnected> {
        let Self {
            source,
            subject,
            state,
        } = self;
        drop(state);
        ConnectableController {
            source,
            subject,
            state: Disconnected,
        }
    }
}

impl<OE, S, State> ConnectableController<OE, S, State> {
    pub fn observable(&self) -> SubjectObservable<S>
    where
        S: Clone,
    {
        SubjectObservable::new(self.subject.clone())
    }
}
