//! The [`ElementAt`] operator, behind
//! [`ObservableExt::element_at`](crate::observable::ObservableExt::element_at).

use crate::utils::subscribe_with_auto_dispose_on_termination::AutoDisposeOnTerminationObserver;
use crate::{
    observable::Subscription,
    observable::{Observable, ObservableTypes},
    observer::{Flow, Observer, Termination},
    utils::subscribe_with_auto_dispose_on_termination::subscribe_with_auto_dispose_on_termination,
};
use educe::Educe;

/// Emits only the item at `index`, counting from zero, emitted by the source Observable.
/// See <https://reactivex.io/documentation/operators/elementat.html>
///
/// A source that completes before reaching `index` completes it without an item, rather than with
/// an error.
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt,
///     observer::Termination,
///     operators::{
///         creating::from_iter::FromIter,
///         filtering::element_at::ElementAt,
///     },
/// };
///
/// let mut values = Vec::new();
/// let mut terminations = Vec::new();
///
/// let observable = ElementAt::new(FromIter::new(vec![1, 2, 3]), 1);
/// observable.subscribe_with_callback(
///     |value| values.push(value),
///     |termination| terminations.push(termination),
/// );
///
/// assert_eq!(values, vec![2]);
/// assert_eq!(terminations, vec![Termination::Completed]);
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct ElementAt<OE> {
    source: OE,
    index: usize,
}

impl<OE> ElementAt<OE> {
    /// Creates an [`ElementAt`] over `source`;
    /// [`ObservableExt::element_at`](crate::observable::ObservableExt::element_at) is the fluent
    /// form.
    pub fn new(source: OE, index: usize) -> Self {
        Self { source, index }
    }
}

impl<T, E, OE> ObservableTypes for ElementAt<OE>
where
    OE: ObservableTypes<Item = T, Error = E>,
{
    type Item = T;
    type Error = E;
    type Mode = OE::Mode;
    type D = crate::utils::subscribe_with_auto_dispose_on_termination::Disposal<OE::Mode, OE::D>;
}

impl<T, E, OE, OR> Observable<OR> for ElementAt<OE>
where
    OR: Observer<T, E>,
    OE: Observable<
            ElementAtObserver<
                AutoDisposeOnTerminationObserver<
                    <OE as ObservableTypes>::Mode,
                    OR,
                    <OE as ObservableTypes>::D,
                >,
            >,
            Item = T,
            Error = E,
        >,
{
    fn subscribe(self, observer: OR) -> Subscription<Self::D> {
        subscribe_with_auto_dispose_on_termination(observer, |observer| {
            self.source.subscribe(ElementAtObserver {
                observer: Some(observer),
                index: self.index,
            })
        })
    }
}

pub struct ElementAtObserver<OR> {
    observer: Option<OR>,
    index: usize,
}

impl<T, E, OR> Observer<T, E> for ElementAtObserver<OR>
where
    OR: Observer<T, E>,
{
    fn on_next(&mut self, value: T) -> Flow {
        if self.index > 0 {
            self.index -= 1;
            return Flow::Continue;
        }
        // The wanted element was reached, so the rest of the source is of no use.
        let Some(mut observer) = self.observer.take() else {
            return Flow::Stop;
        };
        if observer.on_next(value).is_continue() {
            observer.on_termination(Termination::Completed);
        }
        Flow::Stop
    }

    fn on_termination(mut self, termination: Termination<E>) {
        if let Some(observer) = self.observer.take() {
            observer.on_termination(termination);
        }
    }
}
