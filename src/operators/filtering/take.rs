//! The [`Take`] operator, behind [`ObservableExt::take`](crate::observable::ObservableExt::take).

use crate::delegate_disposal;
use crate::disposable::dispose_on_drop::DisposeOnDrop;
use crate::disposable::option_disposal::OptionDisposal;
use crate::disposable::{Disposable, DisposableExt};
use crate::thread_mode::ThreadMode;
use crate::utils::subscribe_with_auto_dispose_on_termination;
use crate::utils::subscribe_with_auto_dispose_on_termination::AutoDisposeOnTerminationObserver;
use crate::{
    observable::{Observable, ObservableTypes},
    observer::{Flow, Observer, Termination},
    utils::subscribe_with_auto_dispose_on_termination::subscribe_with_auto_dispose_on_termination,
};
use educe::Educe;

/// Emits only the first N items emitted by an Observable.
/// See <https://reactivex.io/documentation/operators/take.html>
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt,
///     observer::Termination,
///     operators::{
///         creating::from_iter::FromIter,
///         filtering::take::Take,
///     },
/// };
///
/// let mut values = Vec::new();
/// let mut terminations = Vec::new();
///
/// let observable = Take::new(FromIter::new(vec![1, 2, 3, 4]), 2);
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
pub struct Take<OE> {
    source: OE,
    count: usize,
}

impl<OE> Take<OE> {
    /// Creates a [`Take`] over `source`;
    /// [`ObservableExt::take`](crate::observable::ObservableExt::take) is the fluent form.
    pub fn new(source: OE, count: usize) -> Self {
        Self { source, count }
    }
}

delegate_disposal!(
    Disposal<M, D>,
    OptionDisposal<DisposeOnDrop<subscribe_with_auto_dispose_on_termination::Disposal<M, D>>>,
    where M: ThreadMode, D: Disposable
);

impl<T, E, OE> ObservableTypes for Take<OE>
where
    OE: ObservableTypes<Item = T, Error = E>,
{
    type Item = T;
    type Error = E;
    type Mode = OE::Mode;
    type Disposal = Disposal<OE::Mode, OE::Disposal>;
}

impl<T, E, OE, OR> Observable<OR> for Take<OE>
where
    OR: Observer<T, E>,
    OE: Observable<
            TakeObserver<
                AutoDisposeOnTerminationObserver<
                    <OE as ObservableTypes>::Mode,
                    OR,
                    <OE as ObservableTypes>::Disposal,
                >,
            >,
            Item = T,
            Error = E,
        >,
{
    fn subscribe(self, observer: OR) -> DisposeOnDrop<Self::Disposal> {
        if self.count == 0 {
            observer.on_termination(Termination::Completed);
            OptionDisposal::none().into_dispose_on_drop()
        } else {
            subscribe_with_auto_dispose_on_termination(observer, |observer| {
                self.source.subscribe(TakeObserver {
                    observer: Some(observer),
                    count: self.count,
                })
            })
            .into_option()
            .into_dispose_on_drop()
        }
    }
}

pub struct TakeObserver<OR> {
    observer: Option<OR>,
    count: usize,
}

impl<T, E, OR> Observer<T, E> for TakeObserver<OR>
where
    OR: Observer<T, E>,
{
    fn on_next(&mut self, value: T) -> Flow {
        let Some(observer) = &mut self.observer else {
            return Flow::Stop;
        };
        let flow = observer.on_next(value);
        self.count -= 1;
        if flow.is_stop() {
            // Downstream ended the stream first, so nothing is completed here: the observer is
            // released like a disposed one.
            drop(self.observer.take());
            return Flow::Stop;
        }
        if self.count == 0 {
            self.observer
                .take()
                .unwrap()
                .on_termination(Termination::Completed);
            // The count is reached, so the source is told to stop instead of running to its own
            // end, which is what lets a synchronous one — an infinite one included — return.
            return Flow::Stop;
        }
        Flow::Continue
    }

    fn on_termination(mut self, termination: Termination<E>) {
        if let Some(observer) = self.observer.take() {
            observer.on_termination(termination);
        }
    }
}
