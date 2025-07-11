use std::marker::PhantomData;

/// Using `PhantomData<fn(T) -> T>` instead of `PhantomData<T>` to make MarkerType to be `NecessarySend + Sync` when T is not `NecessarySend + Sync`.
/// For more detail: https://doc.rust-lang.org/nomicon/phantom-data.html#table-of-phantomdata-patterns
/// But the lifetime of MarkerType is affected by T. Which means T and MarkerType have the same lifetime.
/// TODO: find a better solution, so we can remove some restriction like `T: 'static` in some cases.
/// For more detail: https://users.rust-lang.org/t/getting-phantomdata-to-have-a-static-lifetime/38505
pub type MarkerType<T> = PhantomData<fn(T) -> T>;

cfg_if::cfg_if! {
    if #[cfg(feature = "single-threaded")] {
        use std::{
            cell::{Ref, RefCell, RefMut},
            rc::Rc,
        };
        pub type Shared<T> = Rc<T>;
        pub type Mutable<T> = RefCell<T>;
        pub trait MutableHelper<T> {
            fn lock_mut(&self) -> RefMut<'_, T>;
            fn lock_ref(&self) -> Ref<'_, T>;
        }
        impl<T> MutableHelper<T> for RefCell<T> {
            fn lock_mut(&self) -> RefMut<'_, T> {
                self.borrow_mut()
            }
            fn lock_ref(&self) -> Ref<'_, T> {
                self.borrow()
            }
        }
        pub trait NecessarySend {}
        impl<T> NecessarySend for T {}
    } else {
        use std::sync::{Arc, Mutex, MutexGuard};
        pub type Shared<T> = Arc<T>;
        pub type Mutable<T> = Mutex<T>;
        pub trait MutableHelper<T> {
            fn lock_mut(&self) -> MutexGuard<'_, T>;
            fn lock_ref(&self) -> MutexGuard<'_, T>;
        }
        impl<T> MutableHelper<T> for Mutex<T> {
            fn lock_mut(&self) -> MutexGuard<'_, T> {
                self.lock().unwrap()
            }
            fn lock_ref(&self) -> MutexGuard<'_, T> {
                self.lock().unwrap()
            }
        }
        pub trait NecessarySend: Send {}
        impl<T> NecessarySend for T where T: Send {}
    }
}
