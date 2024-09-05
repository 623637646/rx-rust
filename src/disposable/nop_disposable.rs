use crate::disposable::Disposable;

pub struct NopDisposable;

impl Disposable for NopDisposable {
    fn dispose(&self) {}
}
