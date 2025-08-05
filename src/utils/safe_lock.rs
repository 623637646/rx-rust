//! We use this mod to avoid deadlocks.
//! Refer to this case: https://stackoverflow.com/q/79621758/9315497
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
        Clone::clone(&*$lock_name.lock_ref())
    }};

    (set: $lock_name:expr, $value:expr) => {{
        use $crate::utils::types::MutableHelper;
        let value = $value;
        *$lock_name.lock_mut() = value
    }};

    (mem_take: $lock_name:expr) => {{
        use $crate::utils::types::MutableHelper;
        std::mem::take(&mut *$lock_name.lock_mut())
    }};

    (mem_take: $lock_name:expr, $field_name:ident) => {{
        use $crate::utils::types::MutableHelper;
        std::mem::take(&mut $lock_name.lock_mut().$field_name)
    }};

    (mem_replace: $lock_name:expr, $value:expr) => {{
        use $crate::utils::types::MutableHelper;
        let value = $value;
        std::mem::replace(&mut *$lock_name.lock_mut(), value)
    }};
}

#[macro_export]
macro_rules! safe_lock_option {
    (is_none: $lock_name:expr) => {{
        use $crate::utils::types::MutableHelper;
        Option::is_none(&$lock_name.lock_ref())
    }};

    (is_some: $lock_name:expr) => {{
        use $crate::utils::types::MutableHelper;
        Option::is_some(&$lock_name.lock_ref())
    }};

    (take: $lock_name:expr) => {{
        use $crate::utils::types::MutableHelper;
        Option::take(&mut $lock_name.lock_mut())
    }};

    (take: $lock_name:expr, $field_name:ident) => {{
        use $crate::utils::types::MutableHelper;
        Option::take(&mut $lock_name.lock_mut().$field_name)
    }};

    (replace: $lock_name:expr, $value:expr) => {{
        use $crate::utils::types::MutableHelper;
        let value = $value;
        Option::replace(&mut $lock_name.lock_mut(), value)
    }};

    (replace: $lock_name:expr, $field_name:ident, $value:expr) => {{
        use $crate::utils::types::MutableHelper;
        let value = $value;
        Option::replace(&mut $lock_name.lock_mut().$field_name, value)
    }};
}

#[macro_export]
macro_rules! safe_lock_observer {
    (on_next: $lock_name:expr, $value:expr) => {{
        use $crate::utils::types::MutableHelper;
        let value = $value;
        Observer::on_next(&mut *$lock_name.lock_mut(), value)
    }};
}

#[macro_export]
macro_rules! safe_lock_option_observer {
    (on_next: $lock_name:expr, $value:expr) => {{
        use $crate::utils::types::MutableHelper;
        let value = $value;
        if let Some(observer) = $lock_name.lock_mut().as_mut() {
            Observer::on_next(observer, value);
            true
        } else {
            false
        }
    }};

    (on_termination: $lock_name:expr, $value:expr) => {{
        use $crate::safe_lock_option;
        if let Some(observer) = safe_lock_option!(take: $lock_name) {
            let value = $value;
            Observer::on_termination(observer, value);
            true
        } else {
            false
        }
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
        Vec::is_empty(&$lock_name.lock_ref())
    }};

    (len: $lock_name:expr) => {{
        use $crate::utils::types::MutableHelper;
        Vec::len(&$lock_name.lock_ref())
    }};

    (push: $lock_name:expr, $value:expr) => {{
        use $crate::utils::types::MutableHelper;
        let value = $value;
        Vec::push(&mut $lock_name.lock_mut(), value)
    }};
}

#[macro_export]
macro_rules! safe_lock_slot_map {
    (insert: $lock_name:expr, $field_name:ident, $value:expr) => {{
        use $crate::utils::types::MutableHelper;
        let value = $value;
        SlotMap::insert(&mut $lock_name.lock_mut().$field_name, value)
    }};
}
