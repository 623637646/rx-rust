use super::{Observer, Termination};
use crate::utils::types::MaybeSend;

trait ErasedObserver<T, E>: Observer<T, E> {
    fn on_termination_boxed(self: Box<Self>, termination: Termination<E>);
}

impl<T, E, OR> ErasedObserver<T, E> for OR
where
    OR: Observer<T, E>,
{
    fn on_termination_boxed(self: Box<Self>, termination: Termination<E>) {
        Observer::on_termination(*self, termination);
    }
}

cfg_if::cfg_if! {
    if #[cfg(feature = "single-threaded")] {
        /// Type-erased observer for single-threaded builds to handle this problem <https://stackoverflow.com/q/46620790/9315497>
        pub struct BoxedObserver<'or, T, E>(Box<dyn ErasedObserver<T, E> + 'or>);
    } else {
        /// Type-erased observer for multi-threaded builds to handle this problem <https://stackoverflow.com/q/46620790/9315497>
        pub struct BoxedObserver<'or, T, E>(Box<dyn ErasedObserver<T, E> + Send + 'or>);
    }
}

impl<'or, T, E> BoxedObserver<'or, T, E> {
    pub fn new(observer: impl Observer<T, E> + MaybeSend + 'or) -> Self {
        Self(Box::new(observer))
    }
}

impl<T, E> Observer<T, E> for BoxedObserver<'_, T, E> {
    #[inline]
    fn on_next(&mut self, value: T) {
        self.0.on_next(value);
    }

    #[inline]
    fn on_termination(self, termination: Termination<E>) {
        self.0.on_termination_boxed(termination);
    }
}

impl<T, E> std::fmt::Debug for BoxedObserver<'_, T, E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(std::any::type_name::<Self>())
    }
}
