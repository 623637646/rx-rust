pub mod anonymous_disposable;
pub mod nop_disposable;

pub trait Disposable {
    fn dispose(self);
}
