//! The [`Single`] operator, behind
//! [`ObservableExt::single`](crate::observable::ObservableExt::single).

use crate::utils::emit_then_terminate::emit_then_terminate;
use crate::{
    disposable::dispose_on_drop::DisposeOnDrop,
    observable::{Observable, ObservableTypes},
    observer::{Flow, Observer, Termination},
    utils::subscribe_with_auto_dispose_on_termination::{
        self, subscribe_with_auto_dispose_on_termination,
    },
};
use educe::Educe;

/// The error type of [`Single`]: the source did not emit exactly one item, or it failed itself.
#[derive(Educe)]
#[educe(Debug, Clone, PartialEq, Eq)]
pub enum Error<E> {
    /// The source completed without emitting an item.
    Empty,
    /// The source emitted a second item.
    MoreThanOne,
    /// The source terminated with this error.
    SourceError(E),
}

/// Emits the only item of the source once it completes, and fails if the source emits none or more
/// than one.
/// See <https://reactivex.io/documentation/operators/first.html> (`single`).
///
/// The item is held until the source completes, since only the completion shows that no second one
/// follows. A second item ends the stream at once with [`Error::MoreThanOne`] and disposes the
/// source; a completion with no item ends it with [`Error::Empty`]. The source's own error is
/// wrapped in [`Error::SourceError`].
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt,
///     observer::Termination,
///     operators::{
///         creating::{empty::Empty, from_iter::FromIter, just::Just},
///         filtering::single::{Error, Single},
///     },
/// };
///
/// let mut values = Vec::new();
/// Single::new(Just::new(1)).subscribe_with_callback(
///     |value| values.push(value),
///     |termination| assert_eq!(termination, Termination::Completed),
/// );
/// assert_eq!(values, [1]);
///
/// Single::new(FromIter::new([1, 2])).subscribe_with_callback(
///     |_| {},
///     |termination| assert_eq!(termination, Termination::Error(Error::MoreThanOne)),
/// );
///
/// Single::new(Empty).subscribe_with_callback(
///     |_| {},
///     |termination| assert_eq!(termination, Termination::Error(Error::Empty)),
/// );
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct Single<OE> {
    source: OE,
}

impl<OE> Single<OE> {
    /// Creates a [`Single`] over `source`;
    /// [`ObservableExt::single`](crate::observable::ObservableExt::single) is the fluent form.
    pub fn new(source: OE) -> Self {
        Self { source }
    }
}

impl<T, E, OE> ObservableTypes for Single<OE>
where
    OE: ObservableTypes<Item = T, Error = E>,
{
    type Item = T;
    type Error = Error<E>;
    type Mode = OE::Mode;
    type Disposal = subscribe_with_auto_dispose_on_termination::DisposalOf<OE>;
}

impl<T, E, OE, OR> Observable<OR> for Single<OE>
where
    OR: Observer<T, Error<E>>,
    OE: Observable<
            SingleObserver<T, subscribe_with_auto_dispose_on_termination::ObserverOf<OE, OR>>,
            Item = T,
            Error = E,
        >,
{
    fn subscribe(self, observer: OR) -> DisposeOnDrop<Self::Disposal> {
        subscribe_with_auto_dispose_on_termination(observer, |observer| {
            self.source.subscribe(SingleObserver {
                observer: Some(observer),
                value: None,
            })
        })
    }
}

#[doc(hidden)]
pub struct SingleObserver<T, OR> {
    observer: Option<OR>,
    value: Option<T>,
}

impl<T, E, OR> Observer<T, E> for SingleObserver<T, OR>
where
    OR: Observer<T, Error<E>>,
{
    fn on_next(&mut self, value: T) -> Flow {
        if self.value.is_none() {
            self.value = Some(value);
            return Flow::Continue(());
        }
        // A second item: the stream has failed, and the rest of the source is of no use.
        if let Some(observer) = self.observer.take() {
            observer.on_termination(Termination::Error(Error::MoreThanOne));
        }
        Flow::Break(())
    }

    fn on_termination(mut self, termination: Termination<E>) {
        let Some(observer) = self.observer.take() else {
            return;
        };
        match termination {
            Termination::Completed => match self.value.take() {
                Some(value) => emit_then_terminate(observer, Some(value), Termination::Completed),
                None => observer.on_termination(Termination::Error(Error::Empty)),
            },
            Termination::Error(error) => {
                observer.on_termination(Termination::Error(Error::SourceError(error)))
            }
        }
    }
}
