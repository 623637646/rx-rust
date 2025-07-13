use crate::disposable::Disposable;

impl Disposable for futures::stream::AbortHandle {
    fn dispose(self) {
        self.abort();
    }
}
