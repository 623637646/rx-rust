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
    pub fn new(callback: F) -> CallbackDisposal<F> {
        CallbackDisposal(callback)
    }
}

impl<F: FnOnce()> Disposable for CallbackDisposal<F> {
    fn dispose(self) {
        self.0();
    }
}

/// TODO: doc
/// https://stackoverflow.com/a/56447952/9315497
pub struct BoxedDisposal<'a>(Box<dyn FnOnce() + Send + 'a>);

impl<'a> BoxedDisposal<'a> {
    pub fn new(disposal: impl Disposable + Send + 'a) -> Self {
        BoxedDisposal(Box::new(|| {
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
        let boxed_disposal;
        let mut called = false;
        {
            let call_back_disposal = CallbackDisposal::new(|| {
                called = true;
            });
            boxed_disposal = BoxedDisposal::new(call_back_disposal);
        }
        boxed_disposal.dispose();
        assert!(called);
    }
}
