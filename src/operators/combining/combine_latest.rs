use crate::observable::observable_ext::ObservableExt;
use crate::utils::subscribe_with_shared_model::{
    Context, SharedModel, subscribe_with_shared_model,
};
use crate::utils::types::NecessarySend;
use crate::{
    disposable::subscription::Subscription,
    observable::Observable,
    observer::{Observer, Termination},
    utils::subscribe_unsub_after_termination::subscribe_unsub_after_termination,
};
use educe::Educe;

/// Combines multiple Observables to create an Observable whose values are calculated from the latest values of each of its input Observables.
/// See <https://reactivex.io/documentation/operators/combinelatest.html>
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::observable_ext::ObservableExt,
///     observer::{Observer, Termination},
///     operators::combining::combine_latest::CombineLatest,
///     subject::behavior_subject::BehaviorSubject,
/// };
/// use std::convert::Infallible;
///
/// let mut values = Vec::new();
/// let mut terminations = Vec::new();
///
/// let mut subject_1 = BehaviorSubject::<'_, i32, Infallible>::new(0);
/// let mut subject_2 = BehaviorSubject::<'_, i32, Infallible>::new(10);
///
/// let subscription =
///     CombineLatest::new(subject_1.clone(), subject_2.clone()).subscribe_with_callback(
///         |value| values.push(value),
///         |termination| terminations.push(termination),
///     );
///
/// subject_1.on_next(1);
/// subject_2.on_next(11);
/// subject_1.on_termination(Termination::Completed);
/// subject_2.on_termination(Termination::Completed);
/// drop(subscription);
///
/// assert_eq!(values, vec![(0, 10), (1, 10), (1, 11)]);
/// assert_eq!(terminations, vec![Termination::Completed]);
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct CombineLatest<OE1, OE2> {
    source_1: OE1,
    source_2: OE2,
}

impl<OE1, OE2> CombineLatest<OE1, OE2> {
    pub fn new<'or, 'sub, T1, T2, E>(source_1: OE1, source_2: OE2) -> Self
    where
        OE1: Observable<'or, 'sub, T1, E>,
        OE2: Observable<'or, 'sub, T2, E>,
    {
        Self { source_1, source_2 }
    }
}

impl<'or, 'sub, T1, T2, E, OE1, OE2> Observable<'or, 'sub, (T1, T2), E> for CombineLatest<OE1, OE2>
where
    'or: 'sub,
    'sub: 'or,
    T1: Clone + NecessarySend + 'or,
    T2: Clone + NecessarySend + 'or,
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
                latest_1: None,
                latest_2: None,
                should_completed: false,
            };
            subscribe_with_shared_model(observer, model, |context| {
                let sub_1 = self
                    .source_1
                    .map(NextEvent::First)
                    .subscribe(context.create_observer(true));
                let sub_2 = self
                    .source_2
                    .map(NextEvent::Second)
                    .subscribe(context.create_observer(false));
                sub_1 + sub_2
            })
        })
    }
}

enum NextEvent<T1, T2> {
    First(T1),
    Second(T2),
}

struct Model<T1, T2> {
    latest_1: Option<T1>,
    latest_2: Option<T2>,
    should_completed: bool,
}

impl<'or, T1, T2, E> SharedModel<'or, NextEvent<T1, T2>, (T1, T2), E, bool> for Model<T1, T2>
where
    T1: Clone,
    T2: Clone,
{
    fn on_next<OR>(context: Context<(T1, T2), E, OR, Self>, value: NextEvent<T1, T2>, _: &mut bool)
    where
        OR: Observer<(T1, T2), E> + NecessarySend + 'or,
    {
        context.lock_model(
            |model| match value {
                NextEvent::First(latest_1) => {
                    if let Some(latest_2) = &model.latest_2 {
                        model.latest_1 = Some(latest_1.clone());
                        Some((latest_1, latest_2.clone()))
                    } else {
                        model.latest_1 = Some(latest_1);
                        None
                    }
                }
                NextEvent::Second(latest_2) => {
                    if let Some(latest_1) = &model.latest_1 {
                        model.latest_2 = Some(latest_2.clone());
                        Some((latest_1.clone(), latest_2))
                    } else {
                        model.latest_2 = Some(latest_2);
                        None
                    }
                }
            },
            |action, context| {
                if let Some(next) = action {
                    context.send_next(next);
                }
            },
        );
    }

    fn on_termination<OR>(
        context: Context<(T1, T2), E, OR, Self>,
        termination: Termination<E>,
        is_first: bool,
    ) where
        OR: Observer<(T1, T2), E> + NecessarySend + 'or,
    {
        context.lock_model(
            |model| match termination {
                Termination::Completed => {
                    if model.should_completed
                        || (is_first && model.latest_1.is_none())
                        || (!is_first && model.latest_2.is_none())
                    {
                        Some(Termination::Completed)
                    } else {
                        model.should_completed = true;
                        None
                    }
                }
                Termination::Error(error) => Some(Termination::Error(error)),
            },
            |action, context| {
                if let Some(termination) = action {
                    context.send_termination(termination);
                }
            },
        );
    }
}
