use crate::utils::subscribe_unsub_after_termination::subscribe_unsub_after_termination;
use crate::utils::types::{MarkerType, NecessarySend};
use crate::{
    disposable::subscription::Subscription,
    observable::Observable,
    observer::{Observer, Termination},
};
use educe::Educe;
use std::marker::PhantomData;

/// Emits a single boolean value that indicates whether all items emitted by a source Observable satisfy a specified condition.
/// See <https://reactivex.io/documentation/operators/all.html>
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::observable_ext::ObservableExt,
///     observer::Termination,
///     operators::{
///         conditional_boolean::all::All,
///         creating::from_iter::FromIter,
///     },
/// };
///
/// let mut values = Vec::new();
/// let mut terminations = Vec::new();
///
/// let observable = All::new(FromIter::new(vec![1, 2, 3]), |value| value < 5);
/// observable.subscribe_with_callback(
///     |value| values.push(value),
///     |termination| terminations.push(termination),
/// );
///
/// assert_eq!(values, vec![true]);
/// assert_eq!(terminations, vec![Termination::Completed]);
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct All<T, OE, F> {
    source: OE,
    callback: F,
    _marker: MarkerType<T>,
}

impl<T, OE, F> All<T, OE, F> {
    pub fn new<'or, 'sub, E>(source: OE, callback: F) -> Self
    where
        OE: Observable<'or, 'sub, T, E>,
        F: FnMut(T) -> bool,
    {
        Self {
            source,
            callback,
            _marker: PhantomData,
        }
    }
}

impl<'or, 'sub, T, E, OE, F> Observable<'or, 'sub, bool, E> for All<T, OE, F>
where
    OE: Observable<'or, 'sub, T, E>,
    F: FnMut(T) -> bool + NecessarySend + 'or,
    'sub: 'or,
{
    fn subscribe(
        self,
        observer: impl Observer<bool, E> + NecessarySend + 'or,
    ) -> Subscription<'sub> {
        subscribe_unsub_after_termination(observer, |observer| {
            let observer = AllObserver {
                observer: Some(observer),
                callback: self.callback,
            };
            self.source.subscribe(observer)
        })
    }
}

struct AllObserver<OR, F> {
    observer: Option<OR>,
    callback: F,
}

impl<T, E, OR, F> Observer<T, E> for AllObserver<OR, F>
where
    OR: Observer<bool, E>,
    F: FnMut(T) -> bool,
{
    fn on_next(&mut self, value: T) {
        if !(self.callback)(value)
            && let Some(mut observer) = self.observer.take()
        {
            observer.on_next(false);
            observer.on_termination(Termination::Completed);
        }
    }

    fn on_termination(mut self, termination: Termination<E>) {
        if let Some(mut observer) = self.observer.take() {
            match termination {
                Termination::Completed => observer.on_next(true),
                Termination::Error(_) => {}
            }
            observer.on_termination(termination);
        }
    }
}
