//! The [`DefaultIfEmpty`] operator, behind
//! [`ObservableExt::default_if_empty`](crate::observable::ObservableExt::default_if_empty).

use crate::{
    observable::{Observable, ObservableTypes, Subscription},
    observer::{Flow, Observer, Termination},
};
use educe::Educe;

/// Emits a specified item if the source Observable completes without emitting any items.
/// See <https://reactivex.io/documentation/operators/defaultifempty.html>
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt,
///     observer::Termination,
///     operators::{
///         conditional_boolean::default_if_empty::DefaultIfEmpty,
///         creating::from_iter::FromIter,
///     },
/// };
///
/// let mut values = Vec::new();
/// let mut terminations = Vec::new();
///
/// let observable = DefaultIfEmpty::new(FromIter::new(Vec::<i32>::new()), 42);
/// observable.subscribe_with_callback(
///     |value| values.push(value),
///     |termination| terminations.push(termination),
/// );
///
/// assert_eq!(values, vec![42]);
/// assert_eq!(terminations, vec![Termination::Completed]);
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct DefaultIfEmpty<T, OE> {
    source: OE,
    default_value: T,
}

impl<T, OE> DefaultIfEmpty<T, OE> {
    /// Creates a [`DefaultIfEmpty`] over `source`;
    /// [`ObservableExt::default_if_empty`](crate::observable::ObservableExt::default_if_empty) is
    /// the fluent form.
    pub fn new<E>(source: OE, item: T) -> Self
    where
        OE: ObservableTypes<Item = T, Error = E>,
    {
        Self {
            source,
            default_value: item,
        }
    }
}

impl<T, E, OE> ObservableTypes for DefaultIfEmpty<T, OE>
where
    OE: ObservableTypes<Item = T, Error = E>,
{
    type Item = T;
    type Error = E;
    type Mode = OE::Mode;
    type Disposal = OE::Disposal;
}

impl<T, E, OE, OR> Observable<OR> for DefaultIfEmpty<T, OE>
where
    OR: Observer<T, E>,
    OE: Observable<DefaultIfEmptyObserver<T, OR>, Item = T, Error = E>,
{
    fn subscribe(self, observer: OR) -> Subscription<Self::Disposal> {
        let observer = DefaultIfEmptyObserver {
            observer,
            default_value: Some(self.default_value),
        };
        self.source.subscribe(observer)
    }
}

pub struct DefaultIfEmptyObserver<T, OR> {
    observer: OR,
    default_value: Option<T>, // None means the source's got at least one value already.
}

impl<T, E, OR> Observer<T, E> for DefaultIfEmptyObserver<T, OR>
where
    OR: Observer<T, E>,
{
    fn on_next(&mut self, value: T) -> Flow {
        self.default_value = None;
        self.observer.on_next(value)
    }

    fn on_termination(mut self, termination: Termination<E>) {
        // The final value ends the stream, so a downstream that stopped on it is not completed
        // on top of that: it has already ended itself.
        if matches!(termination, Termination::Completed)
            && let Some(default_value) = self.default_value.take()
            && self.observer.on_next(default_value).is_stop()
        {
            return;
        }
        self.observer.on_termination(termination);
    }
}
