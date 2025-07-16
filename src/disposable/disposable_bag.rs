use crate::disposable::Disposable;
use std::ops::Add;

pub struct DisposableBag<T>(Vec<T>);

impl<T> DisposableBag<T> {
    pub fn new() -> Self {
        Self(Vec::new())
    }

    pub fn append_disposable(&mut self, disposable: T) {
        self.0.push(disposable);
    }
}

impl<T> Default for DisposableBag<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Add<T> for DisposableBag<T> {
    type Output = DisposableBag<T>;

    #[inline]
    fn add(mut self, other: T) -> DisposableBag<T> {
        self.append_disposable(other);
        self
    }
}

impl<T> Disposable for DisposableBag<T>
where
    T: Disposable,
{
    fn dispose(self) {
        for disposable in self.0 {
            disposable.dispose();
        }
    }
}
