use crate::utils::types::MaybeSend;
use crate::{
    observable::{Observable, Subscription},
    observer::{Flow, Observer, Termination},
    utils::types::MarkerType,
};
use educe::Educe;
use std::marker::PhantomData;

/// Transforms an Observable's error while leaving its items unchanged.
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt,
///     observer::Termination,
///     operators::creating::throw::Throw,
/// };
///
/// let mut terminations = Vec::new();
///
/// Throw::new("boom")
///     .with_item_type::<i32>()
///     .map_err(|error| error.len())
///     .subscribe_with_callback(
///         |_| -> () { unreachable!() },
///         |termination| terminations.push(termination),
///     );
///
/// assert_eq!(terminations, vec![Termination::Error(4)]);
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct MapErr<E, OE, F> {
    source: OE,
    callback: F,
    _marker: MarkerType<E>,
}

impl<E, OE, F> MapErr<E, OE, F> {
    pub fn new<'or, T, E1>(source: OE, callback: F) -> Self
    where
        OE: Observable<'or, T, E>,
        F: FnOnce(E) -> E1,
    {
        Self {
            source,
            callback,
            _marker: PhantomData,
        }
    }
}

impl<'or, T, E, E1, OE, F> Observable<'or, T, E1> for MapErr<E, OE, F>
where
    OE: Observable<'or, T, E>,
    F: FnOnce(E) -> E1 + MaybeSend + 'or,
{
    type D = OE::D;

    fn subscribe(self, observer: impl Observer<T, E1> + MaybeSend + 'or) -> Subscription<Self::D> {
        self.source.subscribe(MapErrObserver {
            observer,
            callback: self.callback,
        })
    }
}

struct MapErrObserver<OR, F> {
    observer: OR,
    callback: F,
}

impl<T, E, E1, OR, F> Observer<T, E> for MapErrObserver<OR, F>
where
    OR: Observer<T, E1>,
    F: FnOnce(E) -> E1,
{
    fn on_next(&mut self, value: T) -> Flow {
        self.observer.on_next(value)
    }

    fn on_termination(self, termination: Termination<E>) {
        match termination {
            Termination::Completed => self.observer.on_termination(Termination::Completed),
            Termination::Error(error) => self
                .observer
                .on_termination(Termination::Error((self.callback)(error))),
        }
    }
}
