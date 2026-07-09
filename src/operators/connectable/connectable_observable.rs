use super::ref_count::RefCount;
use crate::disposable::{Disposable, chain_disposal::ChainDisposal};
use crate::observable::{Observable, Subscription};
use crate::observer::Observer;
use crate::utils::types::{MaybeSend, MutableBool, MutableBoolHelper, Shared};
use educe::Educe;

/// Represents an Observable that waits until its `connect()` method is called before it begins emitting items to its Observers.
/// See <https://reactivex.io/documentation/operators/connect.html>
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt,
///     observer::Termination,
///     operators::{
///         connectable::connectable_observable::ConnectableObservable,
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
/// let connectable =
///     ConnectableObservable::new(FromIter::new(vec![1, 2]), subject.clone());
/// let values_1_observer = Arc::clone(&values_1);
/// let values_2_observer = Arc::clone(&values_2);
/// let terminations_observer = Arc::clone(&terminations);
///
/// let subscription_1 = connectable.clone().subscribe_with_callback(
///     move |value| values_1_observer.lock().unwrap().push(value),
///     |_| {},
/// );
/// let subscription_2 = connectable.clone().subscribe_with_callback(
///     move |value| values_2_observer.lock().unwrap().push(value),
///     move |termination| terminations_observer
///         .lock()
///         .unwrap()
///         .push(termination),
/// );
///
/// let connection = connectable.connect();
/// drop(connection);
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
#[educe(Debug, Clone)]
pub struct ConnectableObservable<OE, S> {
    source: OE,
    subject: S,
    is_connected: Shared<MutableBool>,
}

impl<OE, S> ConnectableObservable<OE, S> {
    pub fn new(source: OE, subject: S) -> Self {
        Self {
            source,
            subject,
            is_connected: Shared::new(MutableBool::new(false)),
        }
    }

    // Subscribe the source. Return None if already connected
    pub fn connect<'or, T, E>(
        self,
    ) -> Option<Subscription<ChainDisposal<ConnectableObservableDisposable, OE::D>>>
    where
        OE: Observable<'or, T, E>,
        S: Observer<T, E> + MaybeSend + 'or,
    {
        if self.is_connected.change_if_not_equal(true) {
            Some(
                self.source
                    .subscribe(self.subject)
                    .preceded_by(ConnectableObservableDisposable(self.is_connected)),
            )
        } else {
            None
        }
    }

    pub fn ref_count<'or, T, E>(self) -> RefCount<'or, T, E, OE, S>
    where
        OE: Observable<'or, T, E>,
    {
        RefCount::new(self)
    }
}

impl<'or, T, E, OE, S> Observable<'or, T, E> for ConnectableObservable<OE, S>
where
    S: Observable<'or, T, E>,
{
    type D = S::D;

    fn subscribe(self, observer: impl Observer<T, E> + MaybeSend + 'or) -> Subscription<Self::D> {
        self.subject.subscribe(observer)
    }
}

pub struct ConnectableObservableDisposable(Shared<MutableBool>);

impl Disposable for ConnectableObservableDisposable {
    fn dispose(self) {
        self.0.write(false);
    }
}
