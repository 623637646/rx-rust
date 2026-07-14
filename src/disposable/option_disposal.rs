use crate::disposable::Disposable;
use educe::Educe;

#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct OptionDisposal<D>(Option<D>);

impl<D> OptionDisposal<D> {
    pub fn some(disposal: D) -> Self {
        Self(Some(disposal))
    }

    pub fn none() -> Self {
        Self(None)
    }
}

impl<D: Disposable> Disposable for OptionDisposal<D> {
    fn dispose(self) {
        if let Some(disposal) = self.0 {
            Disposable::dispose(disposal)
        }
    }
}
