//! The [`Hold`] operator, behind
//! [`ObservableExt::hold`](crate::observable::ObservableExt::hold).

use crate::{
    disposable::dispose_on_drop::DisposeOnDrop,
    observable::{Observable, ObservableTypes},
    observer::{Flow, Observer, Termination},
};
use educe::Educe;

/// Holds a value for as long as the subscription to the source lasts, and drops it when the
/// subscription ends: after the termination is delivered, when the subscription is disposed, or
/// when the source lets go of its observer without a termination. The items and the termination
/// go through unchanged.
/// See <https://reactivex.io/documentation/operators/using.html>
///
/// The value lives in the observer given to the source, so it is dropped wherever and whenever the
/// source drops that observer. An observable never subscribed drops it with itself.
///
/// This is how the `StreamAck` of a `FromStreamAcked` item (feature `futures`) is kept until the
/// item has been handled, inside the `concat_map` or `flat_map` that handles it.
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt,
///     observer::{Observer, Termination},
///     subject::publish_subject::PublishSubject,
/// };
/// use std::{cell::Cell, convert::Infallible, rc::Rc};
///
/// // A value that tells when it is dropped.
/// struct Guard(Rc<Cell<bool>>);
/// impl Drop for Guard {
///     fn drop(&mut self) {
///         self.0.set(true);
///     }
/// }
///
/// let dropped = Rc::new(Cell::new(false));
/// let mut subject = PublishSubject::<_, Infallible, _>::local();
/// let _subscription = subject
///     .clone()
///     .hold(Guard(dropped.clone()))
///     .subscribe_with_callback(|_: i32| {}, |_| {});
///
/// let _ = subject.on_next(1);
/// assert!(!dropped.get()); // Held while the subscription lasts.
///
/// subject.on_termination(Termination::Completed);
/// assert!(dropped.get()); // Dropped once it has ended.
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct Hold<OE, R> {
    source: OE,
    resource: R,
}

impl<OE, R> Hold<OE, R> {
    /// Creates a [`Hold`] over `source`;
    /// [`ObservableExt::hold`](crate::observable::ObservableExt::hold) is the fluent form.
    pub fn new(source: OE, resource: R) -> Self {
        Self { source, resource }
    }
}

impl<OE, R> ObservableTypes for Hold<OE, R>
where
    OE: ObservableTypes,
{
    type Item = OE::Item;
    type Error = OE::Error;
    type Mode = OE::Mode;
    type Disposal = OE::Disposal;
}

impl<T, E, OE, R, OR> Observable<OR> for Hold<OE, R>
where
    OR: Observer<T, E>,
    OE: Observable<HoldObserver<OR, R>, Item = T, Error = E>,
{
    fn subscribe(self, observer: OR) -> DisposeOnDrop<Self::Disposal> {
        self.source.subscribe(HoldObserver {
            observer,
            _resource: self.resource,
        })
    }
}

#[doc(hidden)]
pub struct HoldObserver<OR, R> {
    observer: OR,
    _resource: R,
}

impl<T, E, OR, R> Observer<T, E> for HoldObserver<OR, R>
where
    OR: Observer<T, E>,
{
    fn on_next(&mut self, value: T) -> Flow {
        self.observer.on_next(value)
    }

    fn on_termination(self, termination: Termination<E>) {
        // The resource is dropped when this returns, after the termination is delivered.
        self.observer.on_termination(termination)
    }
}
