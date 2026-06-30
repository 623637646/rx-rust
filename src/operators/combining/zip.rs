use crate::utils::subscribe_with_shared_model::{
    Context, EventGroup, ModificationResult, subscribe_with_shared_model,
};
use crate::utils::types::NecessarySend;
use crate::{
    disposable::subscription::Subscription,
    observable::Observable,
    observer::{Observer, Termination},
    utils::subscribe_unsub_after_termination::subscribe_unsub_after_termination,
};
use educe::Educe;
use std::collections::VecDeque;

/// Combines the emissions of multiple Observables together via a specified function and emits single items for each combination based on the sequence of their emissions.
/// See <https://reactivex.io/documentation/operators/zip.html>
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::observable_ext::ObservableExt,
///     observer::Termination,
///     operators::{
///         combining::zip::Zip,
///         creating::from_iter::FromIter,
///     },
/// };
///
/// let mut values = Vec::new();
/// let mut terminations = Vec::new();
///
/// let observable = Zip::new(
///     FromIter::new(vec![1, 2]),
///     FromIter::new(vec![10, 20]),
/// );
/// observable.subscribe_with_callback(
///     |value| values.push(value),
///     |termination| terminations.push(termination),
/// );
///
/// assert_eq!(values, vec![(1, 10), (2, 20)]);
/// assert_eq!(terminations, vec![Termination::Completed]);
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct Zip<OE1, OE2> {
    source_1: OE1,
    source_2: OE2,
}

impl<OE1, OE2> Zip<OE1, OE2> {
    pub fn new<'or, 'sub, T1, T2, E>(source_1: OE1, source_2: OE2) -> Self
    where
        OE1: Observable<'or, 'sub, T1, E>,
        OE2: Observable<'or, 'sub, T2, E>,
    {
        Self { source_1, source_2 }
    }
}

impl<'or, 'sub, T1, T2, E, OE1, OE2> Observable<'or, 'sub, (T1, T2), E> for Zip<OE1, OE2>
where
    'sub: 'or,
    'or: 'sub,
    T1: NecessarySend + 'or,
    T2: NecessarySend + 'or,
    E: NecessarySend + 'or,
    OE1: Observable<'or, 'sub, T1, E>,
    OE2: Observable<'or, 'sub, T2, E>,
{
    fn subscribe(
        self,
        observer: impl Observer<(T1, T2), E> + NecessarySend + 'or,
    ) -> Subscription<'sub> {
        subscribe_unsub_after_termination(observer, |observer| {
            let model = Model {
                first: (VecDeque::new(), false),
                second: (VecDeque::new(), false),
            };
            subscribe_with_shared_model(observer, model, |context| {
                let subscription_1 = self.source_1.subscribe(ZipObserver1(context.clone()));
                let subscription_2 = self.source_2.subscribe(ZipObserver2(context));
                subscription_1 + subscription_2
            })
        })
    }
}

struct Model<T1, T2> {
    first: (VecDeque<T1>, bool),  // bool means completed
    second: (VecDeque<T2>, bool), // bool means completed
}

struct ZipObserver1<T1, T2, E, OR>(Context<(T1, T2), E, OR, Model<T1, T2>>);

impl<T1, T2, E, OR> Observer<T1, E> for ZipObserver1<T1, T2, E, OR>
where
    OR: Observer<(T1, T2), E>,
{
    fn on_next(&mut self, value: T1) {
        let _ = self.0.modify_model(|model| {
            let second = model.second.0.pop_front();
            if let Some(second) = second {
                if model.second.1 && model.second.0.is_empty() {
                    ModificationResult::default().send_events(EventGroup::NextAndTermination(
                        (value, second),
                        Termination::Completed,
                    ))
                } else {
                    ModificationResult::new_send_next((value, second))
                }
            } else {
                model.first.0.push_back(value);
                ModificationResult::default()
            }
        });
    }

    fn on_termination(self, termination: Termination<E>) {
        match termination {
            Termination::Completed => {
                let _ = self.0.modify_model(|model| {
                    model.first.1 = true;
                    if model.first.0.is_empty() {
                        ModificationResult::new_send_termination(termination)
                    } else {
                        ModificationResult::default()
                    }
                });
            }
            Termination::Error(_) => {
                self.0.send_termination(termination);
            }
        };
    }
}

struct ZipObserver2<T1, T2, E, OR>(Context<(T1, T2), E, OR, Model<T1, T2>>);

impl<T1, T2, E, OR> Observer<T2, E> for ZipObserver2<T1, T2, E, OR>
where
    OR: Observer<(T1, T2), E>,
{
    fn on_next(&mut self, value: T2) {
        let _ = self.0.modify_model(|model| {
            let first = model.first.0.pop_front();
            if let Some(first) = first {
                if model.first.1 && model.first.0.is_empty() {
                    ModificationResult::default().send_events(EventGroup::NextAndTermination(
                        (first, value),
                        Termination::Completed,
                    ))
                } else {
                    ModificationResult::new_send_next((first, value))
                }
            } else {
                model.second.0.push_back(value);
                ModificationResult::default()
            }
        });
    }

    fn on_termination(self, termination: Termination<E>) {
        match termination {
            Termination::Completed => {
                let _ = self.0.modify_model(|model| {
                    model.second.1 = true;
                    if model.second.0.is_empty() {
                        ModificationResult::new_send_termination(termination)
                    } else {
                        ModificationResult::default()
                    }
                });
            }
            Termination::Error(_) => {
                self.0.send_termination(termination);
            }
        };
    }
}
