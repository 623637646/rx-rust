use crate::delegate_disposal;
use crate::disposable::{Disposable, DisposableExt};
use crate::utils::types::{MaybeSend, Mutable, MutableHelper, Shared, WeakShared};
use crate::{
    observable::{Observable, Subscription},
    observer::{Observer, Termination},
};
use educe::Educe;

/// Given two or more source Observables, emit all of the items from only the first of these Observables to emit an item or notification.
/// See <https://reactivex.io/documentation/operators/amb.html>
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt,
///     observer::Termination,
///     operators::{
///         conditional_boolean::amb::Amb,
///         creating::from_iter::FromIter,
///     },
/// };
///
/// let mut values = Vec::new();
/// let mut terminations = Vec::new();
///
/// let observable = Amb::new([
///     FromIter::new(vec![1, 2]),
///     FromIter::new(vec![3, 4]),
/// ]);
/// observable.subscribe_with_callback(
///     |value| values.push(value),
///     |termination| terminations.push(termination),
/// );
///
/// assert_eq!(values, vec![1, 2]);
/// assert_eq!(terminations, vec![Termination::Completed]);
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct Amb<I> {
    sources: I,
}

impl<I> Amb<I> {
    pub fn new(sources: I) -> Self {
        Self { sources }
    }
}

delegate_disposal!(
    Disposal<D>,
    Shared<Mutable<AmbContext<D>>>,
    where D: Disposable
);

impl<'or, T, E, OE, I> Observable<'or, T, E> for Amb<I>
where
    I: IntoIterator<Item = OE>,
    OE: Observable<'or, T, E>,
    OE::D: MaybeSend + 'or,
{
    type D = Disposal<OE::D>;

    fn subscribe(self, observer: impl Observer<T, E> + MaybeSend + 'or) -> Subscription<Self::D> {
        let sources = self.sources.into_iter();
        let minimum_source_count = sources.size_hint().0;
        let observer = Shared::new(Mutable::new(Some(observer)));
        let context = Shared::new(Mutable::new(AmbContext {
            state: AmbState::Racing(Vec::with_capacity(minimum_source_count)),
        }));

        let mut has_sources = false;
        for source in sources {
            has_sources = true;
            let Some(key) = reserve_subscription_slot(&context) else {
                break;
            };
            let amb_observer = AmbObserver(AmbObserverState::Racing {
                observer: observer.clone(),
                context: Shared::downgrade(&context),
                key,
            });
            let subscription = source.subscribe(amb_observer);
            if !store_subscription(&context, key, subscription) {
                break;
            }
        }

        if !has_sources {
            // Without a source, no one can ever win the race, so it completes right away.
            // The observer is taken out of its slot so that it is notified outside the lock.
            let observer = observer
                .lock_mut(|mut observer| observer.take())
                .expect("a new amb must retain its downstream observer");
            observer.on_termination(Termination::Completed);
        }

        context.into_subscription()
    }
}

struct AmbContext<D: Disposable> {
    state: AmbState<D>,
}

enum AmbState<D: Disposable> {
    Racing(Vec<Option<Subscription<D>>>),
    Won {
        key: usize,
        subscription: Option<Subscription<D>>,
    },
    Stopped,
}

// TODO: Disposable should not be Cloneable
impl<D> Disposable for Shared<Mutable<AmbContext<D>>>
where
    D: Disposable,
{
    fn dispose(self) {
        let old_state =
            self.lock_mut(|mut lock| std::mem::replace(&mut lock.state, AmbState::Stopped));
        drop(old_state); // Dispose the remaining subscriptions outside the lock.
    }
}

fn reserve_subscription_slot<D>(context: &Mutable<AmbContext<D>>) -> Option<usize>
where
    D: Disposable,
{
    context.lock_mut(|mut lock| match &mut lock.state {
        AmbState::Racing(subscriptions) => {
            let key = subscriptions.len();
            subscriptions.push(None);
            Some(key)
        }
        AmbState::Won { .. } | AmbState::Stopped => None,
    })
}

fn store_subscription<D>(
    context: &Mutable<AmbContext<D>>,
    key: usize,
    subscription: Subscription<D>,
) -> bool
where
    D: Disposable,
{
    // Wrapped in an `Option` so that the branches which do not store the subscription leave it to
    // be dropped outside the lock.
    let mut subscription = Some(subscription);
    let (keep_subscribing, replaced) = context.lock_mut(|mut lock| match &mut lock.state {
        // The race is still open: the subscription belongs in the reserved slot.
        AmbState::Racing(subscriptions) => {
            let slot = subscriptions
                .get_mut(key)
                .expect("a racing source must retain its subscription slot");
            let replaced = std::mem::replace(slot, subscription.take());
            (true, replaced)
        }
        // This source won while it was still being subscribed to: it owns the winning slot.
        AmbState::Won {
            key: winner_key,
            subscription: winner_subscription,
        } if *winner_key == key => {
            let replaced = std::mem::replace(winner_subscription, subscription.take());
            (false, replaced)
        }
        // Another source won, or the race is over: this subscription is not needed anymore.
        AmbState::Won { .. } | AmbState::Stopped => (false, None),
    });
    // Both slots are empty until they are written above, so nothing is ever replaced. This is
    // asserted outside the lock so that a failing assertion cannot poison it.
    debug_assert!(
        replaced.is_none(),
        "a subscription slot is written only once"
    );
    drop((replaced, subscription)); // Drop a late losing subscription outside the lock.
    keep_subscribing
}

fn try_win<D, OR>(
    context: &Mutable<AmbContext<D>>,
    shared_observer: &Mutable<Option<OR>>,
    key: usize,
) -> Option<OR>
where
    D: Disposable,
{
    let losing_subscriptions = context.lock_mut(|mut lock| {
        if !matches!(&lock.state, AmbState::Racing(_)) {
            return None;
        }

        let AmbState::Racing(mut subscriptions) =
            std::mem::replace(&mut lock.state, AmbState::Stopped)
        else {
            unreachable!()
        };
        let winner_subscription = subscriptions
            .get_mut(key)
            .expect("a racing source must retain its subscription slot")
            .take();
        lock.state = AmbState::Won {
            key,
            subscription: winner_subscription,
        };
        Some(subscriptions)
    })?;
    // The `Racing` -> `Won` transition above elects a single winner, so the downstream observer is
    // taken after the context lock is released: no other source can reach this point.
    let observer = shared_observer
        .lock_mut(|mut observer| observer.take())
        .expect("a racing amb must retain its downstream observer");
    drop(losing_subscriptions); // Dispose losing subscriptions outside the lock.
    Some(observer)
}

enum AmbObserverState<D: Disposable, OR> {
    Racing {
        observer: Shared<Mutable<Option<OR>>>,
        context: WeakShared<Mutable<AmbContext<D>>>,
        key: usize,
    },
    Won(OR),
    Lost,
}

struct AmbObserver<D: Disposable, OR>(AmbObserverState<D, OR>);

impl<T, E, D, OR> Observer<T, E> for AmbObserver<D, OR>
where
    D: Disposable,
    OR: Observer<T, E>,
{
    fn on_next(&mut self, value: T) {
        match &mut self.0 {
            AmbObserverState::Won(observer) => {
                observer.on_next(value);
                return;
            }
            AmbObserverState::Lost => return,
            AmbObserverState::Racing { .. } => {}
        }

        let AmbObserverState::Racing {
            observer: shared_observer,
            context,
            key,
        } = std::mem::replace(&mut self.0, AmbObserverState::Lost)
        else {
            unreachable!()
        };
        let Some(shared_context) = context.upgrade() else {
            return;
        };
        let Some(mut observer) = try_win(&shared_context, &shared_observer, key) else {
            return;
        };
        drop(shared_context);
        drop(shared_observer);

        observer.on_next(value);
        self.0 = AmbObserverState::Won(observer);
    }

    fn on_termination(self, termination: Termination<E>) {
        match self.0 {
            AmbObserverState::Won(observer) => observer.on_termination(termination),
            AmbObserverState::Lost => {}
            AmbObserverState::Racing {
                observer: shared_observer,
                context,
                key,
            } => {
                let Some(shared_context) = context.upgrade() else {
                    return;
                };
                let Some(observer) = try_win(&shared_context, &shared_observer, key) else {
                    return;
                };
                drop(shared_context);
                drop(shared_observer);
                observer.on_termination(termination);
            }
        }
    }
}
