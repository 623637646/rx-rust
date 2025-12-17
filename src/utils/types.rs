use std::marker::PhantomData;

/// Using `PhantomData<fn(T) -> T>` instead of `PhantomData<T>` to make MarkerType to be `NecessarySendSync` when T is not `NecessarySendSync`.
/// For more detail: <https://doc.rust-lang.org/nomicon/phantom-data.html#table-of-phantomdata-patterns>
/// But the lifetime of MarkerType is affected by T. Which means T and MarkerType have the same lifetime.
/// TODO: find a better solution, so we can remove some restriction like `T: 'static` in some cases.
/// For more detail: <https://users.rust-lang.org/t/getting-phantomdata-to-have-a-static-lifetime/38505>
pub type MarkerType<T> = PhantomData<fn(T) -> T>;

pub trait MutableHelper<T> {
    fn lock_mut<R>(&self, callback: impl FnOnce(MutGuard<'_, T>) -> R) -> R;
    fn lock_ref<R>(&self, callback: impl FnOnce(RefGuard<'_, T>) -> R) -> R;
}

cfg_if::cfg_if! {
    if #[cfg(feature = "single-threaded")] {
        use std::{
            cell::{Ref, RefCell, RefMut},
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
        pub trait NecessarySendSync {}
        impl<T> NecessarySendSync for T {}
    } else {
        use std::sync::{Arc, Mutex, MutexGuard};
        use std::ops::Deref;
        use educe::Educe;

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
        pub trait NecessarySendSync: Send {}
        impl<T> NecessarySendSync for T where T: Send {}
    }
}
