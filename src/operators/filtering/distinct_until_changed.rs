use crate::utils::types::NecessarySend;
use crate::{
    disposable::subscription::Subscription,
    observable::Observable,
    observer::{Observer, Termination},
};
use educe::Educe;

/// Emits items from the source Observable only if the current item is different from the immediately preceding item.
/// See <https://reactivex.io/documentation/operators/distinctuntilchanged.html>
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::observable_ext::ObservableExt,
///     observer::Termination,
///     operators::{
///         creating::from_iter::FromIter,
///         filtering::distinct_until_changed::DistinctUntilChanged,
///     },
/// };
///
/// let mut values = Vec::new();
/// let mut terminations = Vec::new();
///
/// let observable = DistinctUntilChanged::new(FromIter::new(vec![1, 1, 2, 2, 1, 3]));
/// observable.subscribe_with_callback(
///     |value| values.push(value),
///     |termination| terminations.push(termination),
/// );
///
/// assert_eq!(values, vec![1, 2, 1, 3]);
/// assert_eq!(terminations, vec![Termination::Completed]);
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct DistinctUntilChanged<OE, F> {
    source: OE,
    key_selector: F,
}

impl<OE, F> DistinctUntilChanged<OE, F> {
    pub fn new_with_key_selector<'or, 'sub, T, E, K>(source: OE, key_selector: F) -> Self
    where
        OE: Observable<'or, 'sub, T, E>,
        F: FnMut(&T) -> K,
    {
        Self {
            source,
            key_selector,
        }
    }
}

impl<T, OE> DistinctUntilChanged<OE, fn(&T) -> T> {
    pub fn new<'or, 'sub, E>(source: OE) -> Self
    where
        T: Clone,
        OE: Observable<'or, 'sub, T, E>,
    {
        Self {
            source,
            key_selector: |x| x.clone(),
        }
    }
}

impl<'or, 'sub, T, E, OE, F, K> Observable<'or, 'sub, T, E> for DistinctUntilChanged<OE, F>
where
    OE: Observable<'or, 'sub, T, E>,
    F: FnMut(&T) -> K + NecessarySend + 'or,
    K: Eq + NecessarySend + 'or,
{
    fn subscribe(self, observer: impl Observer<T, E> + NecessarySend + 'or) -> Subscription<'sub> {
        let observer = DistinctUntilChangedObserver {
            observer,
            key_selector: self.key_selector,
            previous_key: None,
        };
        self.source.subscribe(observer)
    }
}

struct DistinctUntilChangedObserver<OR, F, K> {
    observer: OR,
    key_selector: F,
    previous_key: Option<K>,
}

impl<T, E, OR, F, K> Observer<T, E> for DistinctUntilChangedObserver<OR, F, K>
where
    OR: Observer<T, E>,
    F: FnMut(&T) -> K,
    K: Eq,
{
    fn on_next(&mut self, value: T) {
        let key = (self.key_selector)(&value);
        if let Some(previous_key) = self.previous_key.as_ref()
            && previous_key == &key
        {
            return;
        }
        self.previous_key = Some(key);
        self.observer.on_next(value);
    }

    fn on_termination(self, termination: Termination<E>) {
        self.observer.on_termination(termination)
    }
}
