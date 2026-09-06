use crate::{
    disposable::Disposable,
    utils::{
        id_generator::{Id, IdGenerator},
        types::{Mutable, MutableHelper, Shared},
    },
};
use educe::Educe;

#[derive(Educe)]
#[educe(Debug, Default)]
enum State<D> {
    #[educe(Default)]
    Idle,
    /// A disposal is being built with the lock released. The id is what the finished build
    /// compares itself against: a later `replace` starts its own build and stores its own id
    /// here, which is what makes the earlier one stale.
    Building(Id),
    Active(D),
    Disposed,
}

#[derive(Educe)]
#[educe(Debug, Clone, Default)]
pub struct SharedDisposal<D>(Shared<Mutable<(State<D>, IdGenerator)>>);

impl<D> SharedDisposal<D> {
    pub fn replace(&self, disposal_builder: impl FnOnce() -> D)
    where
        D: Disposable,
    {
        let id = self.0.lock_mut(|mut lock| {
            if matches!(lock.0, State::Disposed) {
                return None;
            }
            let id = lock.1.next_id();
            match std::mem::replace(&mut lock.0, State::Building(id)) {
                State::Idle | State::Building(_) => Some(id),
                State::Active(disposal) => {
                    drop(lock);
                    disposal.dispose();
                    Some(id)
                }
                State::Disposed => unreachable!("the disposed state returned above"),
            }
        });

        let Some(id) = id else {
            return;
        };

        let disposable = disposal_builder();

        self.0.lock_mut(|mut lock| {
            let is_current = match &lock.0 {
                State::Building(current) => *current == id,
                State::Idle | State::Active(_) | State::Disposed => false,
            };
            if is_current {
                lock.0 = State::Active(disposable);
            } else {
                // Reset, disposed, or superseded by a later build: this disposal is already dead,
                // and is disposed outside the lock.
                drop(lock);
                disposable.dispose();
            }
        });
    }
}

impl<D> Disposable for SharedDisposal<D>
where
    D: Disposable,
{
    fn dispose(self) {
        self.0.lock_mut(|mut lock| {
            let state = std::mem::replace(&mut lock.0, State::Disposed);
            drop(lock);
            match state {
                State::Idle | State::Building(_) | State::Disposed => {}
                State::Active(disposable) => {
                    Disposable::dispose(disposable);
                }
            }
        })
    }
}
