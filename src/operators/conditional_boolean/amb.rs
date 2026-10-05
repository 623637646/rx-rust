//! The [`Amb`] operator, behind
//! [`ObservableExt::amb_with`](crate::observable::ObservableExt::amb_with).

use crate::disposable::Disposable;
use crate::thread_mode::ThreadMode;
use crate::thread_mode::mutable::{MutableExt, MutableHelper};
use crate::{
    observable::{Observable, ObservableTypes, Subscription},
    observer::{Flow, Observer, Termination},
};
use educe::Educe;

/// Given two or more source Observables, mirrors only the first of them to emit an item or a
/// termination.
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
    /// Creates an [`Amb`] racing every observable of `sources`;
    /// [`ObservableExt::amb_with`](crate::observable::ObservableExt::amb_with) is the two-source
    /// form.
    pub fn new(sources: I) -> Self {
        Self { sources }
    }
}

impl<T, E, OE, I> ObservableTypes for Amb<I>
where
    I: IntoIterator<Item = OE>,
    OE: ObservableTypes<Item = T, Error = E>,
{
    type Item = T;
    type Error = E;
    type Mode = OE::Mode;
    type D = Disposal<OE::Mode, OE::D>;
}

impl<T, E, OE, I, OR> Observable<OR> for Amb<I>
where
    OR: Observer<T, E>,
    I: IntoIterator<Item = OE>,
    OE: Observable<
            AmbObserver<<OE as ObservableTypes>::Mode, <OE as ObservableTypes>::D, OR>,
            Item = T,
            Error = E,
        >,
{
    fn subscribe(self, observer: OR) -> Subscription<Self::D> {
        let sources = self.sources.into_iter();
        let minimum_source_count = sources.size_hint().0;
        let observer = OE::Mode::ptr(Some(observer));
        let context = OE::Mode::ptr(AmbState::Racing(Vec::with_capacity(minimum_source_count)));

        let mut has_sources = false;
        for source in sources {
            has_sources = true;
            let Some(key) = reserve_subscription_slot(&context) else {
                break;
            };
            let amb_observer = AmbObserver(AmbObserverState::Racing {
                observer: observer.clone(),
                context: OE::Mode::downgrade(&context),
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
                .take_value()
                .expect("a new amb must retain its downstream observer");
            observer.on_termination(Termination::Completed);
        }

        Subscription::new(Disposal(context))
    }
}

enum AmbState<D: Disposable> {
    Racing(Vec<Option<Subscription<D>>>),
    Won {
        key: usize,
        subscription: Option<Subscription<D>>,
    },
    Stopped,
}

fn reserve_subscription_slot<D, P>(context: &P) -> Option<usize>
where
    D: Disposable,
    P: MutableHelper<Value = AmbState<D>>,
{
    context.with_mut(|state| match state {
        AmbState::Racing(subscriptions) => {
            let key = subscriptions.len();
            subscriptions.push(None);
            Some(key)
        }
        AmbState::Won { .. } | AmbState::Stopped => None,
    })
}

fn store_subscription<D, P>(context: &P, key: usize, subscription: Subscription<D>) -> bool
where
    D: Disposable,
    P: MutableHelper<Value = AmbState<D>>,
{
    // Wrapped in an `Option` so that the branches which do not store the subscription leave it to
    // be dropped outside the lock.
    let mut subscription = Some(subscription);
    let (keep_subscribing, replaced) = context.with_mut(|state| match state {
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

fn try_win<D, OR, P, PO>(context: &P, shared_observer: &PO, key: usize) -> Option<OR>
where
    D: Disposable,
    P: MutableHelper<Value = AmbState<D>>,
    PO: MutableHelper<Value = Option<OR>>,
{
    let losing_subscriptions = context.with_mut(|state| {
        if !matches!(state, AmbState::Racing(_)) {
            return None;
        }

        let AmbState::Racing(mut subscriptions) = std::mem::replace(state, AmbState::Stopped)
        else {
            unreachable!()
        };
        let winner_subscription = subscriptions
            .get_mut(key)
            .expect("a racing source must retain its subscription slot")
            .take();
        *state = AmbState::Won {
            key,
            subscription: winner_subscription,
        };
        Some(subscriptions)
    })?;
    // The `Racing` -> `Won` transition above elects a single winner, so the downstream observer is
    // taken after the context lock is released: no other source can reach this point.
    let observer = shared_observer
        .take_value()
        .expect("a racing amb must retain its downstream observer");
    drop(losing_subscriptions); // Dispose losing subscriptions outside the lock.
    Some(observer)
}

enum AmbObserverState<M: ThreadMode, D: Disposable, OR> {
    Racing {
        observer: M::Ptr<Option<OR>>,
        context: M::Weak<AmbState<D>>,
        key: usize,
    },
    Won(OR),
    Lost,
}

pub struct AmbObserver<M: ThreadMode, D: Disposable, OR>(AmbObserverState<M, D, OR>);

impl<M, T, E, D, OR> Observer<T, E> for AmbObserver<M, D, OR>
where
    M: ThreadMode,
    D: Disposable,
    OR: Observer<T, E>,
{
    fn on_next(&mut self, value: T) -> Flow {
        match &mut self.0 {
            AmbObserverState::Won(observer) => {
                return observer.on_next(value);
            }
            // This source lost the race, so nothing it emits is wanted anymore.
            AmbObserverState::Lost => return Flow::Stop,
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
        let Some(shared_context) = M::upgrade(&context) else {
            return Flow::Stop;
        };
        let Some(mut observer) = try_win(&shared_context, &shared_observer, key) else {
            return Flow::Stop;
        };
        drop(shared_context);
        drop(shared_observer);

        let flow = observer.on_next(value);
        self.0 = AmbObserverState::Won(observer);
        flow
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
                let Some(shared_context) = M::upgrade(&context) else {
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

/// The disposal of an [`Amb`] subscription: it disposes every source still racing, or the winner.
pub struct Disposal<M: ThreadMode, D: Disposable>(M::Ptr<AmbState<D>>);

impl<M, D> Disposable for Disposal<M, D>
where
    M: ThreadMode,
    D: Disposable,
{
    fn dispose(self) {
        let old_state = self.0.replace_value(AmbState::Stopped);
        drop(old_state); // Dispose the remaining subscriptions outside the lock.
    }
}
