use crate::disposable::Disposable;
use crate::disposable::subscription::Subscription;
use crate::utils::types::{MutGuard, Mutable, MutableHelper, NecessarySendSync, Shared};
use crate::{
    observable::Observable,
    observer::{Observer, Termination},
    operators::creating::from_iter::FromIter,
    utils::{types::MarkerType, unsub_after_termination::subscribe_unsub_after_termination},
};
use crate::{safe_lock_option, safe_lock_option_disposable, safe_lock_option_observer};
use educe::Educe;
use std::{
    collections::VecDeque,
    marker::PhantomData,
    sync::atomic::{AtomicBool, Ordering},
};

/// Concatenates an Observable of Observables, emitting all values from each inner Observable in sequence.
/// See <https://reactivex.io/documentation/operators/concat.html> (referencing concat operator for general concept)
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::observable_ext::ObservableExt,
///     observer::Termination,
///     operators::{
///         combining::concat_all::ConcatAll,
///         creating::from_iter::FromIter,
///     },
/// };
///
/// let mut values = Vec::new();
/// let mut terminations = Vec::new();
///
/// let observable = ConcatAll::new_from_iter([
///     FromIter::new(vec![1, 2]),
///     FromIter::new(vec![3, 4]),
/// ]);
/// observable.subscribe_with_callback(
///     |value| values.push(value),
///     |termination| terminations.push(termination),
/// );
///
/// assert_eq!(values, vec![1, 2, 3, 4]);
/// assert_eq!(terminations, vec![Termination::Completed]);
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct ConcatAll<OE, OE1> {
    source: OE,
    _marker: MarkerType<OE1>,
}

impl<OE, OE1> ConcatAll<OE, OE1> {
    pub fn new<'or, 'sub, T, E>(source: OE) -> Self
    where
        OE: Observable<'or, 'sub, OE1, E>,
        OE1: Observable<'or, 'sub, T, E>,
    {
        Self {
            source,
            _marker: PhantomData,
        }
    }
}

impl<OE1, I> ConcatAll<FromIter<I>, OE1> {
    pub fn new_from_iter<'or, 'sub, T, E>(into_iterator: I) -> Self
    where
        I: IntoIterator<Item = OE1>,
        OE1: Observable<'or, 'sub, T, E>,
    {
        Self {
            source: FromIter::new(into_iterator),
            _marker: PhantomData,
        }
    }
}

impl<'or, 'sub, T, E, OE, OE1> Observable<'or, 'sub, T, E> for ConcatAll<OE, OE1>
where
    T: 'or,
    OE: Observable<'or, 'sub, OE1, E>,
    OE1: Observable<'or, 'sub, T, E> + NecessarySendSync + 'sub,
    'sub: 'or,
{
    fn subscribe(self, observer: impl Observer<T, E> + NecessarySendSync + 'or) -> Subscription<'sub> {
        subscribe_unsub_after_termination(observer, |observer| {
            let context = Shared::new(Mutable::new(ConcatAllContext {
                pending_observables: VecDeque::new(),
                on_going_sub: None,
                completed: false,
            }));
            let observer = ConcatAllObserver {
                observer: Shared::new(Mutable::new(Some(observer))),
                context: context.clone(),
                _marker: PhantomData,
            };
            self.source.subscribe(observer) + context
        })
    }
}

struct ConcatAllContext<'sub, OE1> {
    pending_observables: VecDeque<OE1>,
    on_going_sub: Option<Subscription<'sub>>,
    completed: bool,
}

impl<OE1> Disposable for Shared<Mutable<ConcatAllContext<'_, OE1>>> {
    fn dispose(self) {
        safe_lock_option_disposable!(dispose: self, on_going_sub);
    }
}

fn subscribe_next<'or, 'sub, T, E, OR, OE1>(
    lock: Option<MutGuard<'_, ConcatAllContext<'sub, OE1>>>,
    context: Shared<Mutable<ConcatAllContext<'sub, OE1>>>,
    observer: Shared<Mutable<Option<OR>>>,
) where
    OR: Observer<T, E> + NecessarySendSync + 'or,
    OE1: Observable<'or, 'sub, T, E> + NecessarySendSync + 'or,
    'sub: 'or,
{
    let implementation = |mut lock: MutGuard<'_, ConcatAllContext<'sub, OE1>>| {
        if let Some(observable) = lock.pending_observables.pop_front() {
            drop(lock);
            let terminated = Shared::new(AtomicBool::new(false));
            let observer = ConcatAllInnerObserver {
                observer: observer.clone(),
                context: context.clone(),
                terminated: terminated.clone(),
            };
            let sub = observable.subscribe(observer);
            if !terminated.load(Ordering::SeqCst) {
                safe_lock_option!(replace: context, on_going_sub, sub);
            }
        } else if lock.completed {
            drop(lock);
            safe_lock_option_observer!(on_termination: observer, Termination::Completed);
        } else {
            lock.on_going_sub.take();
        }
    };
    if let Some(lock) = lock {
        implementation(lock);
    } else {
        context.lock_mut(implementation);
    }
}

struct ConcatAllObserver<'sub, T, OR, OE1> {
    observer: Shared<Mutable<Option<OR>>>,
    context: Shared<Mutable<ConcatAllContext<'sub, OE1>>>,
    _marker: MarkerType<T>,
}

impl<'or, 'sub, T, E, OR, OE1> Observer<OE1, E> for ConcatAllObserver<'sub, T, OR, OE1>
where
    OR: Observer<T, E> + NecessarySendSync + 'or,
    OE1: Observable<'or, 'sub, T, E> + NecessarySendSync + 'or,
    'sub: 'or,
{
    fn on_next(&mut self, value: OE1) {
        self.context.lock_mut(|mut lock| {
            lock.pending_observables.push_back(value);
            if lock.on_going_sub.is_none() {
                subscribe_next(Some(lock), self.context.clone(), self.observer.clone());
            }
        });
    }

    fn on_termination(self, termination: Termination<E>) {
        match termination {
            Termination::Completed => {
                self.context.lock_mut(|mut lock| {
                    lock.completed = true;
                    if lock.on_going_sub.is_none() && lock.pending_observables.is_empty() {
                        drop(lock);
                        safe_lock_option_observer!(on_termination: self.observer, termination);
                    }
                });
            }
            Termination::Error(_) => {
                safe_lock_option_observer!(on_termination: self.observer, termination);
            }
        }
    }
}

struct ConcatAllInnerObserver<'sub, OR, OE1> {
    observer: Shared<Mutable<Option<OR>>>,
    context: Shared<Mutable<ConcatAllContext<'sub, OE1>>>,
    terminated: Shared<AtomicBool>,
}

impl<'or, 'sub, T, E, OR, OE1> Observer<T, E> for ConcatAllInnerObserver<'sub, OR, OE1>
where
    OR: Observer<T, E> + NecessarySendSync + 'or,
    OE1: Observable<'or, 'sub, T, E> + NecessarySendSync + 'or,
    'sub: 'or,
{
    fn on_next(&mut self, value: T) {
        safe_lock_option_observer!(on_next: self.observer, value);
    }

    fn on_termination(self, termination: Termination<E>) {
        self.terminated.store(true, Ordering::SeqCst);
        match termination {
            Termination::Completed => subscribe_next(None, self.context, self.observer),
            Termination::Error(_) => {
                safe_lock_option_observer!(on_termination: self.observer, termination);
            }
        }
    }
}
