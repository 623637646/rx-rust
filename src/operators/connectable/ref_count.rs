use super::connectable_observable::ConnectableObservable;
use crate::delegate_disposal;
use crate::disposable::{Disposable, chain_disposal::ChainDisposal};
use crate::observable::{Observable, Subscription};
use crate::observer::Observer;
use crate::operators::connectable::connectable_observable::ConnectableObservableDisposable;
use crate::utils::types::{MaybeSend, Mutable, MutableHelper, Shared};
use educe::Educe;

#[derive(Educe)]
#[educe(Debug)]
enum State<D>
where
    D: Disposable,
{
    Initialized,
    Subscribed(
        usize,
        Option<Subscription<ChainDisposal<ConnectableObservableDisposable, D>>>,
    ),
}

/// Makes a `ConnectableObservable` behave like an ordinary `Observable` that automatically connects and disconnects.
/// See <https://reactivex.io/documentation/operators/refcount.html>
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt,
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
/// let observable = connectable.ref_count();
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
    source: ConnectableObservable<OE, S>,
    state: Shared<Mutable<State<OE::D>>>,
}

impl<'or, T, E, OE, S> RefCount<'or, T, E, OE, S>
where
    OE: Observable<'or, T, E>,
{
    pub fn new(source: ConnectableObservable<OE, S>) -> Self {
        Self {
            source,
            state: Shared::new(Mutable::new(State::Initialized)),
        }
    }
}

delegate_disposal!(
    Disposal<SD, OED>,
    ChainDisposal<SD, RefCountDisposal<OED>>,
    where SD: Disposable,
        OED: Disposable
);

impl<'or, T, E, OE, S> Observable<'or, T, E> for RefCount<'or, T, E, OE, S>
where
    OE: Observable<'or, T, E> + Clone,
    S: Observable<'or, T, E> + Observer<T, E> + Clone + MaybeSend + 'or,
{
    type D = Disposal<S::D, OE::D>;

    fn subscribe(self, observer: impl Observer<T, E> + MaybeSend + 'or) -> Subscription<Self::D> {
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
                .connect() // TODO: if panics, should we dispose of the subscription?
                .expect("ConnectableObservable should not be connected.");
            self.state.lock_mut(|mut lock| match &mut *lock {
                State::Initialized => unreachable!(),
                State::Subscribed(_, subscription) => {
                    assert!(subscription.replace(connect_sub).is_none())
                }
            });
        }
        sub.then(RefCountDisposal(self.state)).map_into()
    }
}

struct RefCountDisposal<D>(Shared<Mutable<State<D>>>)
where
    D: Disposable;

impl<D> Disposable for RefCountDisposal<D>
where
    D: Disposable,
{
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
