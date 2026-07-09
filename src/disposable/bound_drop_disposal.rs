use crate::disposable::Disposable;
use educe::Educe;

/// A disposal that calls the `dispose` method of a `Disposable` when dropped.
#[derive(Educe)]
#[educe(Debug)]
pub struct BoundDropDisposal<T>(Option<T>)
where
    T: Disposable;

impl<T> BoundDropDisposal<T>
where
    T: Disposable,
{
    pub fn new(disposal: T) -> Self {
        Self(Some(disposal))
    }
}

impl<T> Disposable for BoundDropDisposal<T>
where
    T: Disposable,
{
    fn dispose(self) {
        // Drop to call the dispose
    }
}

impl<T> Drop for BoundDropDisposal<T>
where
    T: Disposable,
{
    fn drop(&mut self) {
        if let Some(disposal) = self.0.take() {
            disposal.dispose();
        }
    }
}
