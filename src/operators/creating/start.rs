use super::just::Just;
use crate::{observable::Observable, observer::Observer, subscription::Subscription};
use std::convert::Infallible;

#[derive(Clone)]
pub struct Start<T>(Just<T>);

impl<T> Start<T> {
    pub fn new<F>(f: F) -> Self
    where
        F: FnOnce() -> T,
    {
        Self(Just::new(f()))
    }
}

impl<'a, T, OR> Observable<'a, T, Infallible, OR> for Start<T>
where
    OR: Observer<T, Infallible>,
{
    fn subscribe(self, observer: OR) -> Subscription<'a> {
        self.0.subscribe(observer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{observable::Observable, utils::tests_utils::checking_observer::CheckingObserver};

    #[test]
    fn test_start() {
        let value = 111;
        let observable = Start::new(|| value + 222);
        let checker = CheckingObserver::new();

        let subscription = observable.subscribe(checker.clone());
        assert!(checker.is_values_matched(&[333]));
        assert!(checker.is_completed());

        _ = subscription; // keep the subscription alive
    }
}
