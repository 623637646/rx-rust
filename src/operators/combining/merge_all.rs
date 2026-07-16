use crate::disposable::Disposable;
use crate::operators::others::with_error_type::WithErrorType;
use crate::utils::subscribe_with_context::{
    BoundDisposal, Context, ModificationResult, subscribe_with_context_bound_disposal,
};
use crate::utils::types::MaybeSend;
use crate::{
    observable::{Observable, Subscription},
    observer::{Observer, Termination},
    operators::creating::from_iter::FromIter,
    utils::types::MarkerType,
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

impl<E, OE1, I> MergeAll<WithErrorType<E, FromIter<I>>, OE1> {
    pub fn new_from_iter<'or, T>(into_iterator: I) -> Self
    where
        I: IntoIterator<Item = OE1>,
        OE1: Observable<'or, T, E>,
    {
        Self {
            source: WithErrorType::new(FromIter::new(into_iterator)),
            _marker: PhantomData,
        }
    }
}

impl<'or, T, E, OE, OE1> Observable<'or, T, E> for MergeAll<OE, OE1>
where
    T: MaybeSend + 'or,
    E: MaybeSend + 'or,
    OE: Observable<'or, OE1, E>,
    OE::D: MaybeSend + 'or,
    OE1: Observable<'or, T, E>,
    OE1::D: MaybeSend + 'or,
{
    type D = BoundDisposal<'or>;

    fn subscribe(self, observer: impl Observer<T, E> + MaybeSend + 'or) -> Subscription<Self::D> {
        let model = Model {
            subscriptions: SlotMap::new(),
            is_source_terminated: false,
        };
        subscribe_with_context_bound_disposal(observer, model, |context| {
            self.source.subscribe(MergeAllObserver(context))
        })
    }
}

struct Model<D: Disposable> {
    subscriptions: SlotMap<DefaultKey, Option<Subscription<D>>>,
    is_source_terminated: bool,
}

struct MergeAllObserver<T, E, OR, ID: Disposable, SD: Disposable>(Context<T, E, OR, Model<ID>, SD>);

impl<'or, T, E, OR, OE1, SD> Observer<OE1, E> for MergeAllObserver<T, E, OR, OE1::D, SD>
where
    T: MaybeSend + 'or,
    E: MaybeSend + 'or,
    OR: Observer<T, E> + MaybeSend + 'or,
    OE1: Observable<'or, T, E>,
    OE1::D: MaybeSend + 'or,
    SD: Disposable + MaybeSend + 'or,
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

struct MergeAllInnerObserver<T, E, OR, ID: Disposable, SD: Disposable> {
    context: Context<T, E, OR, Model<ID>, SD>,
    key: DefaultKey,
}

impl<T, E, OR, ID, SD> Observer<T, E> for MergeAllInnerObserver<T, E, OR, ID, SD>
where
    OR: Observer<T, E>,
    ID: Disposable,
    SD: Disposable,
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
