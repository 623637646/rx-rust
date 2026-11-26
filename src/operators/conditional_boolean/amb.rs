//! Mirrors whichever source emits or terminates first.
//!
//! The [`Amb`] operator, behind
//! [`ObservableExt::amb`](crate::observable::ObservableExt::amb).

use crate::{
    disposable::{Disposable, dispose_on_drop::DisposeOnDrop},
    observable::{
        IterSources, Observable, ObservableTypes, SameItemSources, SourceSet,
        sources::{InternalSourceSet, SourceSubscriber, SubscribeEach},
    },
    observer::{Flow, Observer, Termination},
    thread_mode::{
        ThreadMode,
        mutable::{MutableExt, MutableHelper},
    },
    utils::positional::{position::Position, slots::ReservationSlots},
};
use educe::Educe;
use std::ops::ControlFlow;

/// Mirrors the first source observable to emit an item or a termination, and disposes the others.
/// See <https://reactivex.io/documentation/operators/amb.html>
///
/// The sources are a tuple of one to eight observables of one item type and one error type, which
/// may be observables of different types: no boxing is needed to race them.
/// [`ObservableExt::amb`](crate::observable::ObservableExt::amb) is the form for two;
/// [`Amb::new_from_iter`] is the one for any number of observables of one type, decided as the
/// program runs.
///
/// The sources are subscribed in order. One that wins as it is subscribed keeps the ones after it
/// from being subscribed at all. Once a source has won, its items and its termination go straight
/// to the observer, with no lock of the operator's on the way.
///
/// Its iterator may even be infinite if a source wins synchronously: sources are generated only
/// until the race is decided.
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt,
///     observer::{Observer, Termination},
///     operators::conditional_boolean::amb::Amb,
///     subject::publish_subject::PublishSubject,
/// };
/// use std::convert::Infallible;
///
/// let mut values = Vec::new();
/// let mut cache = PublishSubject::<_, Infallible, _>::local();
/// let mut network = PublishSubject::local();
///
/// let subscription = Amb::new((cache.clone(), network.clone().map(|value| value * 10)))
///     .subscribe_with_callback(
///         |value| values.push(value),
///         |termination| assert_eq!(termination, Termination::Completed),
///     );
///
/// let _ = network.on_next(1); // The network answers first: it wins.
/// let _ = cache.on_next(2); // Disposed: not seen.
/// network.on_termination(Termination::Completed);
///
/// drop((subscription, cache));
/// assert_eq!(values, [10]);
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct Amb<S> {
    sources: S,
}

impl<S> Amb<S> {
    /// Creates an [`Amb`] racing the observables of the tuple `sources`;
    /// [`ObservableExt::amb`](crate::observable::ObservableExt::amb) is the fluent form
    /// for two.
    pub fn new(sources: S) -> Self
    where
        S: SameItemSources,
    {
        Self { sources }
    }
}

impl<I> Amb<IterSources<I>> {
    /// Races the sources of an iterator, pulling them only while no source has won.
    ///
    /// An empty iterator completes immediately. A synchronous winner prevents later sources
    /// from even being pulled from the iterator.
    pub fn new_from_iter<OE>(sources: I) -> Self
    where
        I: IntoIterator<Item = OE>,
        OE: ObservableTypes,
    {
        Self {
            sources: IterSources(sources),
        }
    }
}

impl<T, S> ObservableTypes for Amb<S>
where
    S: SameItemSources<SameItem = T>,
{
    type Item = T;
    type Error = S::Error;
    type Mode = S::Mode;
    type Disposal = Disposal<S::Mode, S::SubscriptionSlots>;
}

/// The [`Registration`] of an [`Amb`] over `S`, subscribed with `OR`.
type RegistrationOf<S, OR> =
    Registration<<S as SourceSet>::Mode, <S as SourceSet>::SubscriptionSlots, OR>;

impl<S, OR> Observable<OR> for Amb<S>
where
    S: SameItemSources + InternalSourceSet,
    <S as SourceSet>::Cursor: SubscribeEach<RegistrationOf<S, OR>, ()>,
    OR: Observer<<S as SameItemSources>::SameItem, <S as SourceSet>::Error>,
{
    fn subscribe(self, observer: OR) -> DisposeOnDrop<Self::Disposal> {
        let mut registration = Registration {
            observer: <S as SourceSet>::Mode::ptr(Some(observer)),
            context: <S as SourceSet>::Mode::ptr(AmbState {
                race: Race::Racing,
                slots: Default::default(),
            }),
            subscribed: false,
        };
        // An iterator is pulled only while the race is open; the cursor, with the sources left,
        // is dropped at the end of the statement.
        let exhausted = self
            .sources
            .into_cursor()
            .subscribe_each(&mut registration)
            .is_continue();
        if exhausted && !registration.subscribed {
            // An empty iterator: no source can end the race.
            registration
                .observer
                .take_value()
                .expect("an empty race retains its observer")
                .on_termination(Termination::Completed);
        }
        DisposeOnDrop::new(Disposal(registration.context))
    }
}

/// The shared race state, with the subscription of each source, stored as soon as it is made.
struct AmbState<SL> {
    race: Race,
    /// Every subscription stored while racing, the winner's alone once won, none once stopped.
    slots: SL,
}

#[derive(Clone, Copy)]
enum Race {
    Racing,
    /// Won by the source at this position.
    Won(usize),
    Stopped,
}

impl Race {
    /// Whether the subscription of the source at `position` is still wanted: every one while
    /// racing, the winner's alone once won.
    fn keeps(self, position: usize) -> bool {
        match self {
            Self::Racing => true,
            Self::Won(winner) => winner == position,
            Self::Stopped => false,
        }
    }

    /// Goes on subscribing the next source only while the race is open.
    fn flow(self) -> ControlFlow<()> {
        match self {
            Self::Racing => ControlFlow::Continue(()),
            Self::Won(_) | Self::Stopped => ControlFlow::Break(()),
        }
    }
}

/// Subscribes the sources one at a time while the race is open, and stores each subscription in
/// the race state as soon as it is made, so that a win disposes every loser subscribed so far.
/// It breaks, with nothing to say, once the race is decided.
struct Registration<M: ThreadMode, SL, OR> {
    observer: M::Ptr<Option<OR>>,
    context: M::Ptr<AmbState<SL>>,
    /// Whether a source was pulled to be subscribed: with none, an empty iterator, the race
    /// completes at once.
    subscribed: bool,
}

impl<M, SL, OR, K, OE> SourceSubscriber<K, OE, ()> for Registration<M, SL, OR>
where
    M: ThreadMode,
    K: Position,
    OE: ObservableTypes,
    SL: ReservationSlots<K, Value = DisposeOnDrop<<OE as ObservableTypes>::Disposal>>,
{
    type Observer = AmbObserver<M, SL, OR, K>;

    fn still_running(&self) -> ControlFlow<()> {
        // A source already subscribed may have won the race through `Iterator::next`.
        self.context.with_ref(|state| state.race.flow())
    }

    fn observer(&mut self, position: K) -> Option<Self::Observer> {
        self.subscribed = true;
        Some(AmbObserver(AmbObserverState::Racing {
            observer: self.observer.clone(),
            context: M::downgrade(&self.context),
            position,
        }))
    }

    fn store(
        &mut self,
        position: K,
        subscription: DisposeOnDrop<<OE as ObservableTypes>::Disposal>,
    ) -> ControlFlow<()> {
        // The race may have been decided meanwhile, by this source as it was subscribed or by
        // another one on another thread: only the winner's subscription is kept then.
        let (rejected, flow) = self.context.with_mut(|state| {
            let rejected = if state.race.keeps(position.position()) {
                state.slots.fill(position, subscription)
            } else {
                Some(subscription)
            };
            (rejected, state.race.flow())
        });
        drop(rejected); // Outside the lock.
        flow
    }
}

enum AmbObserverState<M: ThreadMode, SL, OR, K> {
    Racing {
        observer: M::Ptr<Option<OR>>,
        context: M::Weak<AmbState<SL>>,
        position: K,
    },
    Won(OR),
    Lost,
}

impl<M, SL, OR, K> AmbObserverState<M, SL, OR, K>
where
    M: ThreadMode,
    SL: ReservationSlots<K> + Default,
    K: Position,
{
    /// Settles a racing source: it wins if the race is still open, and loses otherwise. A source
    /// already settled stays as it is.
    fn settle(self) -> Self {
        let Self::Racing {
            observer,
            context,
            position,
        } = self
        else {
            return self;
        };
        let Some(context) = M::upgrade(&context) else {
            return Self::Lost;
        };
        let losers = context.with_mut(|state| {
            if !matches!(state.race, Race::Racing) {
                return None;
            }
            state.race = Race::Won(position.position());
            // Every subscription leaves with the losers but the winner's, which moves to fresh
            // slots. Not stored yet, it finds its position still reserved there.
            let mut losers = std::mem::take(&mut state.slots);
            if let Some(winner) = losers.release(position) {
                let late = state.slots.fill(position, winner);
                assert!(late.is_none(), "fresh slots hold no release");
            }
            Some(losers)
        });
        let Some(losers) = losers else {
            return Self::Lost;
        };
        // The `Racing` -> `Won` transition above elects a single winner, so the downstream
        // observer is taken after the context lock is released: no other source can reach this
        // point.
        let observer = observer
            .take_value()
            .expect("a racing amb must retain its downstream observer");
        drop(losers); // Outside the lock, the last subscribed first.
        Self::Won(observer)
    }
}

#[doc(hidden)]
pub struct AmbObserver<M: ThreadMode, SL, OR, K>(AmbObserverState<M, SL, OR, K>);

impl<M, T, E, SL, OR, K> Observer<T, E> for AmbObserver<M, SL, OR, K>
where
    M: ThreadMode,
    SL: ReservationSlots<K> + Default,
    K: Position,
    OR: Observer<T, E>,
{
    fn on_next(&mut self, value: T) -> Flow {
        if let AmbObserverState::Racing { .. } = self.0 {
            self.0 = std::mem::replace(&mut self.0, AmbObserverState::Lost).settle();
        }
        match &mut self.0 {
            AmbObserverState::Won(observer) => observer.on_next(value),
            // This source lost the race, so nothing it emits is wanted anymore.
            AmbObserverState::Lost | AmbObserverState::Racing { .. } => Flow::Break(()),
        }
    }

    fn on_termination(self, termination: Termination<E>) {
        if let AmbObserverState::Won(observer) = self.0.settle() {
            observer.on_termination(termination);
        }
    }
}

/// The disposal of an [`Amb`] subscription: it disposes every source still racing, or the winner.
#[derive(Educe)]
#[educe(Debug)]
pub struct Disposal<M: ThreadMode, SL>(#[educe(Debug(ignore))] M::Ptr<AmbState<SL>>);

impl<M, SL> Disposable for Disposal<M, SL>
where
    M: ThreadMode,
    SL: Default,
{
    fn dispose(self) {
        let slots = self.0.with_mut(|state| {
            state.race = Race::Stopped;
            std::mem::take(&mut state.slots)
        });
        drop(slots); // Outside the lock, the last subscribed first.
    }
}
