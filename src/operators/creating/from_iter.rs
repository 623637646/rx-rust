use crate::{
    observable::Observable,
    observer::{Observer, Terminal},
    subscription::Subscription,
};
use educe::Educe;
use std::convert::Infallible;

#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct FromIter<I>(I);

impl<I> FromIter<I> {
    pub fn new(into_iterator: I) -> Self
    where
        I: IntoIterator,
    {
        Self(into_iterator)
    }
}

impl<'a, T, OR, I> Observable<'a, T, Infallible, OR> for FromIter<I>
where
    OR: Observer<T, Infallible>,
    I: IntoIterator<Item = T>,
{
    fn subscribe(self, mut observer: OR) -> Subscription<'a> {
        for value in self.0.into_iter() {
            observer.on_next(value);
        }
        observer.on_terminal(Terminal::Completed);
        Subscription::new_none_disposal()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        observable::observable_ext::ObservableExt,
        utils::tests_utils::checking_observer::CheckingObserver,
    };

    #[test]
    fn test_completed_array() {
        let source = [1, 2, 3];

        let observable = FromIter::new(source);
        let checker = CheckingObserver::new();

        let subscription = observable.subscribe(checker.clone());
        assert!(checker.is_values_matched(&[1, 2, 3]));
        assert!(checker.is_completed());

        _ = subscription; // keep the subscription alive
    }

    #[test]
    fn test_completed_array_ref() {
        let source = [1, 2, 3];

        let observable = FromIter::new(&source);
        let checker = CheckingObserver::new();

        let subscription = observable.subscribe(checker.clone());
        assert!(checker.is_values_matched(&[&1, &2, &3]));
        assert!(checker.is_completed());

        _ = subscription; // keep the subscription alive
    }

    #[test]
    fn test_completed_array_mut() {
        let mut source = [1, 2, 3];

        let observable = FromIter::new(&mut source);

        let subscription = observable.subscribe_with_callback(
            |value| {
                *value *= 2;
            },
            |terminal| assert!(matches!(terminal, Terminal::Completed)),
        );
        assert_eq!(source, [2, 4, 6]);

        _ = subscription; // keep the subscription alive
    }

    #[test]
    fn test_completed_slice() {
        let source: &[i32] = &[1, 2, 3];

        let observable = FromIter::new(source);
        let checker = CheckingObserver::new();

        let subscription = observable.subscribe(checker.clone());
        assert!(checker.is_values_matched(&[&1, &2, &3]));
        assert!(checker.is_completed());

        _ = subscription; // keep the subscription alive
    }

    #[test]
    fn test_completed_slice_mut() {
        let mut data = [1, 2, 3];
        let source: &mut [i32] = &mut data;

        let observable = FromIter::new(source);

        let subscription = observable.subscribe_with_callback(
            |value| {
                *value *= 2;
            },
            |terminal| assert!(matches!(terminal, Terminal::Completed)),
        );
        assert_eq!(data, [2, 4, 6]);

        _ = subscription; // keep the subscription alive
    }

    #[test]
    fn test_completed_vec() {
        let source = vec![1, 2, 3];

        let observable = FromIter::new(source);
        let checker = CheckingObserver::new();

        let subscription = observable.subscribe(checker.clone());
        assert!(checker.is_values_matched(&[1, 2, 3]));
        assert!(checker.is_completed());

        _ = subscription; // keep the subscription alive
    }

    #[test]
    fn test_completed_vec_ref() {
        let source = vec![1, 2, 3];

        let observable = FromIter::new(&source);
        let checker = CheckingObserver::new();

        let subscription = observable.subscribe(checker.clone());
        assert!(checker.is_values_matched(&[&1, &2, &3]));
        assert!(checker.is_completed());

        _ = subscription; // keep the subscription alive
    }

    #[test]
    fn test_completed_vec_mut() {
        let mut source = vec![1, 2, 3];

        let observable = FromIter::new(&mut source);

        let subscription = observable.subscribe_with_callback(
            |value| {
                *value *= 2;
            },
            |terminal| assert!(matches!(terminal, Terminal::Completed)),
        );
        assert_eq!(source, [2, 4, 6]);

        _ = subscription; // keep the subscription alive
    }

    #[test]
    fn test_completed_range() {
        let source = 100..103;

        let observable = FromIter::new(source);
        let checker = CheckingObserver::new();

        let subscription = observable.subscribe(checker.clone());
        assert!(checker.is_values_matched(&[100, 101, 102]));
        assert!(checker.is_completed());

        _ = subscription; // keep the subscription alive
    }

    #[test]
    fn test_ref() {
        let v1 = 1;
        let v2 = 2;
        let v3 = 3;
        let source = [&v1, &v2, &v3];

        let observable = FromIter::new(source);
        let checker = CheckingObserver::new();

        let subscription = observable.subscribe(checker.clone());
        assert!(checker.is_values_matched(&[&v1, &v2, &v3]));
        assert!(checker.is_completed());

        _ = subscription; // keep the subscription alive
    }

    #[test]
    fn test_mut_ref() {
        let mut v1 = 1;
        let mut v2 = 2;
        let mut v3 = 3;
        let source = [&mut v1, &mut v2, &mut v3];

        let observable = FromIter::new(source);

        let subscription = observable.subscribe_with_callback(
            |value| {
                *value *= 2;
            },
            |terminal| assert!(matches!(terminal, Terminal::Completed)),
        );
        assert_eq!(v1, 2);
        assert_eq!(v2, 4);
        assert_eq!(v3, 6);

        _ = subscription; // keep the subscription alive
    }

    #[tokio::test]
    async fn test_async() {
        let source = vec![1, 2, 3];
        let observable = FromIter::new(source);
        let checker: CheckingObserver<i32, Infallible> = CheckingObserver::new();

        let checker_cloned = checker.clone();
        let handle = tokio::spawn(async move { observable.subscribe(checker_cloned) });
        let subscription = handle.await.unwrap();
        assert!(checker.is_values_matched(&[1, 2, 3]));
        assert!(checker.is_completed());

        let handle = tokio::spawn(async { subscription.unsubscribe() });
        handle.await.unwrap();
        assert!(checker.is_values_matched(&[1, 2, 3]));
        assert!(checker.is_completed());
    }

    #[test]
    fn test_subscribe_by_different_observer() {
        let source = [1, 2, 3];

        let observable = FromIter::new(source);
        let observable_1 = observable;
        let observable_2 = observable_1.clone();

        let checker_1 = CheckingObserver::new();
        let checker_2 = CheckingObserver::new();

        let subscription_1 = observable_1.subscribe(checker_1.clone());

        let (on_next, on_terminal) = checker_2.clone().into_callbacks();
        let subscription_2 = observable_2.subscribe_with_callback(on_next, on_terminal);

        assert!(checker_1.is_values_matched(&[1, 2, 3]));
        assert!(checker_1.is_completed());
        assert!(checker_2.is_values_matched(&[1, 2, 3]));
        assert!(checker_2.is_completed());

        _ = subscription_1; // keep the subscription alive
        _ = subscription_2; // keep the subscription alive
    }

    #[test]
    fn test_clone() {
        let source = [1, 2, 3];
        let observable = FromIter::new(source);
        let _ = observable.clone();
    }
}
