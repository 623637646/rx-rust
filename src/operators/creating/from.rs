use crate::{
    observable::Observable,
    observer::{Observer, Terminal},
    subscription::Subscription,
};
use std::convert::Infallible;

#[derive(Clone)]
pub struct From<IT>(IT);

impl<IT> From<IT> {
    pub fn new(into_iterator: IT) -> Self
    where
        IT: IntoIterator,
    {
        Self(into_iterator)
    }
}

impl<'a, T, OR, IT> Observable<'a, T, Infallible, OR> for From<IT>
where
    OR: Observer<T, Infallible>,
    IT: IntoIterator<Item = T>,
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
        observable::observable_subscribe_ext::ObservableSubscribeExt,
        utils::tests_utils::checking_observer::CheckingObserver,
    };

    #[test]
    fn test_array() {
        let source = [1, 2, 3];

        let observable = From::new(source);
        let checker = CheckingObserver::new();

        let subscription = observable.subscribe(checker.clone());
        assert!(checker.is_values_matched(&[1, 2, 3]));
        assert!(checker.is_completed());

        drop(subscription); // keep the subscription alive
    }

    #[test]
    fn test_array_ref() {
        let source = [1, 2, 3];

        let observable = From::new(&source);
        let checker = CheckingObserver::new();

        let subscription = observable.subscribe(checker.clone());
        assert!(checker.is_values_matched(&[&1, &2, &3]));
        assert!(checker.is_completed());

        drop(subscription); // keep the subscription alive
    }

    #[test]
    fn test_array_mut() {
        let mut source = [1, 2, 3];

        let observable = From::new(&mut source);

        let subscription = observable.subscribe_on(
            |value| {
                *value *= 2;
            },
            |terminal| assert!(matches!(terminal, Terminal::Completed)),
        );
        assert_eq!(source, [2, 4, 6]);

        drop(subscription); // keep the subscription alive
    }

    #[test]
    fn test_slice() {
        let source: &[i32] = &[1, 2, 3];

        let observable = From::new(source);
        let checker = CheckingObserver::new();

        let subscription = observable.subscribe(checker.clone());
        assert!(checker.is_values_matched(&[&1, &2, &3]));
        assert!(checker.is_completed());

        drop(subscription); // keep the subscription alive
    }

    #[test]
    fn test_slice_mut() {
        let mut data = [1, 2, 3];
        let source: &mut [i32] = &mut data;

        let observable = From::new(source);

        let subscription = observable.subscribe_on(
            |value| {
                *value *= 2;
            },
            |terminal| assert!(matches!(terminal, Terminal::Completed)),
        );
        assert_eq!(data, [2, 4, 6]);

        drop(subscription); // keep the subscription alive
    }

    #[test]
    fn test_vec() {
        let source = vec![1, 2, 3];

        let observable = From::new(source);
        let checker = CheckingObserver::new();

        let subscription = observable.subscribe(checker.clone());
        assert!(checker.is_values_matched(&[1, 2, 3]));
        assert!(checker.is_completed());

        drop(subscription); // keep the subscription alive
    }

    #[test]
    fn test_vec_ref() {
        let source = vec![1, 2, 3];

        let observable = From::new(&source);
        let checker = CheckingObserver::new();

        let subscription = observable.subscribe(checker.clone());
        assert!(checker.is_values_matched(&[&1, &2, &3]));
        assert!(checker.is_completed());

        drop(subscription); // keep the subscription alive
    }

    #[test]
    fn test_vec_mut() {
        let mut source = vec![1, 2, 3];

        let observable = From::new(&mut source);

        let subscription = observable.subscribe_on(
            |value| {
                *value *= 2;
            },
            |terminal| assert!(matches!(terminal, Terminal::Completed)),
        );
        assert_eq!(source, [2, 4, 6]);

        drop(subscription); // keep the subscription alive
    }

    #[test]
    fn test_range() {
        let source = 100..103;

        let observable = From::new(source);
        let checker = CheckingObserver::new();

        let subscription = observable.subscribe(checker.clone());
        assert!(checker.is_values_matched(&[100, 101, 102]));
        assert!(checker.is_completed());

        drop(subscription); // keep the subscription alive
    }

    #[test]
    fn test_ref() {
        let v1 = 1;
        let v2 = 2;
        let v3 = 3;
        let source = [&v1, &v2, &v3];

        let observable = From::new(source);
        let checker = CheckingObserver::new();

        let subscription = observable.subscribe(checker.clone());
        assert!(checker.is_values_matched(&[&v1, &v2, &v3]));
        assert!(checker.is_completed());

        drop(subscription); // keep the subscription alive
    }

    #[test]
    fn test_mut_ref() {
        let mut v1 = 1;
        let mut v2 = 2;
        let mut v3 = 3;
        let source = [&mut v1, &mut v2, &mut v3];

        let observable = From::new(source);

        let subscription = observable.subscribe_on(
            |value| {
                *value *= 2;
            },
            |terminal| assert!(matches!(terminal, Terminal::Completed)),
        );
        assert_eq!(v1, 2);
        assert_eq!(v2, 4);
        assert_eq!(v3, 6);

        drop(subscription); // keep the subscription alive
    }

    #[test]
    fn test_subscribe_by_different_observer() {
        let source = [1, 2, 3];

        let observable = From::new(source);
        let observable_1 = observable;
        let observable_2 = observable_1.clone();

        let checker_1 = CheckingObserver::new();
        let checker_2 = CheckingObserver::new();

        let subscription_1 = observable_1.subscribe(checker_1.clone());

        let (on_next, on_terminal) = checker_2.fn_for_subscribe_on();
        let subscription_2 = observable_2.subscribe_on(on_next, on_terminal);

        assert!(checker_1.is_values_matched(&[1, 2, 3]));
        assert!(checker_1.is_completed());
        assert!(checker_2.is_values_matched(&[1, 2, 3]));
        assert!(checker_2.is_completed());

        drop(subscription_1); // keep the subscription alive
        drop(subscription_2); // keep the subscription alive
    }
}
