pub(crate) mod anonymous_disposable;
pub(crate) mod nop_disposable;

pub trait Disposable {
    fn dispose(&self);
}
