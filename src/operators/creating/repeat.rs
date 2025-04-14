use super::from_iter::FromIter;
use crate::{observable::Observable, observer::Observer, subscription::Subscription};
use educe::Educe;
use std::{convert::Infallible, iter::RepeatN};

#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct Repeat<T>(FromIter<RepeatN<T>>);

impl<T> Repeat<T> {
    pub fn new(value: T, n: usize) -> Self
    where
        T: Clone,
    {
        Self(FromIter::new(std::iter::repeat_n(value, n)))
    }
}

impl<'a, T, OR> Observable<'a, T, Infallible, OR> for Repeat<T>
where
    T: Clone,
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
    fn test_repeat() {
        let observable = Repeat::new(3, 4);
        let checker = CheckingObserver::new();

        let subscription = observable.subscribe(checker.clone());
        assert!(checker.is_values_matched(&[3, 3, 3, 3]));
        assert!(checker.is_completed());

        _ = subscription; // keep the subscription alive
    }

    #[test]
    fn test_clone() {
        let observable = Repeat::new(3, 4);
        let _ = observable.clone();
    }
}
