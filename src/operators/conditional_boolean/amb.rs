use crate::disposable::Disposable;
use crate::utils::types::{Mutable, MutableHelper, NecessarySend, Shared};
use crate::{
    disposable::subscription::Subscription,
    observable::Observable,
    observer::{Observer, Termination},
};
use crate::{safe_lock, safe_lock_option, safe_lock_slot_map};
use educe::Educe;
use slotmap::{DefaultKey, SlotMap};

/// Given two or more source Observables, emit all of the items from only the first of these Observables to emit an item or notification.
/// See <https://reactivex.io/documentation/operators/amb.html>
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::observable_ext::ObservableExt,
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

impl<'or, 'sub, T, E, OE, I> Observable<'or, 'sub, T, E> for Amb<I>
where
    'sub: 'or,
    I: IntoIterator<Item = OE>,
    OE: Observable<'or, 'sub, T, E>,
{
    fn subscribe(self, observer: impl Observer<T, E> + NecessarySend + 'or) -> Subscription<'sub> {
        let observer = Shared::new(Mutable::new(Some(observer)));

        let mut slop_map = SlotMap::new();
        let sources_and_key: Vec<_> = self
            .sources
            .into_iter()
            .map(|e| {
                let key = slop_map.insert(None);
                (e, key)
            })
            .collect();
        let context = Shared::new(Mutable::new(AmbContext {
            subscriptions: slop_map,
        }));

        for (source, key) in sources_and_key {
            if safe_lock_option!(is_none: observer) {
                // determined
                break;
            }
            let amb_observer = AmbObserver {
                observer: observer.clone(),
                context: context.clone(),
                key,
                determined_observer: None,
            };
            let sub = source.subscribe(amb_observer);
            safe_lock_slot_map!(replace: context, subscriptions, key, Some(sub));
        }

        Subscription::new_with_disposal(context)
    }
}

struct AmbContext<'sub> {
    subscriptions: SlotMap<DefaultKey, Option<Subscription<'sub>>>,
}

impl Disposable for Shared<Mutable<AmbContext<'_>>> {
    fn dispose(self) {
        safe_lock!(mem_take: self, subscriptions);
    }
}

// TODO: should use state for observer and determined_observer for better performance.
struct AmbObserver<'sub, OR> {
    observer: Shared<Mutable<Option<OR>>>,
    context: Shared<Mutable<AmbContext<'sub>>>,
    key: DefaultKey,
    determined_observer: Option<OR>, // Some means this AmbObserver is the first. None means this AmbObserver is not the first or not determined yet.
}

impl<'sub, T, E, OR> Observer<T, E> for AmbObserver<'sub, OR>
where
    OR: Observer<T, E>,
{
    fn on_next(&mut self, value: T) {
        if let Some(observer) = self.determined_observer.as_mut() {
            observer.on_next(value);
            return;
        }
        if let Some(observer) = safe_lock_option!(take: self.observer) {
            self.determined_observer = Some(observer);
            drop_none_matched_subscriptions(&self.context, self.key);
            self.on_next(value);
        }
    }

    fn on_termination(self, termination: Termination<E>) {
        if let Some(observer) = self.determined_observer {
            observer.on_termination(termination);
            return;
        }
        if let Some(observer) = safe_lock_option!(take: self.observer) {
            drop_none_matched_subscriptions(&self.context, self.key);
            observer.on_termination(termination);
        }
    }
}

fn drop_none_matched_subscriptions<'sub>(context: &Mutable<AmbContext<'sub>>, key: DefaultKey) {
    let drop_subscriptions = context.lock_mut(|mut lock| {
        let keep = lock.subscriptions.remove(key).unwrap();
        let mut keep_slot_map = SlotMap::new();
        keep_slot_map.insert(keep);
        std::mem::replace(&mut lock.subscriptions, keep_slot_map)
    });
    drop(drop_subscriptions);
}
