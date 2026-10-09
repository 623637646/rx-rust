//! The [`Concat`] operator, behind
//! [`ObservableExt::concat_with`](crate::observable::ObservableExt::concat_with).

use crate::delegate_disposal;
use crate::disposable::{
    Disposable, chain_disposal::ChainDisposal, shared_disposal::SharedDisposal,
};
use crate::thread_mode::Joined;
use crate::thread_mode::ThreadMode;
use crate::{
    disposable::dispose_on_drop::DisposeOnDrop,
    observable::{Observable, ObservableTypes},
    observer::{Flow, Observer, Termination},
};
use educe::Educe;

/// Emits all of the values of the first Observable, then, once it completes, all of the values of
/// the second.
/// See <https://reactivex.io/documentation/operators/concat.html>
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt,
///     observer::Termination,
///     operators::{
///         combining::concat::Concat,
///         creating::from_iter::FromIter,
///     },
/// };
///
/// let mut values = Vec::new();
/// let mut terminations = Vec::new();
///
/// let observable =
///     Concat::new(FromIter::new(vec![1, 2]), FromIter::new(vec![3, 4]));
/// observable.subscribe_with_callback(
///     |value| values.push(value),
///     |termination| terminations.push(termination),
/// );
///
/// assert_eq!(values, vec![1, 2, 3, 4]);
/// assert_eq!(terminations, vec![Termination::Completed]);
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct Concat<OE1, OE2> {
    source_1: OE1,
    source_2: OE2,
}

impl<OE1, OE2> Concat<OE1, OE2> {
    /// Creates a [`Concat`] over `source_1` and `source_2`;
    /// [`ObservableExt::concat_with`](crate::observable::ObservableExt::concat_with) is the fluent
    /// form.
    pub fn new<T, E>(source_1: OE1, source_2: OE2) -> Self
    where
        OE1: ObservableTypes<Item = T, Error = E>,
        OE2: ObservableTypes<Item = T, Error = E>,
    {
        Self { source_1, source_2 }
    }
}

delegate_disposal!(
    Disposal<M, D1, D2>,
    ChainDisposal<SharedDisposal<M, DisposeOnDrop<D2>>, D1>,
    where M: ThreadMode, D1: Disposable, D2: Disposable
);

impl<T, E, OE1, OE2> ObservableTypes for Concat<OE1, OE2>
where
    OE1: ObservableTypes<Item = T, Error = E>,
    OE2: ObservableTypes<Item = T, Error = E>,
{
    type Item = T;
    type Error = E;
    type Mode = Joined<OE1::Mode, OE2::Mode>;
    type Disposal = Disposal<Joined<OE1::Mode, OE2::Mode>, OE1::Disposal, OE2::Disposal>;
}

impl<T, E, OE1, OE2, OR> Observable<OR> for Concat<OE1, OE2>
where
    OR: Observer<T, E>,
    OE1: Observable<
            ConcatObserver<
                Joined<<OE1 as ObservableTypes>::Mode, <OE2 as ObservableTypes>::Mode>,
                OR,
                OE2,
                <OE2 as ObservableTypes>::Disposal,
            >,
            Item = T,
            Error = E,
        >,
    OE2: Observable<OR, Item = T, Error = E>,
{
    fn subscribe(self, observer: OR) -> DisposeOnDrop<Self::Disposal> {
        let sub_2 = SharedDisposal::default();
        let observer = ConcatObserver {
            observer,
            source_2: self.source_2,
            sub_2: sub_2.clone(),
        };
        self.source_1
            .subscribe(observer)
            .preceded_by(sub_2)
            .map_into()
    }
}

pub struct ConcatObserver<M: ThreadMode, OR, OE2, D: Disposable> {
    observer: OR,
    source_2: OE2,
    sub_2: SharedDisposal<M, DisposeOnDrop<D>>,
}

impl<M, T, E, OR, OE2> Observer<T, E> for ConcatObserver<M, OR, OE2, OE2::Disposal>
where
    M: ThreadMode,
    OR: Observer<T, E>,
    OE2: Observable<OR, Item = T, Error = E>,
{
    fn on_next(&mut self, value: T) -> Flow {
        self.observer.on_next(value)
    }

    fn on_termination(self, termination: Termination<E>) {
        match termination {
            Termination::Completed => {
                // `replace` does not run the builder once the subscription was disposed, so the
                // second source is never subscribed after downstream unsubscribed, and a
                // subscription built while downstream unsubscribes is disposed right away.
                self.sub_2
                    .replace(|| self.source_2.subscribe(self.observer));
            }
            error @ Termination::Error(_) => {
                self.observer.on_termination(error);
            }
        }
    }
}
