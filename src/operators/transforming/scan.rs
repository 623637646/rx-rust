use crate::utils::types::NecessarySendSync;
use crate::{
    disposable::subscription::Subscription,
    observable::Observable,
    observer::{Observer, Termination},
    utils::types::MarkerType,
};
use educe::Educe;
use std::marker::PhantomData;

/// Applies a function to each item emitted by an Observable, sequentially, and emits each intermediate accumulated value.
/// See <https://reactivex.io/documentation/operators/scan.html>
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::observable_ext::ObservableExt,
///     observer::Termination,
///     operators::{
///         creating::from_iter::FromIter,
///         transforming::scan::Scan,
///     },
/// };
///
/// let mut values = Vec::new();
/// let mut terminations = Vec::new();
///
/// let observable = Scan::new(FromIter::new(vec![1, 2, 3]), 0, |acc, value| acc + value);
/// observable.subscribe_with_callback(
///     |value| values.push(value),
///     |termination| terminations.push(termination),
/// );
///
/// assert_eq!(values, vec![1, 3, 6]);
/// assert_eq!(terminations, vec![Termination::Completed]);
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct Scan<T, T1, OE, F> {
    source: OE,
    initial_value: T,
    callback: F,
    _marker: MarkerType<T1>,
}

impl<T, T1, OE, F> Scan<T, T1, OE, F> {
    pub fn new<'or, 'sub, E>(source: OE, initial_value: T, callback: F) -> Self
    where
        OE: Observable<'or, 'sub, T1, E>,
        F: FnMut(T, T1) -> T,
    {
        Self {
            source,
            initial_value,
            callback,
            _marker: PhantomData,
        }
    }
}

impl<'or, 'sub, T, T1, E, OE, F> Observable<'or, 'sub, T, E> for Scan<T, T1, OE, F>
where
    T: Clone + NecessarySendSync + 'or,
    OE: Observable<'or, 'sub, T1, E>,
    F: FnMut(T, T1) -> T + NecessarySendSync + 'or,
{
    fn subscribe(
        self,
        observer: impl Observer<T, E> + NecessarySendSync + 'or,
    ) -> Subscription<'sub> {
        let observer = ScanObserver {
            observer,
            value: self.initial_value,
            callback: self.callback,
        };
        self.source.subscribe(observer)
    }
}

struct ScanObserver<T, OR, F> {
    observer: OR,
    value: T,
    callback: F,
}

impl<T, T1, E, OR, F> Observer<T1, E> for ScanObserver<T, OR, F>
where
    T: Clone,
    OR: Observer<T, E>,
    F: FnMut(T, T1) -> T,
{
    fn on_next(&mut self, value: T1) {
        self.value = (self.callback)(self.value.clone(), value);
        self.observer.on_next(self.value.clone())
    }

    fn on_termination(self, termination: Termination<E>) {
        self.observer.on_termination(termination)
    }
}
