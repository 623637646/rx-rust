use crate::utils::subscribe_unsub_after_termination::subscribe_unsub_after_termination;
use crate::utils::subscribe_with_shared_model::{
    Context, ModificationResult, subscribe_with_shared_model,
};
use crate::utils::types::{MarkerType, MaybeSend};
use crate::{
    disposable::subscription::Subscription,
    observable::Observable,
    observer::{Observer, Termination},
};
use educe::Educe;
use std::collections::VecDeque;
use std::marker::PhantomData;

/// Emits a single boolean value that indicates whether two Observables emit the same sequence of items.
/// See <https://reactivex.io/documentation/operators/sequenceequal.html>
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::observable_ext::ObservableExt,
///     observer::Termination,
///     operators::{
///         conditional_boolean::sequence_equal::SequenceEqual,
///         creating::from_iter::FromIter,
///     },
/// };
///
/// let mut values = Vec::new();
/// let mut terminations = Vec::new();
///
/// let observable = SequenceEqual::new(
///     FromIter::new(vec![1, 2]),
///     FromIter::new(vec![1, 2]),
/// );
/// observable.subscribe_with_callback(
///     |value| values.push(value),
///     |termination| terminations.push(termination),
/// );
///
/// assert_eq!(values, vec![true]);
/// assert_eq!(terminations, vec![Termination::Completed]);
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct SequenceEqual<T, OE1, OE2> {
    source_1: OE1,
    source_2: OE2,
    _marker: MarkerType<T>,
}

impl<T, OE1, OE2> SequenceEqual<T, OE1, OE2> {
    pub fn new<'or, 'sub, E>(source_1: OE1, source_2: OE2) -> Self
    where
        OE1: Observable<'or, 'sub, T, E>,
        OE2: Observable<'or, 'sub, T, E>,
    {
        Self {
            source_1,
            source_2,
            _marker: PhantomData,
        }
    }
}

impl<'or, 'sub, T, E, OE1, OE2> Observable<'or, 'sub, bool, E> for SequenceEqual<T, OE1, OE2>
where
    'sub: 'or,
    'or: 'sub,
    T: PartialEq + MaybeSend + 'or,
    E: MaybeSend + 'or,
    OE1: Observable<'or, 'sub, T, E>,
    OE2: Observable<'or, 'sub, T, E>,
{
    fn subscribe(self, observer: impl Observer<bool, E> + MaybeSend + 'or) -> Subscription<'sub> {
        subscribe_unsub_after_termination(observer, |observer| {
            let model = Model {
                first: SourceState {
                    queue: VecDeque::new(),
                    completed: false,
                },
                second: SourceState {
                    queue: VecDeque::new(),
                    completed: false,
                },
            };
            subscribe_with_shared_model(observer, model, |context| {
                let observer_1 = SequenceEqualObserver {
                    context: context.clone(),
                    is_first: true,
                };
                let observer_2 = SequenceEqualObserver {
                    context,
                    is_first: false,
                };
                let subscription_1 = self.source_1.subscribe(observer_1);
                let subscription_2 = self.source_2.subscribe(observer_2);
                subscription_1 + subscription_2
            })
        })
    }
}

struct SourceState<T> {
    queue: VecDeque<T>,
    completed: bool,
}

struct Model<T> {
    first: SourceState<T>,
    second: SourceState<T>,
}

struct SequenceEqualObserver<T, E, OR> {
    context: Context<bool, E, OR, Model<T>>,
    is_first: bool,
}

impl<T, E, OR> Observer<T, E> for SequenceEqualObserver<T, E, OR>
where
    OR: Observer<bool, E>,
    T: PartialEq,
{
    fn on_next(&mut self, value: T) {
        let _ = self.context.modify_model(|model| {
            let (mine, other) = if self.is_first {
                (&mut model.first, &mut model.second)
            } else {
                (&mut model.second, &mut model.first)
            };

            match (other.queue.pop_front(), other.completed) {
                (None, true) => {
                    ModificationResult::new_send_next_and_termination(false, Termination::Completed)
                }
                (None, false) => {
                    mine.queue.push_back(value);
                    ModificationResult::new_without_result()
                }
                (Some(next), _) => {
                    if value == next {
                        ModificationResult::new_without_result()
                    } else {
                        ModificationResult::new_send_next_and_termination(
                            false,
                            Termination::Completed,
                        )
                    }
                }
            }
        });
    }

    fn on_termination(self, termination: Termination<E>) {
        match termination {
            Termination::Completed => {
                let _ = self.context.modify_model(|model| {
                    let (mine, other) = if self.is_first {
                        (&mut model.first, &mut model.second)
                    } else {
                        (&mut model.second, &mut model.first)
                    };
                    mine.completed = true;

                    let mine_empty = mine.queue.is_empty();
                    let other_completed = other.completed;
                    let other_empty = other.queue.is_empty();

                    if !other_completed && other_empty {
                        ModificationResult::new_without_result()
                    } else {
                        let is_equal = mine_empty && other_completed && other_empty;
                        ModificationResult::new_send_next_and_termination(is_equal, termination)
                    }
                });
            }
            Termination::Error(_) => self.context.send_termination(termination),
        };
    }
}
