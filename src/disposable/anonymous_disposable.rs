use crate::disposable::Disposable;

/// Create AnonymousDisposable with a closure.
/// The closure will be called when the disposable is disposed or dropped.
/// The closure will be called at most once.
///
/// # Examples
/// ```rust
/// use crate::rx_rust::disposable::anonymous_disposable::AnonymousDisposable;
/// use crate::rx_rust::disposable::Disposable;
/// let disposable = AnonymousDisposable::new(|| println!("disposed"));
/// disposable.dispose();
/// ```

pub struct AnonymousDisposable<F>
where
    F: FnOnce(),
{
    action: Option<F>, // None if disposed
}

impl<F> AnonymousDisposable<F>
where
    F: FnOnce(),
{
    pub fn new(action: F) -> AnonymousDisposable<F> {
        AnonymousDisposable {
            action: Some(action),
        }
    }
}

impl<F> Disposable for AnonymousDisposable<F>
where
    F: FnOnce(),
{
    fn dispose(mut self) {
        let action = self.action.take().unwrap();
        action();
    }
}

impl<F> Drop for AnonymousDisposable<F>
where
    F: FnOnce(),
{
    fn drop(&mut self) {
        if let Some(action) = self.action.take() {
            action();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dispose() {
        let mut disposed = false;
        let disposable = AnonymousDisposable::new(|| {
            assert!(!disposed);
            disposed = true;
        });
        disposable.dispose();
        assert!(disposed);
    }

    #[test]
    fn test_dropped() {
        let mut disposed = false;
        {
            AnonymousDisposable::new(|| {
                assert!(!disposed);
                disposed = true;
            });
        }
        assert!(disposed);
    }
}
