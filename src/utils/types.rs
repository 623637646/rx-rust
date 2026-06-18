use crate::observer::Termination;
use std::marker::PhantomData;

/// Using `PhantomData<fn(T) -> T>` instead of `PhantomData<T>` to make MarkerType to be `NecessarySend` when T is not `NecessarySend`.
/// For more detail: <https://doc.rust-lang.org/nomicon/phantom-data.html#table-of-phantomdata-patterns>
/// But the lifetime of MarkerType is affected by T. Which means T and MarkerType have the same lifetime.
/// TODO: find a better solution, so we can remove some restriction like `T: 'static` in some cases.
/// For more detail: <https://users.rust-lang.org/t/getting-phantomdata-to-have-a-static-lifetime/38505>
pub type MarkerType<T> = PhantomData<fn(T) -> T>;

pub enum ActionAfterLock<T, E> {
    Next(T),
    Termination(Termination<E>),
    None,
}

pub trait MutableHelper<T> {
    fn lock_mut<R>(&self, callback: impl FnOnce(MutGuard<'_, T>) -> R) -> R;
    fn lock_ref<R>(&self, callback: impl FnOnce(RefGuard<'_, T>) -> R) -> R;
}

pub trait MutableBoolHelper {
    fn read(&self) -> bool;
    fn write(&self, value: bool);
    // Change the contained value to `value`, returns true if it was equal. otherwise false.
    fn change_if_not_equal(&self, value: bool) -> bool;
}

cfg_if::cfg_if! {
    if #[cfg(feature = "single-threaded")] {
        use std::{
            cell::{Ref, RefCell, RefMut, Cell},
            rc::Rc,
        };

        pub type Shared<T> = Rc<T>;
        pub type Mutable<T> = RefCell<T>;

        pub type MutGuard<'a, T> = RefMut<'a, T>;
        pub type RefGuard<'a, T> = Ref<'a, T>;
        impl<T> MutableHelper<T> for RefCell<T> {
            fn lock_mut<R>(&self, callback: impl FnOnce(MutGuard<'_, T>) -> R) ->R {
                callback(self.borrow_mut())
            }
            fn lock_ref<R>(&self, callback: impl FnOnce(RefGuard<'_, T>) ->R) ->R {
                callback(self.borrow())
            }
        }

        pub type MutableBool = Cell<bool>;
        impl MutableBoolHelper for Cell<bool> {
            fn read(&self) -> bool {
                self.get()
            }
            fn write(&self, value: bool) {
                self.set(value)
            }
            fn change_if_not_equal(&self, value: bool) -> bool {
                let old = self.replace(value);
                old == value
            }
        }

        pub trait NecessarySend {}
        impl<T> NecessarySend for T {}
        pub trait NecessarySync {}
        impl<T> NecessarySync for T {}
    } else {
        use std::sync::{Arc, Mutex, MutexGuard};
        use std::ops::Deref;
        use educe::Educe;
        use std::sync::atomic::{AtomicBool, Ordering};

        pub type Shared<T> = Arc<T>;
        pub type Mutable<T> = Mutex<T>;

        #[derive(Educe)]
        #[educe(Debug)]
        pub struct ReadOnlyMutexGuard<'a, T: ?Sized + 'a>(MutexGuard<'a, T>);
        impl<'a, T> ReadOnlyMutexGuard<'a, T> {
            pub fn new(guard: MutexGuard<'a, T>) -> Self {
                Self(guard)
            }
        }
        impl<T: ?Sized> Deref for ReadOnlyMutexGuard<'_, T> {
            type Target = T;
            fn deref(&self) -> &T {
                &self.0
            }
        }

        pub type MutGuard<'a, T> = MutexGuard<'a, T>;
        pub type RefGuard<'a, T> = ReadOnlyMutexGuard<'a, T>;
        impl<T> MutableHelper<T> for Mutex<T> {
            fn lock_mut<R>(&self, callback: impl FnOnce(MutGuard<'_, T>) -> R) ->R {
                callback(self.lock().unwrap())
            }
            fn lock_ref<R>(&self, callback: impl FnOnce(RefGuard<'_, T>) ->R) ->R {
                callback(ReadOnlyMutexGuard(self.lock().unwrap()))
            }
        }

        pub type MutableBool = AtomicBool;
        impl MutableBoolHelper for MutableBool {
            fn read(&self) -> bool {
                self.load(Ordering::SeqCst)
            }
            fn write(&self, value: bool) {
                self.store(value, Ordering::SeqCst)
            }
            fn change_if_not_equal(&self, value: bool) -> bool {
                self.compare_exchange(!value, value, Ordering::SeqCst, Ordering::SeqCst).is_err()
            }
        }

        pub trait NecessarySend: Send {}
        impl<T> NecessarySend for T where T: Send {}
        pub trait NecessarySync: Sync {}
        impl<T> NecessarySync for T where T: Sync {}
    }
}
