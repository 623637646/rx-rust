use crate::{
    disposable::Disposable,
    utils::{
        id_generator::{Id, IdGenerator},
        mutable::{Mutable, MutableHelper},
        types::Shared,
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
#[educe(Debug, Default)]
struct Inner<D> {
    state: State<D>,
    id_generator: IdGenerator,
}

#[derive(Educe)]
#[educe(Debug, Clone, Default)]
pub struct SharedDisposal<D>(Shared<Mutable<Inner<D>>>);

impl<D> SharedDisposal<D> {
    pub fn replace(&self, disposal_builder: impl FnOnce() -> D)
    where
        D: Disposable,
    {
        // The superseded disposal is handed back rather than disposed under the lock.
        let (id, superseded) = self.0.with_mut(|inner| {
            if matches!(inner.state, State::Disposed) {
                return (None, None);
            }
            let id = inner.id_generator.next_id();
            match std::mem::replace(&mut inner.state, State::Building(id)) {
                State::Idle | State::Building(_) => (Some(id), None),
                State::Active(disposal) => (Some(id), Some(disposal)),
                State::Disposed => unreachable!("the disposed state returned above"),
            }
        });
        if let Some(disposal) = superseded {
            disposal.dispose();
        }

        let Some(id) = id else {
            return;
        };

        let disposable = disposal_builder();

        let stale = self.0.with_mut(|inner| {
            let is_current = match &inner.state {
                State::Building(current) => *current == id,
                State::Idle | State::Active(_) | State::Disposed => false,
            };
            if is_current {
                inner.state = State::Active(disposable);
                None
            } else {
                // Reset, disposed, or superseded by a later build: this disposal is already dead.
                Some(disposable)
            }
        });
        if let Some(disposable) = stale {
            disposable.dispose();
        }
    }
}

impl<D> Disposable for SharedDisposal<D>
where
    D: Disposable,
{
    fn dispose(self) {
        // The state is replaced under the lock and matched after it is released, so the inner
        // disposal is disposed outside the lock.
        match self
            .0
            .with_mut(|inner| std::mem::replace(&mut inner.state, State::Disposed))
        {
            State::Idle | State::Building(_) | State::Disposed => {}
            State::Active(disposable) => {
                Disposable::dispose(disposable);
            }
        }
    }
}
