//! Where an observer that subscribes again from its own termination waits while the operator is
//! still subscribing.
//!
//! [`Retry`](crate::operators::error_handling::retry::Retry) subscribes again after an error and
//! [`RepeatSource`](crate::operators::utility::repeat_source::RepeatSource) after a completion,
//! both from the termination of the previous subscription. A source that terminates from inside
//! its own `subscribe` would then nest the next subscription inside that call, one level deeper
//! for each round, until the stack overflows. The trampoline turns that recursion into a loop: a
//! termination that arrives while a `subscribe` made by the operator has not returned yet —
//! synchronously from that very call, or from another thread meanwhile — parks what the next round
//! needs (the observer, and the error for `retry`) and returns; [`Trampoline::run`] takes it once
//! the subscription returns and starts that round itself. A termination that arrives once no
//! subscription is being made finds the trampoline idle, and its round starts on the spot.

use crate::{
    thread_mode::{
        ThreadMode,
        mutable::{MutableExt, MutableHelper},
    },
    utils::on_panic::on_panic,
};
use educe::Educe;

/// Where the next round of a re-subscribing operator waits while the operator is still subscribing.
///
/// `P` is what [`park`](Self::park) keeps for the next round, the observer among it. There is one
/// observer at a time, so at most one round is ever parked. See the [module documentation](self).
#[derive(Educe)]
#[educe(Clone(bound()))]
pub(crate) struct Trampoline<M: ThreadMode, P>(M::Ptr<State<P>>);

/// The state of a [`Trampoline`].
enum State<P> {
    /// No subscription is being made.
    Idle,
    /// [`run`](Trampoline::run) is making subscriptions, and no termination has arrived since it
    /// took the last one.
    Subscribing,
    /// A termination arrived while a subscription was being made, and its round waits for it to
    /// return.
    Parked(P),
}

impl<M: ThreadMode, P> Default for Trampoline<M, P> {
    fn default() -> Self {
        Self(M::ptr(State::Idle))
    }
}

impl<M: ThreadMode, P> Trampoline<M, P> {
    /// Runs `subscribe`, then `start_round` on the round parked meanwhile, if any, and so on: a
    /// round that terminates while it is being subscribed parks again and is picked up by this
    /// loop, so the stack stays as deep as one subscription however many rounds in a row end
    /// synchronously.
    ///
    /// The trampoline stays subscribing from the first call to the last: between two of them, the
    /// observer is in this loop's hands, so no termination can arrive. A parked observer holds this
    /// trampoline, which then holds the observer back: if a call unwinds, the guard empties the
    /// trampoline so that the two do not keep each other alive.
    pub(crate) fn run(&self, subscribe: impl FnOnce(), start_round: impl Fn(P)) {
        let previous = self.0.replace_value(State::Subscribing);
        debug_assert!(
            matches!(previous, State::Idle),
            "one subscription is made at a time"
        );
        let _guard = on_panic(|| drop(self.0.replace_value(State::Idle)));
        subscribe();
        while let Some(parked) = self.take_parked() {
            start_round(parked);
        }
    }

    /// Takes the round parked during the call that just returned, staying subscribing for it, or
    /// goes idle if none was parked.
    fn take_parked(&self) -> Option<P> {
        self.0
            .with_mut(|state| match std::mem::replace(state, State::Idle) {
                State::Parked(parked) => {
                    *state = State::Subscribing;
                    Some(parked)
                }
                State::Subscribing => None,
                State::Idle => unreachable!("the trampoline was subscribing"),
            })
    }

    /// Parks the next round while a subscription is being made, or hands it back to be started at
    /// once.
    pub(crate) fn park(&self, next_round: P) -> Option<P> {
        self.0.with_mut(|state| match state {
            State::Subscribing => {
                *state = State::Parked(next_round);
                None
            }
            State::Idle => Some(next_round),
            State::Parked(..) => unreachable!("one observer at a time, so one round at a time"),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::Trampoline;
    use crate::thread_mode::Local;
    use std::cell::RefCell;

    #[test]
    fn test_park_when_idle() {
        let trampoline = Trampoline::<Local, i32>::default();
        assert_eq!(trampoline.park(1), Some(1));
    }

    #[test]
    fn test_run_without_park() {
        let trampoline = Trampoline::<Local, i32>::default();
        let started = RefCell::new(Vec::new());
        trampoline.run(|| {}, |round| started.borrow_mut().push(round));
        assert!(started.borrow().is_empty());
        // The run left the trampoline idle.
        assert_eq!(trampoline.park(1), Some(1));
    }

    #[test]
    fn test_run_starts_parked_rounds_in_a_loop() {
        let trampoline = Trampoline::<Local, i32>::default();
        let started = RefCell::new(Vec::new());
        trampoline.run(
            || assert_eq!(trampoline.park(0), None),
            |round| {
                started.borrow_mut().push(round);
                if round < 3 {
                    assert_eq!(trampoline.park(round + 1), None);
                }
            },
        );
        assert_eq!(*started.borrow(), [0, 1, 2, 3]);
        assert_eq!(trampoline.park(4), Some(4));
    }

    #[cfg(panic = "unwind")]
    #[test]
    fn test_unwind_empties_the_trampoline() {
        use std::panic::{AssertUnwindSafe, catch_unwind};

        let trampoline = Trampoline::<Local, i32>::default();
        let result = catch_unwind(AssertUnwindSafe(|| {
            trampoline.run(
                || {
                    assert_eq!(trampoline.park(0), None);
                    panic!("subscribe unwinds");
                },
                |_| unreachable!("the parked round is dropped, not started"),
            );
        }));
        assert!(result.is_err());
        assert_eq!(trampoline.park(1), Some(1));
    }
}
