//! We use this mod to avoid deadlocks.
//! Refer to this case: <https://stackoverflow.com/q/79621758/9315497>
//! And this case:
//!
//! fn main() {
//!    use std::sync::Mutex;
//!    let lock = Mutex::new("My String".to_owned());
//!    // let equals = { lock.lock().unwrap().clone() } == { lock.lock().unwrap().clone() }; // No deadlock
//!    let equals = lock.lock().unwrap().clone() == lock.lock().unwrap().clone(); // Deadlock
//!    println!("{}", equals);
//! }
//!
//! We don't use this more common macro below, because it may cause deadlocks in Disposable of merge_all.rs.
//!
//! macro_rules! safe_lock {
//!     ($lock_name:expr, $field_name:ident, $method_name:ident) => {{
//!         use $crate::utils::types::MutableHelper;
//!         $lock_name.lock_mut().$field_name.$method_name()
//!     }};
//! }
//!
//! The code like this may cause a deadlock:
//! safe_lock!(self, subscriptions, clear);
//!
//! We use this approach: `Clone::clone(&*$lock_name.lock_ref())` instead of `$lock_name.lock_ref().clone()` to do the type checking.

#[macro_export]
macro_rules! safe_lock {
    (clone: $lock_name:expr) => {{
        use $crate::utils::types::MutableHelper;
        $lock_name.lock_ref(|lock| Clone::clone(&*lock))
    }};

    (set: $lock_name:expr, $value:expr) => {{
        use $crate::utils::types::MutableHelper;
        let value = $value;
        $lock_name.lock_mut(|mut lock| *lock = value)
    }};

    (mem_take: $lock_name:expr) => {{
        use $crate::utils::types::MutableHelper;
        $lock_name.lock_mut(|mut lock| std::mem::take(&mut *lock))
    }};

    (mem_take: $lock_name:expr, $field_name:ident) => {{
        use $crate::utils::types::MutableHelper;
        $lock_name.lock_mut(|mut lock| std::mem::take(&mut lock.$field_name))
    }};

    (mem_replace: $lock_name:expr, $value:expr) => {{
        use $crate::utils::types::MutableHelper;
        let value = $value;
        $lock_name.lock_mut(|mut lock| std::mem::replace(&mut *lock, value))
    }};

    (mem_replace: $lock_name:expr, $field_name:ident, $value:expr) => {{
        use $crate::utils::types::MutableHelper;
        let value = $value;
        $lock_name.lock_mut(|mut lock| std::mem::replace(&mut lock.$field_name, value))
    }};
}

#[macro_export]
macro_rules! safe_lock_option {
    (is_none: $lock_name:expr) => {{
        use $crate::utils::types::MutableHelper;
        $lock_name.lock_ref(|lock| Option::is_none(&*lock))
    }};

    (is_some: $lock_name:expr) => {{
        use $crate::utils::types::MutableHelper;
        $lock_name.lock_ref(|lock| Option::is_some(&*lock))
    }};

    (take: $lock_name:expr) => {{
        use $crate::utils::types::MutableHelper;
        $lock_name.lock_mut(|mut lock| Option::take(&mut *lock))
    }};

    (take: $lock_name:expr, $field_name:ident) => {{
        use $crate::utils::types::MutableHelper;
        $lock_name.lock_mut(|mut lock| Option::take(&mut lock.$field_name))
    }};

    (replace: $lock_name:expr, $value:expr) => {{
        use $crate::utils::types::MutableHelper;
        let value = $value;
        $lock_name.lock_mut(|mut lock| Option::replace(&mut *lock, value))
    }};

    (replace: $lock_name:expr, $field_name:ident, $value:expr) => {{
        use $crate::utils::types::MutableHelper;
        let value = $value;
        $lock_name.lock_mut(|mut lock| Option::replace(&mut lock.$field_name, value))
    }};
}

#[macro_export]
macro_rules! safe_lock_option_disposable {
    (dispose: $lock_name:expr) => {{
        use $crate::safe_lock_option;
        if let Some(disposable) = safe_lock_option!(take: $lock_name) {
            Disposable::dispose(disposable);
            true
        } else {
            false
        }
    }};

    (dispose: $lock_name:expr, $field_name:ident) => {{
        use $crate::safe_lock_option;
        if let Some(disposable) = safe_lock_option!(take: $lock_name, $field_name) {
            Disposable::dispose(disposable);
            true
        } else {
            false
        }
    }};
}

#[macro_export]
macro_rules! safe_lock_vec {
    (is_empty: $lock_name:expr) => {{
        use $crate::utils::types::MutableHelper;
        $lock_name.lock_ref(|lock| Vec::is_empty(&*lock))
    }};

    (len: $lock_name:expr) => {{
        use $crate::utils::types::MutableHelper;
        $lock_name.lock_ref(|lock| Vec::len(&*lock))
    }};

    (push: $lock_name:expr, $value:expr) => {{
        use $crate::utils::types::MutableHelper;
        let value = $value;
        $lock_name.lock_mut(|mut lock| Vec::push(&mut *lock, value))
    }};

    (push: $lock_name:expr, $field_name:ident, $value:expr) => {{
        use $crate::utils::types::MutableHelper;
        let value = $value;
        $lock_name.lock_mut(|mut lock| Vec::push(&mut lock.$field_name, value))
    }};

    (pop: $lock_name:expr) => {{
        use $crate::utils::types::MutableHelper;
        $lock_name.lock_mut(|mut lock| Vec::pop(&mut *lock))
    }};

    (pop: $lock_name:expr, $field_name:ident) => {{
        use $crate::utils::types::MutableHelper;
        $lock_name.lock_mut(|mut lock| Vec::pop(&mut lock.$field_name))
    }};
}
