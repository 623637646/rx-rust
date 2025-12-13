use crate::utils::types::NecessarySend;
use crate::{
    disposable::subscription::Subscription,
    observable::Observable,
    observer::{Observer, Termination},
    utils::types::MarkerType,
};
use educe::Educe;
use std::{convert::Infallible, marker::PhantomData};

/// Maps an Observable with an `Infallible` item type to an Observable with a concrete item type.
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::observable_ext::ObservableExt,
///     observer::Termination,
///     operators::{
///         creating::empty::Empty,
///         others::map_infallible_to_value::MapInfallibleToValue,
///     },
/// };
///
/// let mut values = Vec::<i32>::new();
/// let mut terminations = Vec::new();
///
/// MapInfallibleToValue::<i32, _>::new(Empty).subscribe_with_callback(
///     |value| values.push(value),
///     |termination| terminations.push(termination),
/// );
///
/// assert!(values.is_empty());
/// assert_eq!(terminations, vec![Termination::Completed]);
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct MapInfallibleToValue<T, OE> {
    source: OE,
    _marker: MarkerType<T>,
}

impl<T, OE> MapInfallibleToValue<T, OE> {
    pub fn new(source: OE) -> Self {
        Self {
            source,
            _marker: PhantomData,
        }
    }
}

impl<'or, 'sub, T, E, OE> Observable<'or, 'sub, T, E> for MapInfallibleToValue<T, OE>
where
    T: 'or,
    OE: Observable<'or, 'sub, Infallible, E>,
{
    fn subscribe(
        self,
        observer: impl Observer<T, E> + NecessarySend + 'or,
    ) -> Subscription<'sub> {
        let observer = MapInfallibleToValueObserver {
            observer,
            _marker: PhantomData,
        };
        self.source.subscribe(observer)
    }
}

struct MapInfallibleToValueObserver<T, OR> {
    observer: OR,
    _marker: MarkerType<T>,
}

impl<T, E, OR> Observer<Infallible, E> for MapInfallibleToValueObserver<T, OR>
where
    OR: Observer<T, E>,
{
    fn on_next(&mut self, _: Infallible) {
        unreachable!()
    }

    fn on_termination(self, termination: Termination<E>) {
        self.observer.on_termination(termination);
    }
}
