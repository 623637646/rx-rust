use crate::disposable::Disposable;

impl Disposable for () {
    fn dispose(self) {}
}
