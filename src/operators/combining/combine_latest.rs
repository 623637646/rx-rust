use crate::safe_lock_option_observer;
use crate::utils::types::{Mutable, MutableHelper, NecessarySend, Shared};
use crate::{
    disposable::subscription::Subscription,
    observable::Observable,
    observer::{Observer, Termination},
    utils::unsub_after_termination::subscribe_unsub_after_termination,
};
use educe::Educe;

/// Combines multiple Observables to create an Observable whose values are calculated from the latest values of each of its input Observables.
/// See <https://reactivex.io/documentation/operators/combinelatest.html>
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::observable_ext::ObservableExt,
///     observer::{Observer, Termination},
///     operators::combining::combine_latest::CombineLatest,
///     subject::behavior_subject::BehaviorSubject,
/// };
/// use std::convert::Infallible;
///
/// let mut values = Vec::new();
/// let mut terminations = Vec::new();
///
/// let mut subject_1 = BehaviorSubject::<'_, i32, Infallible>::new(0);
/// let mut subject_2 = BehaviorSubject::<'_, i32, Infallible>::new(10);
///
/// let subscription =
///     CombineLatest::new(subject_1.clone(), subject_2.clone()).subscribe_with_callback(
///         |value| values.push(value),
///         |termination| terminations.push(termination),
///     );
///
/// subject_1.on_next(1);
/// subject_2.on_next(11);
/// subject_1.on_termination(Termination::Completed);
/// subject_2.on_termination(Termination::Completed);
/// drop(subscription);
///
/// assert_eq!(values, vec![(0, 10), (1, 10), (1, 11)]);
/// assert_eq!(terminations, vec![Termination::Completed]);
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct CombineLatest<OE1, OE2> {
    source_1: OE1,
    source_2: OE2,
}

impl<OE1, OE2> CombineLatest<OE1, OE2> {
    pub fn new<'or, 'sub, T1, T2, E>(source_1: OE1, source_2: OE2) -> Self
    where
        OE1: Observable<'or, 'sub, T1, E>,
        OE2: Observable<'or, 'sub, T2, E>,
    {
        Self { source_1, source_2 }
    }
}

impl<'or, 'sub, T1, T2, E, OE1, OE2> Observable<'or, 'sub, (T1, T2), E> for CombineLatest<OE1, OE2>
where
    T1: Clone + NecessarySend + 'or,
    T2: Clone + NecessarySend + 'or,
    OE1: Observable<'or, 'sub, T1, E>,
    OE2: Observable<'or, 'sub, T2, E>,
    'sub: 'or,
{
    fn subscribe(
        self,
        observer: impl Observer<(T1, T2), E> + NecessarySend + 'or,
    ) -> Subscription<'sub> {
        subscribe_unsub_after_termination(observer, |observer| {
            let observer = Shared::new(Mutable::new(Some(observer)));
            let context = Shared::new(Mutable::new(CombineLatestContext {
                latest_1: None,
                latest_2: None,
                should_completed: false,
            }));
            let observer_1 = CombineLatestObserver1 {
                context: context.clone(),
                observer: observer.clone(),
            };
            let observer_2 = CombineLatestObserver2 { context, observer };
            let subscription_1 = self.source_1.subscribe(observer_1);
            let subscription_2 = self.source_2.subscribe(observer_2);
            subscription_1 + subscription_2
        })
    }
}

struct CombineLatestContext<T1, T2> {
    latest_1: Option<T1>,
    latest_2: Option<T2>,
    should_completed: bool,
}

struct CombineLatestObserver1<T1, T2, OR> {
    context: Shared<Mutable<CombineLatestContext<T1, T2>>>,
    observer: Shared<Mutable<Option<OR>>>,
}

impl<T1, T2, E, OR> Observer<T1, E> for CombineLatestObserver1<T1, T2, OR>
where
    T1: Clone,
    T2: Clone,
    OR: Observer<(T1, T2), E>,
{
    fn on_next(&mut self, latest_1: T1) {
        self.context.lock_mut(|mut lock| {
            lock.latest_1 = Some(latest_1.clone());
            if let Some(latest_2) = lock.latest_2.clone() {
                drop(lock);
                safe_lock_option_observer!(on_next: self.observer, (latest_1, latest_2));
            }
        });
    }

    fn on_termination(self, termination: Termination<E>) {
        match termination {
            Termination::Completed => {
                self.context.lock_mut(|mut lock| {
                    if lock.should_completed || lock.latest_1.is_none() {
                        drop(lock);
                        safe_lock_option_observer!(on_termination: self.observer, termination);
                    } else {
                        lock.should_completed = true;
                    }
                });
            }
            Termination::Error(_) => {
                safe_lock_option_observer!(on_termination: self.observer, termination);
            }
        }
    }
}

struct CombineLatestObserver2<T1, T2, OR> {
    context: Shared<Mutable<CombineLatestContext<T1, T2>>>,
    observer: Shared<Mutable<Option<OR>>>,
}

impl<T1, T2, E, OR> Observer<T2, E> for CombineLatestObserver2<T1, T2, OR>
where
    T1: Clone,
    T2: Clone,
    OR: Observer<(T1, T2), E>,
{
    fn on_next(&mut self, latest_2: T2) {
        self.context.lock_mut(|mut lock| {
            lock.latest_2 = Some(latest_2.clone());
            if let Some(latest_1) = lock.latest_1.clone() {
                drop(lock);
                safe_lock_option_observer!(on_next: self.observer, (latest_1, latest_2));
            }
        });
    }

    fn on_termination(self, termination: Termination<E>) {
        match termination {
            Termination::Completed => {
                self.context.lock_mut(|mut lock| {
                    if lock.should_completed || lock.latest_2.is_none() {
                        drop(lock);
                        safe_lock_option_observer!(on_termination: self.observer, termination);
                    } else {
                        lock.should_completed = true;
                    }
                });
            }
            Termination::Error(_) => {
                safe_lock_option_observer!(on_termination: self.observer, termination);
            }
        }
    }
}
