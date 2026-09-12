use crate::utils::types::MaybeSend;
use crate::{
    observable::{Observable, Subscription},
    observer::{Flow, Observer, Termination},
    utils::types::MarkerType,
};
use educe::Educe;
use std::{convert::Infallible, marker::PhantomData};

/// Gives an Observable whose item type is `Infallible` a concrete item type.
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt,
///     observer::Termination,
///     operators::creating::empty::Empty,
/// };
///
/// let mut values = Vec::<i32>::new();
/// let mut terminations = Vec::new();
///
/// Empty.with_item_type::<i32>().subscribe_with_callback(
///     |value| values.push(value),
///     |termination| terminations.push(termination),
/// );
///
/// assert!(values.is_empty());
/// assert_eq!(terminations, vec![Termination::Completed]);
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct WithItemType<T, OE> {
    source: OE,
    _marker: MarkerType<T>,
}

impl<T, OE> WithItemType<T, OE> {
    pub fn new(source: OE) -> Self {
        Self {
            source,
            _marker: PhantomData,
        }
    }
}

impl<'or, T, E, OE> Observable<'or, T, E> for WithItemType<T, OE>
where
    T: 'or,
    OE: Observable<'or, Infallible, E>,
{
    type D = OE::D;

    fn subscribe(self, observer: impl Observer<T, E> + MaybeSend + 'or) -> Subscription<Self::D> {
        let observer = WithItemTypeObserver {
            observer,
            _marker: PhantomData,
        };
        self.source.subscribe(observer)
    }
}

struct WithItemTypeObserver<T, OR> {
    observer: OR,
    _marker: MarkerType<T>,
}

impl<T, E, OR> Observer<Infallible, E> for WithItemTypeObserver<T, OR>
where
    OR: Observer<T, E>,
{
    fn on_next(&mut self, value: Infallible) -> Flow {
        match value {}
    }

    fn on_termination(self, termination: Termination<E>) {
        self.observer.on_termination(termination);
    }
}
