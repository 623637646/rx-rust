//! The [`Contains`] operator, behind
//! [`ObservableExt::contains`](crate::observable::ObservableExt::contains).

use crate::{
    disposable::dispose_on_drop::DisposeOnDrop,
    observable::{Observable, ObservableTypes},
    observer::{Flow, Observer, Termination},
    utils::{
        emit_then_terminate::emit_then_terminate,
        subscribe_with_auto_dispose_on_termination::{
            self, subscribe_with_auto_dispose_on_termination,
        },
    },
};
use educe::Educe;

/// Emits a single boolean value that indicates whether a source observable emits a specified
/// item.
/// See <https://reactivex.io/documentation/operators/contains.html>
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt,
///     observer::Termination,
///     operators::{
///         conditional_boolean::contains::Contains,
///         creating::from_iter::FromIter,
///     },
/// };
///
/// let mut values = Vec::new();
///
/// let observable = Contains::new(FromIter::new([1, 2, 3]), 2);
/// observable.subscribe_with_callback(
///     |value| values.push(value),
///     |termination| assert_eq!(termination, Termination::Completed),
/// );
///
/// assert_eq!(values, vec![true]);
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct Contains<T, OE> {
    source: OE,
    item: T,
}

impl<T, OE> Contains<T, OE> {
    /// Creates a [`Contains`] over `source`;
    /// [`ObservableExt::contains`](crate::observable::ObservableExt::contains) is the fluent form.
    pub fn new(source: OE, item: T) -> Self
    where
        OE: ObservableTypes<Item = T>,
    {
        Self { source, item }
    }
}

impl<T, E, OE> ObservableTypes for Contains<T, OE>
where
    OE: ObservableTypes<Item = T, Error = E>,
{
    type Item = bool;
    type Error = E;
    type Mode = OE::Mode;
    type Disposal = subscribe_with_auto_dispose_on_termination::DisposalOf<OE>;
}

impl<T, E, OE, OR> Observable<OR> for Contains<T, OE>
where
    OR: Observer<bool, E>,
    OE: Observable<
            ContainsObserver<T, subscribe_with_auto_dispose_on_termination::ObserverOf<OE, OR>>,
            Item = T,
            Error = E,
        >,
    T: PartialEq,
{
    fn subscribe(self, observer: OR) -> DisposeOnDrop<Self::Disposal> {
        subscribe_with_auto_dispose_on_termination(observer, |observer| {
            let observer = ContainsObserver {
                observer: Some(observer),
                item: self.item,
            };
            self.source.subscribe(observer)
        })
    }
}

#[doc(hidden)]
pub struct ContainsObserver<T, OR> {
    observer: Option<OR>,
    item: T,
}

impl<T, E, OR> Observer<T, E> for ContainsObserver<T, OR>
where
    OR: Observer<bool, E>,
    T: PartialEq,
{
    fn on_next(&mut self, value: T) -> Flow {
        if self.item != value {
            return Flow::Continue(());
        }
        // The item was found, so the result is decided and the rest of the source is of no use.
        let Some(observer) = self.observer.take() else {
            return Flow::Break(());
        };
        emit_then_terminate(observer, Some(true), Termination::Completed);
        Flow::Break(())
    }

    fn on_termination(mut self, termination: Termination<E>) {
        if let Some(observer) = self.observer.take() {
            emit_then_terminate(observer, Some(false), termination);
        }
    }
}
