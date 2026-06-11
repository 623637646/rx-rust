use super::ref_count::RefCount;
use crate::observable::Observable;
use crate::utils::types::{NecessarySendSync, Shared};
use crate::{disposable::subscription::Subscription, observer::Observer};
use educe::Educe;
use std::sync::atomic::{AtomicBool, Ordering};

/// Represents an Observable that waits until its `connect()` method is called before it begins emitting items to its Observers.
/// See <https://reactivex.io/documentation/operators/connect.html>
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::observable_ext::ObservableExt,
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
    is_connected: Shared<AtomicBool>,
}

impl<OE, S> ConnectableObservable<OE, S> {
    pub fn new(source: OE, subject: S) -> Self {
        Self {
            source,
            subject,
            is_connected: Shared::new(AtomicBool::new(false)),
        }
    }

    // Subscribe the source. Return None if already connected
    pub fn connect<'or, 'sub, T, E>(self) -> Option<Subscription<'sub>>
    where
        OE: Observable<'or, 'sub, T, E>,
        S: Observer<T, E> + NecessarySendSync + 'or,
    {
        if self
            .is_connected
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
        {
            Some(self.source.subscribe(self.subject) + self.is_connected)
        } else {
            None
        }
    }

    pub fn ref_count<'sub>(self) -> RefCount<'sub, OE, S> {
        RefCount::new(self)
    }
}

impl<'or, 'sub, T, E, OE, S> Observable<'or, 'sub, T, E> for ConnectableObservable<OE, S>
where
    S: Observable<'or, 'sub, T, E>,
{
    fn subscribe(
        self,
        observer: impl Observer<T, E> + NecessarySendSync + 'or,
    ) -> Subscription<'sub> {
        self.subject.subscribe(observer)
    }
}
