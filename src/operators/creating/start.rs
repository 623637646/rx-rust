use super::just::Just;

pub fn start<T, F>(f: F) -> Just<T>
where
    F: FnOnce() -> T,
{
    Just::new(f())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{observable::Observable, utils::tests_utils::checking_observer::CheckingObserver};

    #[test]
    fn test_start() {
        let value = 111;
        let observable = start(|| value + 222);
        let checker = CheckingObserver::new();

        let subscription = observable.subscribe(checker.clone());
        assert!(checker.is_values_matched(&[333]));
        assert!(checker.is_completed());

        _ = subscription; // keep the subscription alive
    }
}
