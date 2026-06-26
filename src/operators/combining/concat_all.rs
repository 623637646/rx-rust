use crate::disposable::subscription::Subscription;
use crate::observable::observable_ext::ObservableExt;
use crate::operators::others::map_infallible_to_error::MapInfallibleToError;
use crate::utils::subscribe_with_shared_model::{Action, Context, subscribe_with_shared_model};
use crate::utils::types::NecessarySend;
use crate::{
    observable::Observable,
    observer::{Observer, Termination},
    operators::creating::from_iter::FromIter,
    utils::{
        subscribe_unsub_after_termination::subscribe_unsub_after_termination, types::MarkerType,
    },
};
use educe::Educe;
use std::{collections::VecDeque, marker::PhantomData};

/// Concatenates an Observable of Observables, emitting all values from each inner Observable in sequence.
/// See <https://reactivex.io/documentation/operators/concat.html> (referencing concat operator for general concept)
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::observable_ext::ObservableExt,
///     observer::Termination,
///     operators::{
///         combining::concat_all::ConcatAll,
///         creating::from_iter::FromIter,
///     },
/// };
///
/// let mut values = Vec::new();
/// let mut terminations = Vec::new();
///
/// let observable = ConcatAll::new_from_iter([
///     FromIter::new(vec![1, 2]),
///     FromIter::new(vec![3, 4]),
/// ]);
/// observable.subscribe_with_callback(
///     |value| values.push(value),
///     |termination| terminations.push(termination),
/// );
///
/// assert_eq!(values, vec![1, 2, 3, 4]);
/// assert_eq!(terminations, vec![Termination::Completed]);
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct ConcatAll<OE, OE1> {
    source: OE,
    _marker: MarkerType<OE1>,
}

impl<OE, OE1> ConcatAll<OE, OE1> {
    pub fn new<'or, 'sub, T, E>(source: OE) -> Self
    where
        OE: Observable<'or, 'sub, OE1, E>,
        OE1: Observable<'or, 'sub, T, E>,
    {
        Self {
            source,
            _marker: PhantomData,
        }
    }
}

impl<E, OE1, I> ConcatAll<MapInfallibleToError<E, FromIter<I>>, OE1> {
    pub fn new_from_iter<'or, 'sub, T>(into_iterator: I) -> Self
    where
        I: IntoIterator<Item = OE1>,
        OE1: Observable<'or, 'sub, T, E>,
    {
        Self {
            source: FromIter::new(into_iterator).map_infallible_to_error(),
            _marker: PhantomData,
        }
    }
}

impl<'or, 'sub, T, E, OE, OE1> Observable<'or, 'sub, T, E> for ConcatAll<OE, OE1>
where
    'sub: 'or,
    'or: 'sub,
    T: NecessarySend + 'or,
    E: NecessarySend + 'or,
    OE: Observable<'or, 'sub, OE1, E>,
    OE1: Observable<'or, 'sub, T, E> + NecessarySend + 'or,
{
    fn subscribe(self, observer: impl Observer<T, E> + NecessarySend + 'or) -> Subscription<'sub> {
        subscribe_unsub_after_termination(observer, |observer| {
            let model = Model {
                pending_observables: VecDeque::new(),
                on_going_sub: None,
                is_source_completed: false,
                is_current_terminated: false,
            };
            subscribe_with_shared_model(observer, model, |context| {
                self.source.subscribe(SourceObserver(context.clone()))
            })
        })
    }
}

struct Model<'sub, OE1> {
    pending_observables: VecDeque<OE1>,
    on_going_sub: Option<Subscription<'sub>>,
    is_source_completed: bool,
    is_current_terminated: bool,
}

struct SourceObserver<'sub, T, E, OR, OE1>(Context<T, E, OR, Model<'sub, OE1>>);

impl<'or, 'sub, T, E, OR, OE1> Observer<OE1, E> for SourceObserver<'sub, T, E, OR, OE1>
where
    'sub: 'or,
    T: NecessarySend + 'or,
    E: NecessarySend + 'or,
    OR: Observer<T, E> + NecessarySend + 'or,
    OE1: Observable<'or, 'sub, T, E> + NecessarySend + 'or,
{
    fn on_next(&mut self, value: OE1) {
        let (observable, _subscription) = self.0.modify_model_with_action_and_result(|model| {
            let Some(model) = model else {
                return (Action::None, (None, None));
            };
            model.pending_observables.push_back(value);
            if model.on_going_sub.is_none() {
                process_next_observable(model)
            } else {
                (Action::None, (None, None))
            }
        });
        if let Some(observable) = observable {
            subscribe_next_observable(self.0.clone(), observable);
        }
    }

    fn on_termination(self, termination: Termination<E>) {
        match termination {
            Termination::Completed => {
                self.0.modify_model_with_action(|model| {
                    let Some(model) = model else {
                        return Action::None;
                    };
                    model.is_source_completed = true;
                    if model.on_going_sub.is_none() && model.pending_observables.is_empty() {
                        Action::SendTermination(termination)
                    } else {
                        Action::None
                    }
                });
            }
            Termination::Error(_) => {
                self.0.send_termination(termination);
            }
        }
    }
}

struct InnerObserver<'sub, T, E, OR, OE1>(Context<T, E, OR, Model<'sub, OE1>>);

impl<'or, 'sub, T, E, OR, OE1> Observer<T, E> for InnerObserver<'sub, T, E, OR, OE1>
where
    'sub: 'or,
    T: NecessarySend + 'or,
    E: NecessarySend + 'or,
    OR: Observer<T, E> + NecessarySend + 'or,
    OE1: Observable<'or, 'sub, T, E> + NecessarySend + 'or,
{
    fn on_next(&mut self, value: T) {
        self.0.send_next(value);
    }

    fn on_termination(self, termination: Termination<E>) {
        let (next_source, _subscription) = self.0.modify_model_with_action_and_result(|model| {
            let Some(model) = model else {
                return (Action::None, (None, None));
            };
            model.is_current_terminated = true;
            match termination {
                Termination::Completed => process_next_observable(model),
                Termination::Error(error) => (
                    Action::SendTermination(Termination::Error(error)),
                    (None, None),
                ),
            }
        });
        if let Some(observable) = next_source {
            subscribe_next_observable(self.0.clone(), observable);
        }
    }
}

fn process_next_observable<'sub, T, E, OE1>(
    model: &mut Model<'sub, OE1>,
) -> (Action<T, E>, (Option<OE1>, Option<Subscription<'sub>>)) {
    if let Some(observable) = model.pending_observables.pop_front() {
        model.is_current_terminated = false;
        (Action::None, (Some(observable), None))
    } else if model.is_source_completed {
        (
            Action::SendTermination(Termination::Completed),
            (None, None),
        )
    } else {
        let on_going_sub = model.on_going_sub.take(); // Drop subscription outside the lock to avoid potential deadlock
        (Action::None, (None, on_going_sub))
    }
}

fn subscribe_next_observable<'or, 'sub, T, E, OR, OE1>(
    context: Context<T, E, OR, Model<'sub, OE1>>,
    observable: OE1,
) where
    'sub: 'or,
    T: NecessarySend + 'or,
    E: NecessarySend + 'or,
    OR: Observer<T, E> + NecessarySend + 'or,
    OE1: Observable<'or, 'sub, T, E> + NecessarySend + 'or,
{
    let observer = InnerObserver(context.clone());
    let sub = observable.subscribe(observer);
    let _sub = context.modify_model(|model| {
        let model = model?;
        if !model.is_current_terminated {
            Some(model.on_going_sub.replace(sub)) // Drop subscription outside the lock to avoid potential deadlock 
        } else {
            None
        }
    });
}
