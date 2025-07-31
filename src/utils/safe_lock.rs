/// We use this mod to avoid deadlocks.
/// Refer to this case: https://stackoverflow.com/q/79621758/9315497
/// And this case:
///
/// fn main() {
///    use std::sync::Mutex;
///    let lock = Mutex::new("My String".to_owned());
///    // let equals = { lock.lock().unwrap().clone() } == { lock.lock().unwrap().clone() }; // No deadlock
///    let equals = lock.lock().unwrap().clone() == lock.lock().unwrap().clone(); // Deadlock
///    println!("{}", equals);
/// }
use crate::{
    disposable::Disposable,
    observer::{Observer, Termination},
    utils::types::{Mutable, MutableHelper},
};
use std::collections::VecDeque;

pub trait SafeLock<T> {
    fn safe_lock_clone(&self) -> T
    where
        T: Clone;

    fn safe_lock_set(&self, value: T);

    fn safe_lock_mem_take(&self) -> T
    where
        T: Default;

    #[must_use = "if you don't need the old value, you can just assign the new value directly"]
    fn safe_lock_mem_replace(&self, value: T) -> T;

    fn safe_lock_ref<R>(&self, callback: impl FnOnce(&T) -> R) -> R;

    fn safe_lock_mut<R>(&self, callback: impl FnOnce(&mut T) -> R) -> R;

    fn safe_lock_on_next<T1, E>(&self, value: T1)
    where
        T: Observer<T1, E>;
}

impl<T> SafeLock<T> for Mutable<T> {
    fn safe_lock_clone(&self) -> T
    where
        T: Clone,
    {
        self.lock_ref().clone()
    }

    fn safe_lock_set(&self, value: T) {
        *self.lock_mut() = value;
    }

    fn safe_lock_mem_take(&self) -> T
    where
        T: Default,
    {
        std::mem::take(&mut self.lock_mut())
    }

    fn safe_lock_mem_replace(&self, value: T) -> T {
        std::mem::replace(&mut self.lock_mut(), value)
    }

    fn safe_lock_ref<R>(&self, callback: impl FnOnce(&T) -> R) -> R {
        callback(&self.lock_ref())
    }

    fn safe_lock_mut<R>(&self, callback: impl FnOnce(&mut T) -> R) -> R {
        callback(&mut self.lock_mut())
    }

    fn safe_lock_on_next<T1, E>(&self, value: T1)
    where
        T: Observer<T1, E>,
    {
        self.lock_mut().on_next(value);
    }
}

pub trait SafeLockOption<T> {
    fn safe_lock_is_none(&self) -> bool;

    fn safe_lock_is_some(&self) -> bool;

    fn safe_lock_take(&self) -> Option<T>;

    fn safe_lock_replace(&self, value: T) -> Option<T>;

    fn safe_lock_on_next_if_some<T1, E>(&self, value: T1) -> bool
    where
        T: Observer<T1, E>;

    fn safe_lock_on_next_with_builder<T1, E>(
        &self,
        value_builder: impl FnOnce() -> Option<T1>,
    ) -> bool
    where
        T: Observer<T1, E>;

    fn safe_lock_unwrap_on_next<T1, E>(&self, value: T1)
    where
        T: Observer<T1, E>;

    fn safe_lock_on_termination_if_some<T1, E>(&self, termination: Termination<E>) -> bool
    where
        T: Observer<T1, E>;

    fn safe_lock_unwrap_on_termination<T1, E>(&self, termination: Termination<E>)
    where
        T: Observer<T1, E>;

    fn safe_lock_dispose_if_some(&self) -> bool
    where
        T: Disposable;

    fn safe_lock_unwrap_dispose(&self)
    where
        T: Disposable;
}

impl<T> SafeLockOption<T> for Mutable<Option<T>> {
    fn safe_lock_is_none(&self) -> bool {
        self.lock_ref().is_none()
    }

    fn safe_lock_is_some(&self) -> bool {
        self.lock_ref().is_some()
    }

    fn safe_lock_take(&self) -> Option<T> {
        self.lock_mut().take()
    }

    fn safe_lock_replace(&self, value: T) -> Option<T> {
        self.lock_mut().replace(value)
    }

    fn safe_lock_on_next_if_some<T1, E>(&self, value: T1) -> bool
    where
        T: Observer<T1, E>,
    {
        if let Some(observer) = self.lock_mut().as_mut() {
            observer.on_next(value);
            true
        } else {
            false
        }
    }

    fn safe_lock_on_next_with_builder<T1, E>(
        &self,
        value_builder: impl FnOnce() -> Option<T1>,
    ) -> bool
    where
        T: Observer<T1, E>,
    {
        if let Some(observer) = self.lock_mut().as_mut() {
            if let Some(value) = value_builder() {
                observer.on_next(value);
                true
            } else {
                false
            }
        } else {
            false
        }
    }

    fn safe_lock_unwrap_on_next<T1, E>(&self, value: T1)
    where
        T: Observer<T1, E>,
    {
        self.lock_mut().as_mut().unwrap().on_next(value);
    }

    fn safe_lock_on_termination_if_some<T1, E>(&self, termination: Termination<E>) -> bool
    where
        T: Observer<T1, E>,
    {
        if let Some(observer) = self.safe_lock_take() {
            observer.on_termination(termination);
            true
        } else {
            false
        }
    }

    fn safe_lock_unwrap_on_termination<T1, E>(&self, termination: Termination<E>)
    where
        T: Observer<T1, E>,
    {
        self.safe_lock_take().unwrap().on_termination(termination);
    }

    fn safe_lock_dispose_if_some(&self) -> bool
    where
        T: Disposable,
    {
        if let Some(disposable) = self.safe_lock_take() {
            disposable.dispose();
            true
        } else {
            false
        }
    }

    fn safe_lock_unwrap_dispose(&self)
    where
        T: Disposable,
    {
        self.safe_lock_take().unwrap().dispose();
    }
}

pub trait SafeLockVec<T> {
    fn safe_lock_is_empty(&self) -> bool;

    fn safe_lock_len(&self) -> usize;

    fn safe_lock_clear(&self);

    fn safe_lock_push(&self, value: T);
}

impl<T> SafeLockVec<T> for Mutable<Vec<T>> {
    fn safe_lock_is_empty(&self) -> bool {
        self.lock_ref().is_empty()
    }

    fn safe_lock_len(&self) -> usize {
        self.lock_ref().len()
    }

    fn safe_lock_clear(&self) {
        self.lock_mut().clear();
    }

    fn safe_lock_push(&self, value: T) {
        self.lock_mut().push(value);
    }
}

pub trait SafeLockVecDeque<T> {
    fn safe_lock_is_empty(&self) -> bool;

    fn safe_lock_len(&self) -> usize;

    fn safe_lock_pop_front(&self) -> Option<T>;

    fn safe_lock_push_back(&self, value: T);
}

impl<T> SafeLockVecDeque<T> for Mutable<VecDeque<T>> {
    fn safe_lock_is_empty(&self) -> bool {
        self.lock_ref().is_empty()
    }

    fn safe_lock_len(&self) -> usize {
        self.lock_ref().len()
    }

    fn safe_lock_pop_front(&self) -> Option<T> {
        self.lock_mut().pop_front()
    }

    fn safe_lock_push_back(&self, value: T) {
        self.lock_mut().push_back(value);
    }
}
