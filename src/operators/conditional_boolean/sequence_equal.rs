use crate::safe_lock_option_observer;
use crate::utils::types::{MarkerType, Mutable, MutableHelper, NecessarySend, Shared};
use crate::utils::unsub_after_termination::subscribe_unsub_after_termination;
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
    T: PartialEq + NecessarySend + 'or,
    OE1: Observable<'or, 'sub, T, E>,
    OE2: Observable<'or, 'sub, T, E>,
    'sub: 'or,
{
    fn subscribe(
        self,
        observer: impl Observer<bool, E> + NecessarySend + 'or,
    ) -> Subscription<'sub> {
        subscribe_unsub_after_termination(observer, |observer| {
            let observer = Shared::new(Mutable::new(Some(observer)));
            let state = Shared::new(Mutable::new(SequenceEqualObserverState::None(false)));
            let observer_1 = SequenceEqualObserver {
                observer: observer.clone(),
                state: state.clone(),
                is_one: true,
            };
            let observer_2 = SequenceEqualObserver {
                observer: observer.clone(),
                state: state.clone(),
                is_one: false,
            };
            let subscription_1 = self.source_1.subscribe(observer_1);
            let subscription_2 = self.source_2.subscribe(observer_2);
            subscription_1 + subscription_2
        })
    }
}

enum SequenceEqualObserverState<T> {
    None(bool),
    One(VecDeque<T>, bool),
    Two(VecDeque<T>, bool),
}

struct SequenceEqualObserver<T, OR> {
    observer: Shared<Mutable<Option<OR>>>,
    state: Shared<Mutable<SequenceEqualObserverState<T>>>,
    is_one: bool,
}

impl<T, E, OR> Observer<T, E> for SequenceEqualObserver<T, OR>
where
    OR: Observer<bool, E>,
    T: PartialEq,
{
    fn on_next(&mut self, value: T) {
        self.state.lock_mut(|mut lock| match &mut *lock {
            SequenceEqualObserverState::None(is_completed) => {
                if *is_completed {
                    safe_lock_option_observer!(on_next_and_termination: self.observer, false, Termination::Completed);
                } else if self.is_one {
                    *lock = SequenceEqualObserverState::One(VecDeque::from([value]), false);
                } else {
                    *lock = SequenceEqualObserverState::Two(VecDeque::from([value]), false);
                }
            }
            SequenceEqualObserverState::One(values, is_completed) => {
                if self.is_one {
                    assert!(!*is_completed);
                    values.push_back(value);
                } else {
                    let top = values.pop_front().unwrap();
                    if top == value {
                        if values.is_empty() {
                            *lock = SequenceEqualObserverState::None(*is_completed);
                        }
                    } else {
                        drop(lock);
                        safe_lock_option_observer!(on_next_and_termination: self.observer, false, Termination::Completed);
                    }
                }
            }
            SequenceEqualObserverState::Two(values, is_completed) => {
                if !self.is_one {
                    assert!(!*is_completed);
                    values.push_back(value);
                } else {
                    let top = values.pop_front().unwrap();
                    if top == value {
                        if values.is_empty() {
                            *lock = SequenceEqualObserverState::None(*is_completed);
                        }
                    } else {
                        drop(lock);
                        safe_lock_option_observer!(on_next_and_termination: self.observer, false, Termination::Completed);
                    }
                }
            }
        });
    }

    fn on_termination(self, termination: Termination<E>) {
        match termination {
            Termination::Completed => {
                self.state.lock_mut(|mut lock| match &mut *lock {
                    SequenceEqualObserverState::None(is_completed) => {
                        if *is_completed {
                            safe_lock_option_observer!(on_next_and_termination: self.observer, true, Termination::Completed);
                        } else {
                            *is_completed = true;
                        }
                    }
                    SequenceEqualObserverState::One(_, is_completed) => {
                        if *is_completed {
                            safe_lock_option_observer!(on_next_and_termination: self.observer, false, Termination::Completed);
                        } else if self.is_one {
                            *is_completed = true;
                        } else {
                            safe_lock_option_observer!(on_next_and_termination: self.observer, false, Termination::Completed);
                        }
                    }
                    SequenceEqualObserverState::Two(_, is_completed) => {
                        if *is_completed {
                            safe_lock_option_observer!(on_next_and_termination: self.observer, false, Termination::Completed);
                        } else if !self.is_one {
                            *is_completed = true;
                        } else {
                            safe_lock_option_observer!(on_next_and_termination: self.observer, false, Termination::Completed);
                        }
                    }
                });
            }
            Termination::Error(_) => {
                safe_lock_option_observer!(on_termination: self.observer, termination);
            }
        }
    }
}
