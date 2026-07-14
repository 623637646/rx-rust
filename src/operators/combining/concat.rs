use crate::delegate_disposal;
use crate::disposable::{Disposable, chain_disposal::ChainDisposal};
use crate::safe_lock_option;
use crate::utils::types::{MaybeSend, Mutable, Shared};
use crate::{
    observable::{Observable, Subscription},
    observer::{Observer, Termination},
};
use educe::Educe;

/// Concatenates multiple Observables to create an Observable that emits all of the values from the first, then all of the values from the second, and so on.
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
    pub fn new<'or, T, E>(source_1: OE1, source_2: OE2) -> Self
    where
        OE1: Observable<'or, T = T, E = E>,
        OE2: Observable<'or, T = T, E = E>,
    {
        Self { source_1, source_2 }
    }
}

delegate_disposal!(
    Disposal<D1, D2>,
    ChainDisposal<Shared<Mutable<Option<Subscription<D2>>>>, D1>,
    where D1: Disposable, D2: Disposable
);

impl<'or, T, E, OE1, OE2> Observable<'or> for Concat<OE1, OE2>
where
    OE1: Observable<'or, T = T, E = E>,
    OE2: Observable<'or, T = T, E = E> + MaybeSend + 'or,
    OE2::D: MaybeSend + 'or,
{
    type T = T;
    type E = E;
    type D = Disposal<OE1::D, OE2::D>;

    fn subscribe(self, observer: impl Observer<T, E> + MaybeSend + 'or) -> Subscription<Self::D> {
        let sub_2 = Shared::new(Mutable::new(None));
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

struct ConcatObserver<OR, OE2, D: Disposable> {
    observer: OR,
    source_2: OE2,
    sub_2: Shared<Mutable<Option<Subscription<D>>>>,
}

impl<'or, T, E, OR, OE2> Observer<T, E> for ConcatObserver<OR, OE2, OE2::D>
where
    OR: Observer<T, E> + MaybeSend + 'or,
    OE2: Observable<'or, T = T, E = E>,
{
    fn on_next(&mut self, value: T) {
        self.observer.on_next(value);
    }

    fn on_termination(self, termination: Termination<E>) {
        match termination {
            Termination::Completed => {
                let sub = self.source_2.subscribe(self.observer);
                safe_lock_option!(replace: self.sub_2, sub);
            }
            Termination::Error(_) => {
                self.observer.on_termination(termination);
            }
        }
    }
}
