use super::Subject;
use crate::delegate_disposal;
use crate::disposable::option_disposal::OptionDisposal;
use crate::disposable::{Disposable, DisposableExt};
use crate::observable::Subscription;
use crate::observer::Event;
use crate::utils::types::{MaybeSend, Mutable, MutableHelper, Shared};
use crate::{
    observable::Observable,
    observer::{Observer, Termination, boxed_observer::BoxedObserver},
};
use educe::Educe;
use slotmap::{DefaultKey, DenseSlotMap};

#[derive(Educe)]
#[educe(Debug, Default)]
enum State<'or, T, E> {
    #[educe(Default)]
    Idle(DenseSlotMap<DefaultKey, Option<BoxedObserver<'or, T, E>>>),
    Processing {
        slot_map: DenseSlotMap<DefaultKey, Option<BoxedObserver<'or, T, E>>>,
        events: Vec<Event<T, E>>, // TODO: Should use EventBatch
    },
    Terminated(Termination<E>),
}

/// Basic multicast subject that forwards events to all observers.
#[derive(Educe)]
#[educe(Debug, Clone, Default)]
pub struct PublishSubject<'or, T, E>(Shared<Mutable<State<'or, T, E>>>);

impl<T, E> PublishSubject<'_, T, E> {
    pub fn new() -> Self {
        Self(Shared::new(Mutable::new(State::Idle(DenseSlotMap::new()))))
    }
}

delegate_disposal!(
    Disposal<'or, T, E>,
    OptionDisposal<PublishSubjectDisposal<'or, T, E>>
);

impl<'or, T, E> Observable<'or, T, E> for PublishSubject<'or, T, E>
where
    T: MaybeSend,
    E: Clone + MaybeSend,
{
    type D = Disposal<'or, T, E>;

    fn subscribe(self, observer: impl Observer<T, E> + MaybeSend + 'or) -> Subscription<Self::D> {
        self.0.clone().lock_mut(|mut lock| match &mut *lock {
            State::Idle(observers) => {
                let key = observers.insert(Some(BoxedObserver::new(observer)));
                drop(lock);
                OptionDisposal::some(PublishSubjectDisposal { state: self.0, key })
                    .into_subscription()
            }
            State::Processing {
                slot_map,
                events: _,
            } => {
                let key = slot_map.insert(Some(BoxedObserver::new(observer)));
                drop(lock);
                OptionDisposal::some(PublishSubjectDisposal { state: self.0, key })
                    .into_subscription()
            }
            State::Terminated(termination) => {
                let termination = termination.clone();
                drop(lock);
                observer.on_termination(termination);
                OptionDisposal::none().into_subscription()
            }
        })
    }
}

impl<T, E> Observer<T, E> for PublishSubject<'_, T, E>
where
    T: Clone,
    E: Clone,
{
    fn on_next(&mut self, value: T) {
        self.0.clone().lock_mut(|mut lock| match &mut *lock {
            State::Idle(_) => {
                // Get SloptMap
                let mut dense_slot_map = match std::mem::replace(
                    &mut *lock,
                    State::Processing {
                        slot_map: DenseSlotMap::new(), // Placeholder
                        events: Vec::new(),
                    },
                ) {
                    State::Idle(dense_slot_map) => dense_slot_map,
                    State::Processing { .. } => unreachable!(),
                    State::Terminated(..) => unreachable!(),
                };

                // Get observers
                let mut observers: Vec<_> = dense_slot_map
                    .iter_mut()
                    .map(|value| (value.0, value.1.take().unwrap()))
                    .collect();

                // Set SloptMap
                match &mut *lock {
                    State::Idle(..) => unreachable!(),
                    State::Processing {
                        slot_map,
                        events: _,
                    } => *slot_map = dense_slot_map,
                    State::Terminated(..) => unreachable!(),
                }

                // Notify
                drop(lock);
                observers
                    .iter_mut()
                    .for_each(|observer| observer.1.on_next(value.clone()));

                let events = self.0.clone().lock_mut(|mut lock| {
                    // Get SloptMap and Events
                    let (mut slot_map, events) = match std::mem::replace(
                        &mut *lock,
                        State::Idle(DenseSlotMap::new()), // Placeholder,
                    ) {
                        State::Idle(..) => unreachable!(),
                        State::Processing { slot_map, events } => (slot_map, events),
                        State::Terminated(..) => unreachable!(),
                    };

                    // Set Observers
                    for (key, observer) in observers {
                        if slot_map.contains_key(key) {
                            slot_map[key] = Some(observer);
                        } else {
                            // already unsubscribed
                        }
                    }

                    // Set SloptMap
                    *lock = State::Idle(slot_map);

                    // Return
                    events
                });

                // Handle Events
                for event in events {
                    match event {
                        Event::Next(value) => {
                            self.on_next(value);
                        }
                        Event::Termination(termination) => {
                            self.clone().on_termination(termination);
                            break;
                        }
                    }
                }
            }
            State::Processing {
                slot_map: _,
                events,
            } => {
                events.push(Event::Next(value));
            }
            State::Terminated(_) => {
                // It's already terminated. Do nothing.
            }
        });
    }

    fn on_termination(self, termination: Termination<E>) {
        self.0.lock_mut(|mut lock| {
            match &mut *lock {
                State::Idle(_) => {
                    // Get SloptMap
                    let dense_slot_map =
                        match std::mem::replace(&mut *lock, State::Terminated(termination.clone()))
                        {
                            State::Idle(dense_slot_map) => dense_slot_map,
                            State::Processing { .. } => unreachable!(),
                            State::Terminated(..) => unreachable!(),
                        };
                    drop(lock);

                    // Notify
                    dense_slot_map.into_iter().for_each(|observer| {
                        observer.1.unwrap().on_termination(termination.clone())
                    });
                }
                State::Processing {
                    slot_map: _,
                    events,
                } => {
                    events.push(Event::Termination(termination));
                }
                State::Terminated(_) => {
                    // It's already terminated. Do nothing.
                }
            }
        });
    }
}

impl<'or, T, E> Subject<'or, T, E> for PublishSubject<'or, T, E>
where
    T: Clone + MaybeSend,
    E: Clone + MaybeSend,
{
    fn terminated(&self) -> Option<Termination<E>>
    where
        E: Clone,
    {
        self.0.lock_ref(|lock| match &*lock {
            State::Idle(_) => None,
            State::Processing { .. } => None,
            State::Terminated(termination) => Some(termination.clone()),
        })
    }
}

struct PublishSubjectDisposal<'or, T, E> {
    state: Shared<Mutable<State<'or, T, E>>>,
    key: DefaultKey,
}

impl<T, E> Disposable for PublishSubjectDisposal<'_, T, E> {
    fn dispose(self) {
        self.state.lock_mut(|mut lock| {
            match &mut *lock {
                State::Idle(observers) => {
                    observers.remove(self.key);
                }
                State::Processing {
                    slot_map,
                    events: _,
                } => {
                    slot_map.remove(self.key);
                }
                State::Terminated(_) => {
                    // Do nothing if it was already terminated
                }
            }
        });
    }
}
