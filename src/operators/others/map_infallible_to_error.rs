use crate::utils::types::NecessarySendSync;
use crate::{
    disposable::subscription::Subscription,
    observable::Observable,
    observer::{Observer, Termination},
    utils::types::MarkerType,
};
use educe::Educe;
use std::{convert::Infallible, marker::PhantomData};

/// Maps an Observable with an `Infallible` error type to an Observable with a concrete error type.
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::observable_ext::ObservableExt,
///     observer::Termination,
///     operators::{
///         creating::from_iter::FromIter,
///         others::map_infallible_to_error::MapInfallibleToError,
///     },
/// };
///
/// let mut values = Vec::new();
/// let mut terminations = Vec::new();
///
/// MapInfallibleToError::<String, _>::new(FromIter::new(vec![1, 2]))
///     .subscribe_with_callback(
///         |value| values.push(value),
///         |termination| terminations.push(termination),
///     );
///
/// assert_eq!(values, vec![1, 2]);
/// assert_eq!(terminations, vec![Termination::Completed]);
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct MapInfallibleToError<E, OE> {
    source: OE,
    _marker: MarkerType<E>,
}

impl<E, OE> MapInfallibleToError<E, OE> {
    pub fn new(source: OE) -> Self {
        Self {
            source,
            _marker: PhantomData,
        }
    }
}

impl<'or, 'sub, T, E, OE> Observable<'or, 'sub, T, E> for MapInfallibleToError<E, OE>
where
    E: 'or,
    OE: Observable<'or, 'sub, T, Infallible>,
{
    fn subscribe(
        self,
        observer: impl Observer<T, E> + NecessarySendSync + 'or,
    ) -> Subscription<'sub> {
        let observer = MapInfallibleToErrorObserver {
            observer,
            _marker: PhantomData,
        };
        self.source.subscribe(observer)
    }
}

struct MapInfallibleToErrorObserver<E, OR> {
    observer: OR,
    _marker: MarkerType<E>,
}

impl<T, E, OR> Observer<T, Infallible> for MapInfallibleToErrorObserver<E, OR>
where
    OR: Observer<T, E>,
{
    fn on_next(&mut self, value: T) {
        self.observer.on_next(value);
    }

    fn on_termination(self, termination: Termination<Infallible>) {
        match termination {
            Termination::Completed => self.observer.on_termination(Termination::Completed),
            Termination::Error(_) => unreachable!(),
        }
    }
}
