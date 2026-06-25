use crate::utils::subscribe_with_shared_model::{Action, Context, subscribe_with_shared_model};
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
    T1: Clone + NecessarySend + 'sub,
    T2: Clone + NecessarySend + 'sub,
    E: NecessarySend + 'sub,
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
                let sub_1 = self.source_1.subscribe(ObserverImpl1(context.clone()));
                let sub_2 = self.source_2.subscribe(ObserverImpl2(context));
                sub_1 + sub_2
            })
        })
    }
}

struct Model<T1, T2> {
    latest_1: Option<T1>,
    latest_2: Option<T2>,
    should_completed: bool,
}

struct ObserverImpl1<T1, T2, E, OR>(Context<(T1, T2), E, OR, Model<T1, T2>>);

impl<T1, T2, E, OR> Observer<T1, E> for ObserverImpl1<T1, T2, E, OR>
where
    OR: Observer<(T1, T2), E>,
    T1: Clone,
    T2: Clone,
{
    fn on_next(&mut self, latest_1: T1) {
        self.0.lock_model(|model| {
            if let Some(latest_2) = &model.latest_2 {
                model.latest_1 = Some(latest_1.clone());
                Action::Next((latest_1, latest_2.clone()))
            } else {
                model.latest_1 = Some(latest_1);
                Action::None
            }
        });
    }

    fn on_termination(self, termination: Termination<E>) {
        self.0.lock_model(|model| match termination {
            Termination::Completed => {
                if model.should_completed || model.latest_1.is_none() {
                    Action::Termination(Termination::Completed)
                } else {
                    model.should_completed = true;
                    Action::None
                }
            }
            Termination::Error(error) => Action::Termination(Termination::Error(error)),
        });
    }
}

struct ObserverImpl2<T1, T2, E, OR>(Context<(T1, T2), E, OR, Model<T1, T2>>);

impl<T1, T2, E, OR> Observer<T2, E> for ObserverImpl2<T1, T2, E, OR>
where
    OR: Observer<(T1, T2), E>,
    T1: Clone,
    T2: Clone,
{
    fn on_next(&mut self, latest_2: T2) {
        self.0.lock_model(|model| {
            if let Some(latest_1) = &model.latest_1 {
                model.latest_2 = Some(latest_2.clone());
                Action::Next((latest_1.clone(), latest_2))
            } else {
                model.latest_2 = Some(latest_2);
                Action::None
            }
        });
    }

    fn on_termination(self, termination: Termination<E>) {
        self.0.lock_model(|model| match termination {
            Termination::Completed => {
                if model.should_completed || model.latest_2.is_none() {
                    Action::Termination(Termination::Completed)
                } else {
                    model.should_completed = true;
                    Action::None
                }
            }
            Termination::Error(error) => Action::Termination(Termination::Error(error)),
        });
    }
}
