use super::from::From;
use std::iter::RepeatN;

pub fn repeat<T>(value: T, n: usize) -> From<RepeatN<T>>
where
    T: Clone,
{
    From::new(std::iter::repeat_n(value, n))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{observable::Observable, utils::tests_utils::checking_observer::CheckingObserver};

    #[test]
    fn test_repeat() {
        let observable = repeat(3, 4);
        let checker = CheckingObserver::new();

        let subscription = observable.subscribe(checker.clone());
        assert!(checker.is_values_matched(&[3, 3, 3, 3]));
        assert!(checker.is_completed());

        _ = subscription; // keep the subscription alive
    }
}
