//! The [`Collect`] operator, behind
//! [`ObservableExt::collect`](crate::observable::ObservableExt::collect),
//! [`ObservableExt::to_vec`](crate::observable::ObservableExt::to_vec).

use crate::{
    observable::Subscription,
    observable::{Observable, ObservableTypes},
    observer::{Flow, Observer, Termination},
    utils::MarkerType,
};
use educe::Educe;
use std::marker::PhantomData;

/// Gathers all the items emitted by an Observable into a single collection and emits it when the
/// source completes.
///
/// The collection is built with [`Default`] and [`Extend`], so it can be a `Vec<T>`, a
/// `HashSet<T>`, a `String` of `char`s, or any other type that implements both.
/// [`to_vec`](crate::observable::ObservableExt::to_vec) is the `Vec<T>` case.
///
/// A source that completes without emitting yields the empty collection, an error discards the
/// items gathered so far and is forwarded on its own, and a source that never terminates never
/// emits while gathering its items without bound.
/// See <https://reactivex.io/documentation/operators/to.html>
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt,
///     observer::Termination,
///     operators::creating::from_iter::FromIter,
/// };
/// use std::collections::BTreeSet;
///
/// let mut values = Vec::new();
/// let mut terminations = Vec::new();
///
/// let observable = FromIter::new(vec![1, 2, 2, 3]).collect::<BTreeSet<_>>();
/// observable.subscribe_with_callback(
///     |value| values.push(value),
///     |termination| terminations.push(termination),
/// );
///
/// assert_eq!(values, vec![BTreeSet::from([1, 2, 3])]);
/// assert_eq!(terminations, vec![Termination::Completed]);
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct Collect<C, T, OE> {
    source: OE,
    _marker: MarkerType<(C, T)>,
}

impl<C, T, OE> Collect<C, T, OE> {
    /// Creates a [`Collect`] over `source`;
    /// [`ObservableExt::collect`](crate::observable::ObservableExt::collect) is the fluent form.
    pub fn new<E>(source: OE) -> Self
    where
        OE: ObservableTypes<Item = T, Error = E>,
        C: Default + Extend<T>,
    {
        Self {
            source,
            _marker: PhantomData,
        }
    }
}

impl<C, T, E, OE> ObservableTypes for Collect<C, T, OE>
where
    C: Default + Extend<T>,
    OE: ObservableTypes<Item = T, Error = E>,
{
    type Item = C;
    type Error = E;
    type Mode = OE::Mode;
    type D = OE::D;
}

impl<C, T, E, OE, OR> Observable<OR> for Collect<C, T, OE>
where
    OR: Observer<C, E>,
    C: Default + Extend<T>,
    OE: Observable<CollectObserver<C, OR>, Item = T, Error = E>,
{
    fn subscribe(self, observer: OR) -> Subscription<Self::D> {
        let observer = CollectObserver {
            observer,
            collection: C::default(),
        };
        self.source.subscribe(observer)
    }
}

pub struct CollectObserver<C, OR> {
    observer: OR,
    collection: C,
}

impl<C, T, E, OR> Observer<T, E> for CollectObserver<C, OR>
where
    C: Extend<T>,
    OR: Observer<C, E>,
{
    fn on_next(&mut self, value: T) -> Flow {
        self.collection.extend(Some(value));
        Flow::Continue
    }

    fn on_termination(mut self, termination: Termination<E>) {
        // The final value ends the stream, so a downstream that stopped on it is not completed
        // on top of that: it has already ended itself.
        if matches!(termination, Termination::Completed)
            && self.observer.on_next(self.collection).is_stop()
        {
            return;
        }
        self.observer.on_termination(termination)
    }
}
