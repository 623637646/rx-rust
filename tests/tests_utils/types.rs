use rx_rust::utils::types::RefGuard;

pub(crate) trait TestMutableHelper<T> {
    fn test_lock_ref(&self) -> RefGuard<'_, T>;
}

cfg_if::cfg_if! {
    if #[cfg(feature = "single-threaded")] {
        use std::cell::RefCell;
        impl<T> TestMutableHelper<T> for RefCell<T> {
            fn test_lock_ref(&self) -> RefGuard<'_, T> {
                self.borrow()
            }
        }
    } else {
        use std::sync::{Mutex};
        use rx_rust::utils::types::ReadOnlyMutexGuard;
        impl<T> TestMutableHelper<T> for Mutex<T> {
            fn test_lock_ref(&self) -> RefGuard<'_, T> {
                ReadOnlyMutexGuard::new(self.lock().unwrap())
            }
        }
    }
}
