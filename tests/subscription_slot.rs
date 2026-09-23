mod tests_utils;

use rx_rust::utils::subscription_slot::SubscriptionSlot;
use tests_utils::drop_probe::DropCount;

#[test]
fn test_release_before_fill_keeps_reservation() {
    let drops = DropCount::new();
    let mut slot = SubscriptionSlot::Idle;
    assert!(slot.reserve_if_idle());

    assert!(slot.release().is_none());
    assert!(!slot.is_idle());
    assert!(slot.is_reserved());
    assert!(!slot.reserve_if_idle());

    let finished = slot.fill(drops.probe());
    assert!(finished.is_some());
    assert!(slot.is_idle());
    assert_eq!(drops.get(), 0);

    // A later build is independent of the finished subscription returned to the caller.
    assert!(slot.reserve_if_idle());
    assert!(slot.fill(drops.probe()).is_none());
    drop(finished);
    assert_eq!(drops.get(), 1);
    assert!(!slot.is_idle());
    assert!(!slot.is_reserved());

    let finished = slot.release();
    assert!(finished.is_some());
    assert!(slot.is_idle());
    assert_eq!(drops.get(), 1);
    drop(finished);
    assert_eq!(drops.get(), 2);
}

#[test]
fn test_fill_before_release_hands_back_subscription() {
    let drops = DropCount::new();
    let mut slot = SubscriptionSlot::Idle;
    assert!(slot.reserve_if_idle());
    assert!(slot.fill(drops.probe()).is_none());
    assert!(!slot.reserve_if_idle());

    let finished = slot.release();
    assert!(finished.is_some());
    assert!(slot.is_idle());
    assert_eq!(drops.get(), 0);
    drop(finished);
    assert_eq!(drops.get(), 1);
}

#[test]
fn test_reserve_replacing_active_subscription() {
    let drops = DropCount::new();
    let mut slot = SubscriptionSlot::Idle;
    assert!(slot.reserve_replacing().is_none());
    assert!(slot.fill(drops.probe()).is_none());

    let evicted = slot.reserve_replacing();
    assert!(evicted.is_some());
    assert!(slot.is_reserved());
    assert_eq!(drops.get(), 0);
    drop(evicted);
    assert_eq!(drops.get(), 1);

    assert!(slot.release().is_none());
    let finished = slot.fill(drops.probe());
    assert!(finished.is_some());
    assert!(slot.is_idle());
    drop(finished);
    assert_eq!(drops.get(), 2);
}

#[test]
#[should_panic(expected = "the slot is already reserved")]
fn test_reserve_replacing_rejects_released_build_in_flight() {
    let mut slot = SubscriptionSlot::<()>::Idle;
    assert!(slot.reserve_if_idle());
    assert!(slot.release().is_none());
    slot.reserve_replacing();
}

#[test]
#[should_panic(expected = "the slot was released without being reserved")]
fn test_release_rejects_idle_slot() {
    SubscriptionSlot::<()>::Idle.release();
}

#[test]
fn test_release_rejects_released_build_in_flight() {
    let mut slot = SubscriptionSlot::<()>::Idle;
    assert!(slot.reserve_if_idle());
    assert!(slot.release().is_none());

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| slot.release()));
    assert!(result.is_err());
    // Reject the duplicate before changing state: catching the panic must not free the slot.
    assert!(matches!(slot, SubscriptionSlot::ReleasedWhileReserved));
    assert!(!slot.reserve_if_idle());
    assert_eq!(slot.fill(()), Some(()));
    assert!(slot.is_idle());
}

#[test]
#[should_panic(expected = "the slot was released without being reserved")]
fn test_release_rejects_already_released_subscription() {
    let mut slot = SubscriptionSlot::<()>::Idle;
    assert!(slot.reserve_if_idle());
    assert!(slot.fill(()).is_none());
    assert_eq!(slot.release(), Some(()));
    slot.release();
}
