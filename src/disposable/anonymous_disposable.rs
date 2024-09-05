use crate::disposable::Disposable;

pub struct AnonymousDisposable<F> {
    action: F,
}

impl<F> AnonymousDisposable<F> {
    pub fn new(action: F) -> AnonymousDisposable<F> {
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
