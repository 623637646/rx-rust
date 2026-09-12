use crate::utils::subscribe_with_auto_dispose_on_termination;
use crate::utils::subscribe_with_auto_dispose_on_termination::subscribe_with_auto_dispose_on_termination;
use crate::utils::types::MaybeSend;
use crate::{
    observable::{Observable, Subscription},
    observer::{Flow, Observer, Termination},
};
use educe::Educe;

/// Emits items emitted by a source Observable as long as a specified condition is true.
/// See <https://reactivex.io/documentation/operators/takewhile.html>
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt,
///     observer::Termination,
///     operators::{
///         conditional_boolean::take_while::TakeWhile,
///         creating::from_iter::FromIter,
///     },
/// };
///
/// let mut values = Vec::new();
/// let mut terminations = Vec::new();
///
/// let observable = TakeWhile::new(FromIter::new(vec![1, 2, 3, 4]), |value| *value < 3);
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
pub struct TakeWhile<OE, F> {
    source: OE,
    callback: F,
}

impl<OE, F> TakeWhile<OE, F> {
    pub fn new<'or, T, E>(source: OE, callback: F) -> Self
    where
        OE: Observable<'or, T, E>,
        F: FnMut(&T) -> bool,
    {
        Self { source, callback }
    }
}

impl<'or, T, E, OE, F> Observable<'or, T, E> for TakeWhile<OE, F>
where
    OE: Observable<'or, T, E>,
    OE::D: MaybeSend + 'or,
    F: FnMut(&T) -> bool + MaybeSend + 'or,
{
    type D = subscribe_with_auto_dispose_on_termination::Disposal<OE::D>;

    fn subscribe(self, observer: impl Observer<T, E> + MaybeSend + 'or) -> Subscription<Self::D> {
        subscribe_with_auto_dispose_on_termination(observer, |observer| {
            let observer = TakeWhileObserver {
                observer: Some(observer),
                callback: self.callback,
            };
            self.source.subscribe(observer)
        })
    }
}

struct TakeWhileObserver<OR, F> {
    observer: Option<OR>,
    callback: F,
}

impl<T, E, OR, F> Observer<T, E> for TakeWhileObserver<OR, F>
where
    OR: Observer<T, E>,
    F: FnMut(&T) -> bool,
{
    fn on_next(&mut self, value: T) -> Flow {
        // A source that does not honor the flow or the disposal keeps emitting; the callback is
        // the caller's and may have side effects, so it must not run once the window has closed.
        let Some(observer) = self.observer.as_mut() else {
            return Flow::Stop;
        };
        if !(self.callback)(&value) {
            if let Some(observer) = self.observer.take() {
                observer.on_termination(Termination::Completed);
            }
            return Flow::Stop;
        }
        let flow = observer.on_next(value);
        if flow.is_stop() {
            drop(self.observer.take());
        }
        flow
    }

    fn on_termination(self, termination: Termination<E>) {
        if let Some(observer) = self.observer {
            observer.on_termination(termination);
        }
    }
}
