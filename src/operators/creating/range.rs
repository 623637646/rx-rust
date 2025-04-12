use super::from_iter::FromIter;
use std::ops::RangeBounds;

pub fn range<T, R>(range: R) -> FromIter<R>
where
    R: IntoIterator<Item = T> + RangeBounds<T>,
{
    FromIter::new(range)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{observable::Observable, utils::tests_utils::checking_observer::CheckingObserver};

    #[test]
    fn test_range() {
        let source = 100..103;

        let observable = range(source);
        let checker = CheckingObserver::new();

        let subscription = observable.subscribe(checker.clone());
        assert!(checker.is_values_matched(&[100, 101, 102]));
        assert!(checker.is_completed());

        _ = subscription; // keep the subscription alive
    }

    #[test]
    fn test_range_inclusive() {
        let source = 100..=103;

        let observable = range(source);
        let checker = CheckingObserver::new();

        let subscription = observable.subscribe(checker.clone());
        assert!(checker.is_values_matched(&[100, 101, 102, 103]));
        assert!(checker.is_completed());

        _ = subscription; // keep the subscription alive
    }

    #[test]
    fn test_clone() {
        let source = 100..103;
        let observable = range(source);
        let _ = observable.clone();
    }
}
