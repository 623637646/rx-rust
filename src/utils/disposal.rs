/// A struct that calls a function when it is dropped.

pub struct Disposal<F>
where
    F: FnOnce(),
{
    action: Option<F>,
}

impl<F> Disposal<F>
where
    F: FnOnce(),
{
    pub fn new(action: F) -> Disposal<F> {
        Disposal {
            action: Some(action),
        }
    }

    pub fn dispose(self) {
        // drop self to call the dispose
    }

    /// Converts the disposal to a boxed disposal. It's used in delay.rs.
    pub fn to_boxed(mut self) -> Disposal<Box<dyn FnOnce() + Send>>
    where
        F: Send + 'static,
    {
        Disposal {
            action: Some(Box::new(self.action.take().unwrap())),
        }
    }
}

impl<F> Drop for Disposal<F>
where
    F: FnOnce(),
{
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
        let mut disposed = false;
        let disposal = Disposal::new(|| {
            assert!(!disposed);
            disposed = true;
        });
        disposal.dispose();
        assert!(disposed);
    }

    #[test]
    fn test_dropped() {
        let mut disposed = false;
        {
            Disposal::new(|| {
                assert!(!disposed);
                disposed = true;
            });
        }
        assert!(disposed);
    }

    #[test]
    fn test_call_dispose_boxed() {
        let disposed = Arc::new(Mutex::new(false));
        let disposed_cloned = disposed.clone();
        let disposal = Disposal::new(move || {
            let mut disposed = disposed_cloned.lock().unwrap();
            assert!(!*disposed);
            *disposed = true;
        });
        let disposal = disposal.to_boxed();
        disposal.dispose();
        assert!(*disposed.lock().unwrap());
    }

    #[test]
    fn test_dropped_boxed() {
        let disposed = Arc::new(Mutex::new(false));
        let disposed_cloned = disposed.clone();
        {
            let disposal = Disposal::new(move || {
                let mut disposed = disposed_cloned.lock().unwrap();
                assert!(!*disposed);
                *disposed = true;
            });
            let disposal = disposal.to_boxed();
            _ = disposal;
        }
        assert!(*disposed.lock().unwrap());
    }
}
