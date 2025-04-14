use super::from_iter::FromIter;
use crate::{observable::Observable, observer::Observer, subscription::Subscription};
use std::{convert::Infallible, ops::RangeBounds};

#[derive(Clone)]
pub struct Range<I>(FromIter<I>);

impl<I> Range<I> {
    pub fn new<T>(range: I) -> Self
    where
        I: IntoIterator<Item = T> + RangeBounds<T>,
    {
        Self(FromIter::new(range))
    }
}

impl<'a, T, OR, I> Observable<'a, T, Infallible, OR> for Range<I>
where
    OR: Observer<T, Infallible>,
    I: IntoIterator<Item = T>,
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
    fn test_range() {
        let source = 100..103;

        let observable = Range::new(source);
        let checker = CheckingObserver::new();

        let subscription = observable.subscribe(checker.clone());
        assert!(checker.is_values_matched(&[100, 101, 102]));
        assert!(checker.is_completed());

        _ = subscription; // keep the subscription alive
    }

    #[test]
    fn test_range_inclusive() {
        let source = 100..=103;

        let observable = Range::new(source);
        let checker = CheckingObserver::new();

        let subscription = observable.subscribe(checker.clone());
        assert!(checker.is_values_matched(&[100, 101, 102, 103]));
        assert!(checker.is_completed());

        _ = subscription; // keep the subscription alive
    }

    #[test]
    fn test_clone() {
        let source = 100..103;
        let observable = Range::new(source);
        let _ = observable.clone();
    }
}
