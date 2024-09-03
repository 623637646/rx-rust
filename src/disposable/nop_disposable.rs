use super::Disposable;

pub(crate) struct NopDisposable;

impl NopDisposable {
    pub(crate) fn new() -> NopDisposable {
        NopDisposable
    }
}

impl Disposable for NopDisposable {
    fn dispose(&self) {}
}
