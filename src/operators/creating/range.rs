use super::from::From;

pub type Range<I> = From<I>;

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
