use crate::delegate_disposal;
use crate::disposable::Disposable;
use crate::operators::others::map_infallible_to_error::MapInfallibleToError;
use crate::utils::subscribe_with_shared_model::{
    self, Context, ModificationResult, subscribe_with_shared_model,
};
use crate::utils::types::MaybeSend;
use crate::{
    observable::{Observable, Subscription},
    observer::{Observer, Termination},
    operators::creating::from_iter::FromIter,
    utils::{
        subscribe_unsub_after_termination::{self, subscribe_unsub_after_termination},
        types::MarkerType,
    },
};
use educe::Educe;
use slotmap::{DefaultKey, SlotMap};
use std::marker::PhantomData;

/// Merges an Observable of Observables into a single Observable that emits all of their emissions.
/// See <https://reactivex.io/documentation/operators/merge.html> (referencing merge operator for general concept)
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt,
///     observer::Termination,
///     operators::{
///         combining::merge_all::MergeAll,
///         creating::from_iter::FromIter,
///     },
/// };
///
/// let mut values = Vec::new();
/// let mut terminations = Vec::new();
///
/// let observable = MergeAll::new_from_iter([
///     FromIter::new(vec![1, 3]),
///     FromIter::new(vec![2, 4]),
/// ]);
/// observable.subscribe_with_callback(
///     |value| values.push(value),
///     |termination| terminations.push(termination),
/// );
///
/// assert_eq!(values, vec![1, 3, 2, 4]);
/// assert_eq!(terminations, vec![Termination::Completed]);
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct MergeAll<OE, OE1> {
    source: OE,
    _marker: MarkerType<OE1>,
}

impl<OE, OE1> MergeAll<OE, OE1> {
    pub fn new<'or, T, E>(source: OE) -> Self
    where
        OE: Observable<'or, OE1, E>,
        OE1: Observable<'or, T, E>,
    {
        Self {
            source,
            _marker: PhantomData,
        }
    }
}

impl<E, OE1, I> MergeAll<MapInfallibleToError<E, FromIter<I>>, OE1> {
    pub fn new_from_iter<'or, T>(into_iterator: I) -> Self
    where
        I: IntoIterator<Item = OE1>,
        OE1: Observable<'or, T, E>,
    {
        Self {
            source: MapInfallibleToError::new(FromIter::new(into_iterator)),
            _marker: PhantomData,
        }
    }
}

delegate_disposal!(
    Disposal<'or>,
    subscribe_unsub_after_termination::Disposal<subscribe_with_shared_model::Disposal<'or>>
);

impl<'or, T, E, OE, OE1> Observable<'or, T, E> for MergeAll<OE, OE1>
where
    T: MaybeSend + 'or,
    E: MaybeSend + 'or,
    OE: Observable<'or, OE1, E>,
    OE::D: MaybeSend + 'or,
    OE1: Observable<'or, T, E>,
    OE1::D: MaybeSend + 'or,
{
    type D = Disposal<'or>;

    fn subscribe(self, observer: impl Observer<T, E> + MaybeSend + 'or) -> Subscription<Self::D> {
        subscribe_unsub_after_termination(observer, |observer| {
            let model = Model {
                subscriptions: SlotMap::new(),
                is_source_terminated: false,
            };
            subscribe_with_shared_model(observer, model, |context| {
                self.source.subscribe(MergeAllObserver(context))
            })
        })
        .map_into()
    }
}

struct Model<D: Disposable> {
    subscriptions: SlotMap<DefaultKey, Option<Subscription<D>>>,
    is_source_terminated: bool,
}

struct MergeAllObserver<T, E, OR, D: Disposable>(Context<T, E, OR, Model<D>>);

impl<'or, T, E, OR, OE1> Observer<OE1, E> for MergeAllObserver<T, E, OR, OE1::D>
where
    T: MaybeSend + 'or,
    E: MaybeSend + 'or,
    OR: Observer<T, E> + MaybeSend + 'or,
    OE1: Observable<'or, T, E>,
    OE1::D: MaybeSend + 'or,
{
    fn on_next(&mut self, value: OE1) {
        // Insert a placeholder subscription.
        let result = self.0.modify_model(|model| {
            ModificationResult::new(model.subscriptions.insert(None)).ignore_drop_outside()
        });
        let key = match result {
            Ok(key) => key,
            Err(_) => return,
        };
        let observer = MergeAllInnerObserver {
            context: self.0.clone(),
            key,
        };
        let sub = value.subscribe(observer);

        let _ = self.0.modify_model(|model| {
            if model.subscriptions.contains_key(key) {
                model.subscriptions[key] = Some(sub);
                ModificationResult::new_without_result()
            } else {
                // already terminated
                ModificationResult::new_without_result().drop_outside(sub)
            }
        });
    }

    fn on_termination(self, termination: Termination<E>) {
        match termination {
            Termination::Completed => {
                let _ = self.0.modify_model(|model| {
                    if model.subscriptions.is_empty() {
                        ModificationResult::new_send_termination(termination)
                    } else {
                        model.is_source_terminated = true;
                        ModificationResult::new_without_result()
                    }
                });
            }
            Termination::Error(_) => {
                self.0.send_termination(termination);
            }
        }
    }
}

struct MergeAllInnerObserver<T, E, OR, D: Disposable> {
    context: Context<T, E, OR, Model<D>>,
    key: DefaultKey,
}

impl<T, E, OR, D> Observer<T, E> for MergeAllInnerObserver<T, E, OR, D>
where
    OR: Observer<T, E>,
    D: Disposable,
{
    fn on_next(&mut self, value: T) {
        self.context.send_next(value);
    }

    fn on_termination(self, termination: Termination<E>) {
        match termination {
            Termination::Completed => {
                let _ = self.context.modify_model(|model| {
                    let subscription = model.subscriptions.remove(self.key);
                    if model.is_source_terminated && model.subscriptions.is_empty() {
                        ModificationResult::new_without_result()
                            .drop_outside(subscription)
                            .send_termination(termination)
                    } else {
                        ModificationResult::new_without_result().drop_outside(subscription)
                    }
                });
            }
            Termination::Error(_) => {
                self.context.send_termination(termination);
            }
        }
    }
}
