//! The [`SkipWhile`] operator, behind
//! [`ObservableExt::skip_while`](crate::observable::ObservableExt::skip_while).

use crate::{
    disposable::dispose_on_drop::DisposeOnDrop,
    observable::{Observable, ObservableTypes},
    observer::{Flow, Observer, Termination},
};
use educe::Educe;

/// Discards items emitted by an observable until a specified condition becomes false.
/// See <https://reactivex.io/documentation/operators/skipwhile.html>
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt,
///     observer::Termination,
///     operators::{
///         conditional_boolean::skip_while::SkipWhile,
///         creating::from_iter::FromIter,
///     },
/// };
///
/// let mut values = Vec::new();
///
/// let observable = SkipWhile::new(FromIter::new(vec![1, 2, 3, 4]), |value| *value < 3);
/// observable.subscribe_with_callback(
///     |value| values.push(value),
///     |termination| assert_eq!(termination, Termination::Completed),
/// );
///
/// assert_eq!(values, vec![3, 4]);
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct SkipWhile<OE, F> {
    source: OE,
    callback: F,
}

impl<OE, F> SkipWhile<OE, F> {
    /// Creates a [`SkipWhile`] over `source`;
    /// [`ObservableExt::skip_while`](crate::observable::ObservableExt::skip_while) is the fluent
    /// form.
    pub fn new<T>(source: OE, callback: F) -> Self
    where
        OE: ObservableTypes<Item = T>,
        F: FnMut(&T) -> bool,
    {
        Self { source, callback }
    }
}

impl<T, E, OE, F> ObservableTypes for SkipWhile<OE, F>
where
    OE: ObservableTypes<Item = T, Error = E>,
    F: FnMut(&T) -> bool,
{
    type Item = T;
    type Error = E;
    type Mode = OE::Mode;
    type Disposal = OE::Disposal;
}

impl<T, E, OE, F, OR> Observable<OR> for SkipWhile<OE, F>
where
    OR: Observer<T, E>,
    OE: Observable<SkipWhileObserver<OR, F>, Item = T, Error = E>,
    F: FnMut(&T) -> bool,
{
    fn subscribe(self, observer: OR) -> DisposeOnDrop<Self::Disposal> {
        let observer = SkipWhileObserver {
            observer,
            callback: self.callback,
            skip: true,
        };
        self.source.subscribe(observer)
    }
}

#[doc(hidden)]
pub struct SkipWhileObserver<OR, F> {
    observer: OR,
    callback: F,
    skip: bool,
}

impl<T, E, OR, F> Observer<T, E> for SkipWhileObserver<OR, F>
where
    OR: Observer<T, E>,
    F: FnMut(&T) -> bool,
{
    fn on_next(&mut self, value: T) -> Flow {
        if self.skip {
            self.skip = (self.callback)(&value);
        }
        if self.skip {
            Flow::Continue
        } else {
            self.observer.on_next(value)
        }
    }

    fn on_termination(self, termination: Termination<E>) {
        self.observer.on_termination(termination)
    }
}
