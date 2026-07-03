use crate::{
    disposable::Disposable,
    utils::{
        increment_id::IncrementId,
        types::{Mutable, MutableHelper, Shared},
    },
};
use educe::Educe;

#[derive(Educe)]
#[educe(Debug, Default)]
enum State<D> {
    #[educe(Default)]
    Idle,
    Building,
    Processing(D),
    Disposed,
}

#[derive(Educe)]
#[educe(Debug, Clone, Default)]
pub struct SharedDisposal<D>(Shared<Mutable<(State<D>, IncrementId)>>);

impl<D> SharedDisposal<D> {
    pub fn replace(&self, disposal_builder: impl FnOnce() -> D)
    where
        D: Disposable,
    {
        let id = self.0.lock_mut(|mut lock| match &mut lock.0 {
            State::Idle => {
                lock.0 = State::Building;
                Some(lock.1.increment())
            }
            State::Building => Some(lock.1.increment()),
            State::Processing(_) => match std::mem::replace(&mut lock.0, State::Building) {
                State::Idle | State::Disposed | State::Building => {
                    unreachable!()
                }
                State::Processing(disposal) => {
                    let id = lock.1.increment();
                    drop(lock);
                    disposal.dispose();
                    Some(id)
                }
            },
            State::Disposed => None,
        });

        let Some(id) = id else {
            return;
        };

        let disposable = disposal_builder();

        self.0.lock_mut(|mut lock| {
            if id != lock.1 {
                drop(lock);
                disposable.dispose();
                return;
            }

            match &mut lock.0 {
                State::Idle => {
                    // It's reset already.
                    drop(lock);
                    disposable.dispose();
                }
                State::Building => {
                    lock.0 = State::Processing(disposable);
                }
                State::Processing(_) => {
                    unreachable!()
                }
                State::Disposed => {
                    drop(lock);
                    disposable.dispose()
                }
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
                State::Idle | State::Building | State::Disposed => {}
                State::Processing(disposable) => {
                    Disposable::dispose(disposable);
                }
            }
        })
    }
}
