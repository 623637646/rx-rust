use crate::disposable::Disposable;

pub struct AutoDisposal<T>(Option<T>)
where
    T: Disposable;

impl<T> AutoDisposal<T>
where
    T: Disposable,
{
    pub fn new(disposal: T) -> Self {
        Self(Some(disposal))
    }
}

impl<T> Disposable for AutoDisposal<T>
where
    T: Disposable,
{
    fn dispose(self) {
        // drop self to call the dispose
    }
}

impl<T> Drop for AutoDisposal<T>
where
    T: Disposable,
{
    fn drop(&mut self) {
        if let Some(disposal) = self.0.take() {
            disposal.dispose();
        }
    }
}
