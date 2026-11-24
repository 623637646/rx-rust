use super::LazySubscription;
use crate::{observer::callback_observer::CallbackObserver, operators::creating::range::Range};

#[test]
fn test_subscribe_once_then_release() {
    let mut seen = Vec::new();
    let mut subscription = LazySubscription::new(Range::new(1..4));
    assert!(matches!(subscription, LazySubscription::Source(_))); // Nothing subscribed yet.

    subscription.subscribe_once(|| CallbackObserver::new(|value| seen.push(value), |_| {}));
    assert!(matches!(subscription, LazySubscription::Subscribed(_)));

    // Every later call is a no-op: the source is not subscribed to again.
    subscription.subscribe_once(|| CallbackObserver::new(|value| seen.push(value), |_| {}));

    subscription.release(); // Disposes the subscription.
    assert!(matches!(subscription, LazySubscription::Released));
    assert_eq!(seen, [1, 2, 3]);
}
