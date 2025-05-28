// Refer to https://stackoverflow.com/a/79621785/9315497
// Refer to https://gist.github.com/623637646/9a0221954781084acc3299af117d2f4e

use std::sync::Mutex;

pub(crate) trait InstantMutLock<T, R> {
    fn lock_mut(&self, callback: impl FnOnce(&mut T) -> R) -> R;
}

impl<T, R> InstantMutLock<T, R> for Mutex<T> {
    fn lock_mut(&self, callback: impl FnOnce(&mut T) -> R) -> R {
        let mut lock = self.lock().unwrap();
        let result = callback(&mut lock);
        drop(lock);
        result
    }
}

pub(crate) trait InstantRefLock<T, R> {
    fn lock_ref(&self, callback: impl FnOnce(&T) -> R) -> R;
}

impl<T, R> InstantRefLock<T, R> for Mutex<T> {
    fn lock_ref(&self, callback: impl FnOnce(&T) -> R) -> R {
        let lock = self.lock().unwrap();
        let result = callback(&lock);
        drop(lock);
        result
    }
}
