/// A struct that calls a function when it is dropped.

pub struct Disposal {
    action: Option<Box<dyn FnOnce() + Sync + Send + 'static>>,
}

impl Disposal {
    pub fn new_no_action() -> Disposal {
        Disposal { action: None }
    }

    pub fn new<F>(action: F) -> Disposal
    where
        F: FnOnce() + Sync + Send + 'static,
    {
        Disposal {
            action: Some(Box::new(action)),
        }
    }

    pub fn dispose(self) {
        // drop self to call the dispose
    }
}

impl Drop for Disposal {
    fn drop(&mut self) {
        // This code causes tarpaulin wrong coverage report. For more details, see: https://github.com/xd009642/tarpaulin/issues/1624
        // if let Some(action) = self.action.take() {
        //     action();
        // }
        // This code is a workaround for the issue above.
        match self.action.take() {
            Some(action) => action(),
            None => {
                // Do nothing, using this pattern to avoid clippy warning.
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    #[test]
    fn test_call_dispose() {
        let disposed = Arc::new(Mutex::new(false));
        let disposed_cloned = disposed.clone();
        let disposal = Disposal::new(move || {
            let mut disposed = disposed_cloned.lock().unwrap();
            assert!(!*disposed);
            *disposed = true;
        });
        disposal.dispose();
        assert!(*disposed.lock().unwrap());
    }

    #[test]
    fn test_dropped() {
        let disposed = Arc::new(Mutex::new(false));
        let disposed_cloned = disposed.clone();
        {
            Disposal::new(move || {
                let mut disposed = disposed_cloned.lock().unwrap();
                assert!(!*disposed);
                *disposed = true;
            });
        }
        assert!(*disposed.lock().unwrap());
    }
}
