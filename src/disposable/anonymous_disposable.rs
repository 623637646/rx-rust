use super::Disposable;

pub(crate) struct AnonymousDisposable<F> {
    action: F,
}

impl<F> AnonymousDisposable<F> {
    pub(crate) fn new(action: F) -> AnonymousDisposable<F> {
        AnonymousDisposable { action }
    }
}

impl<F> Disposable for AnonymousDisposable<F>
where
    F: Fn(),
{
    fn dispose(&self) {
        (self.action)();
    }
}

// TODO unit tests
