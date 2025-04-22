/// A trait that represents a disposable resource.
pub trait Disposable {
    /// Disposes of the resource.
    fn dispose(self);
}

/// A disposal that calls a callback when disposed.
pub struct CallbackDisposal<F: FnOnce()>(F);

impl<F> CallbackDisposal<F>
where
    F: FnOnce(),
{
    /// Creates a new callback disposal.
    pub fn new(callback: F) -> Self {
        Self(callback)
    }
}

impl<F: FnOnce()> Disposable for CallbackDisposal<F> {
    fn dispose(self) {
        self.0();
    }
}

/// TODO: doc
/// https://stackoverflow.com/a/56447952/9315497
pub struct BoxedDisposal<'dis>(Box<dyn FnOnce() + Send + 'dis>);

impl<'dis> BoxedDisposal<'dis> {
    pub fn new(disposal: impl Disposable + Send + 'dis) -> Self {
        Self(Box::new(|| {
            disposal.dispose();
        }))
    }
}

impl Disposable for BoxedDisposal<'_> {
    fn dispose(self) {
        self.0();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::tests_utils::test_struct::TestStruct;

    #[test]
    fn test_callback_disposal() {
        let mut called = false;
        let disposal = CallbackDisposal::new(|| {
            called = true;
        });
        disposal.dispose();
        assert!(called);
    }

    #[test]
    fn test_boxed_disposal() {
        let mut called = false;
        let disposal = CallbackDisposal::new(|| {
            called = true;
        });
        let disposal = BoxedDisposal::new(disposal);
        disposal.dispose();
        assert!(called);
    }

    #[test]
    fn test_lifetime_boxed() {
        // OK
        let life_marker = TestStruct;
        let disposal;

        // Error
        // let disposal;
        // let life_marker = TestStruct;

        {
            let callback_disposal = CallbackDisposal::new(|| {
                life_marker.consume_ref();
            });
            disposal = BoxedDisposal::new(callback_disposal);
        }

        _ = disposal;
    }
}
