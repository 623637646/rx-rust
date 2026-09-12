use crate::utils::types::MaybeSend;
use crate::{
    observable::Observable,
    observable::Subscription,
    observer::{Flow, Observer, Termination},
    utils::types::MarkerType,
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
    pub fn new<'or, E>(source: OE) -> Self
    where
        OE: Observable<'or, T, E>,
        C: Default + Extend<T>,
    {
        Self {
            source,
            _marker: PhantomData,
        }
    }
}

impl<'or, C, T, E, OE> Observable<'or, C, E> for Collect<C, T, OE>
where
    C: Default + Extend<T> + MaybeSend + 'or,
    OE: Observable<'or, T, E>,
{
    type D = OE::D;

    fn subscribe(self, observer: impl Observer<C, E> + MaybeSend + 'or) -> Subscription<Self::D> {
        let observer = CollectObserver {
            observer,
            collection: C::default(),
        };
        self.source.subscribe(observer)
    }
}

struct CollectObserver<C, OR> {
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
