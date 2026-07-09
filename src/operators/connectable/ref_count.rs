use super::connectable_observable::ConnectableObservable;
use crate::disposable::Disposable;
use crate::disposable::subscription::Subscription;
use crate::observable::Observable;
use crate::observer::Observer;
use crate::utils::types::{MaybeSend, Mutable, MutableHelper, Shared};
use educe::Educe;

#[derive(Educe)]
#[educe(Debug)]
enum State<'sub> {
    Initialized,
    Subscribed(usize, Option<Subscription<'sub>>),
}

/// Makes a `ConnectableObservable` behave like an ordinary `Observable` that automatically connects and disconnects.
/// See <https://reactivex.io/documentation/operators/refcount.html>
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::observable_ext::ObservableExt,
///     observer::Termination,
///     operators::{
///         connectable::{connectable_observable::ConnectableObservable, ref_count::RefCount},
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
/// let connectable =
///     ConnectableObservable::new(FromIter::new(vec![1, 2]), subject.clone());
/// let observable: RefCount<'_, _, _> = connectable.ref_count();
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
pub struct RefCount<'sub, OE, S> {
    source: ConnectableObservable<OE, S>,
    state: Shared<Mutable<State<'sub>>>,
}

impl<OE, S> RefCount<'_, OE, S> {
    pub fn new(source: ConnectableObservable<OE, S>) -> Self {
        Self {
            source,
            state: Shared::new(Mutable::new(State::Initialized)),
        }
    }
}

impl<'or, 'sub, T, E, OE, S> Observable<'or, 'sub, T, E> for RefCount<'sub, OE, S>
where
    OE: Observable<'or, 'sub, T, E> + Clone,
    S: Observable<'or, 'sub, T, E> + Observer<T, E> + Clone + MaybeSend + 'or,
{
    fn subscribe(self, observer: impl Observer<T, E> + MaybeSend + 'or) -> Subscription<'sub> {
        let sub = self.source.clone().subscribe(observer);
        let should_connect = self.state.lock_mut(|mut lock| match &mut *lock {
            State::Initialized => {
                *lock = State::Subscribed(1, None);
                true
            }
            State::Subscribed(count, _) => {
                *count += 1;
                false
            }
        });
        if should_connect {
            let connect_sub = self
                .source
                .connect()
                .expect("ConnectableObservable should not be connected.");
            self.state.lock_mut(|mut lock| match &mut *lock {
                State::Initialized => unreachable!(),
                State::Subscribed(_, subscription) => {
                    assert!(subscription.replace(connect_sub).is_none())
                }
            });
        }
        sub + RefCountDisposal(self.state)
    }
}

struct RefCountDisposal<'sub>(Shared<Mutable<State<'sub>>>);

impl Disposable for RefCountDisposal<'_> {
    fn dispose(self) {
        self.0.lock_mut(|mut lock| match &mut *lock {
            State::Initialized => unreachable!(),
            State::Subscribed(count, _) => {
                *count -= 1;
                if *count == 0 {
                    let state = std::mem::replace(&mut *lock, State::Initialized);
                    drop(lock);
                    match state {
                        State::Initialized => unreachable!(),
                        State::Subscribed(_, subscription) => {
                            subscription.unwrap().dispose();
                        }
                    }
                }
            }
        });
    }
}
