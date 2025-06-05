mod tests_utils;

use rx_rust::observer::{Observer, Termination, observer_collection::ObserverCollection};
use std::convert::Infallible;
use tests_utils::checker::Checker;

#[test]
fn test_completed() {
    let mut observers = ObserverCollection::default();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    let _key1 = observers.insert(observer_1);
    let mut agent = observers.borrow_agent().unwrap();
    agent.on_next(1);
    observers.return_agent(agent);
    assert_eq!(checker_1.values(), [1]);
    assert!(checker_1.is_active());
    assert!(checker_2.values().is_empty());
    assert!(checker_2.is_active());

    let key2 = observers.insert(observer_2);
    let mut agent = observers.borrow_agent().unwrap();
    agent.on_next(2);
    observers.return_agent(agent);
    assert_eq!(checker_1.values(), [1, 2]);
    assert!(checker_1.is_active());
    assert_eq!(checker_2.values(), [2]);
    assert!(checker_2.is_active());

    assert!(observers.remove(key2).is_some());
    let mut agent = observers.borrow_agent().unwrap();
    agent.on_next(3);
    observers.return_agent(agent);
    assert_eq!(checker_1.values(), [1, 2, 3]);
    assert!(checker_1.is_active());
    assert_eq!(checker_2.values(), [2]);
    assert!(checker_2.is_dropped());

    let agent = observers.borrow_agent().unwrap();
    agent.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker_1.values(), [1, 2, 3]);
    assert!(checker_1.is_completed());
    assert_eq!(checker_2.values(), [2]);
    assert!(checker_2.is_dropped());
}

#[test]
fn test_error() {
    let mut observers = ObserverCollection::default();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    let _key1 = observers.insert(observer_1);
    let mut agent = observers.borrow_agent().unwrap();
    agent.on_next(1);
    observers.return_agent(agent);
    assert_eq!(checker_1.values(), [1]);
    assert!(checker_1.is_active());
    assert!(checker_2.values().is_empty());
    assert!(checker_2.is_active());

    let key2 = observers.insert(observer_2);
    let mut agent = observers.borrow_agent().unwrap();
    agent.on_next(2);
    observers.return_agent(agent);
    assert_eq!(checker_1.values(), [1, 2]);
    assert!(checker_1.is_active());
    assert_eq!(checker_2.values(), [2]);
    assert!(checker_2.is_active());

    assert!(observers.remove(key2).is_some());
    let mut agent = observers.borrow_agent().unwrap();
    agent.on_next(3);
    observers.return_agent(agent);
    assert_eq!(checker_1.values(), [1, 2, 3]);
    assert!(checker_1.is_active());
    assert_eq!(checker_2.values(), [2]);
    assert!(checker_2.is_dropped());

    let agent = observers.borrow_agent().unwrap();
    agent.on_termination(Termination::Error("error"));
    assert_eq!(checker_1.values(), [1, 2, 3]);
    assert!(checker_1.is_error("error"));
    assert_eq!(checker_2.values(), [2]);
    assert!(checker_2.is_dropped());
}

#[test]
fn test_remove_all() {
    let mut observers = ObserverCollection::default();
    let (checker_1, observer_1) = Checker::<_, Infallible>::new();
    let (checker_2, observer_2) = Checker::new();

    let key1 = observers.insert(observer_1);
    let mut agent = observers.borrow_agent().unwrap();
    agent.on_next(1);
    observers.return_agent(agent);
    assert_eq!(checker_1.values(), [1]);
    assert!(checker_1.is_active());
    assert!(checker_2.values().is_empty());
    assert!(checker_2.is_active());

    let key2 = observers.insert(observer_2);
    let mut agent = observers.borrow_agent().unwrap();
    agent.on_next(2);
    observers.return_agent(agent);
    assert_eq!(checker_1.values(), [1, 2]);
    assert!(checker_1.is_active());
    assert_eq!(checker_2.values(), [2]);
    assert!(checker_2.is_active());

    assert!(observers.remove(key2).is_some());
    let mut agent = observers.borrow_agent().unwrap();
    agent.on_next(3);
    observers.return_agent(agent);
    assert_eq!(checker_1.values(), [1, 2, 3]);
    assert!(checker_1.is_active());
    assert_eq!(checker_2.values(), [2]);
    assert!(checker_2.is_dropped());

    assert!(observers.remove(key1).is_some());
    let mut agent = observers.borrow_agent().unwrap();
    agent.on_next(4);
    observers.return_agent(agent);
    assert_eq!(checker_1.values(), [1, 2, 3]);
    assert!(checker_1.is_dropped());
    assert_eq!(checker_2.values(), [2]);
    assert!(checker_2.is_dropped());
}

#[test]
fn test_insert_between_using_agent() {
    let mut observers = ObserverCollection::default();
    let (checker_1, observer_1) = Checker::<_, Infallible>::new();
    let (checker_2, observer_2) = Checker::new();

    let key1 = observers.insert(observer_1);
    let mut agent = observers.borrow_agent().unwrap();
    agent.on_next(1);
    let key2 = observers.insert(observer_2);
    observers.return_agent(agent);
    assert_eq!(checker_1.values(), [1]);
    assert!(checker_1.is_active());
    assert!(checker_2.values().is_empty());
    assert!(checker_2.is_active());

    let mut agent = observers.borrow_agent().unwrap();
    agent.on_next(2);
    observers.return_agent(agent);
    assert_eq!(checker_1.values(), [1, 2]);
    assert!(checker_1.is_active());
    assert_eq!(checker_2.values(), [2]);
    assert!(checker_2.is_active());

    assert!(observers.remove(key2).is_some());
    let mut agent = observers.borrow_agent().unwrap();
    agent.on_next(3);
    observers.return_agent(agent);
    assert_eq!(checker_1.values(), [1, 2, 3]);
    assert!(checker_1.is_active());
    assert_eq!(checker_2.values(), [2]);
    assert!(checker_2.is_dropped());

    assert!(observers.remove(key1).is_some());
    let mut agent = observers.borrow_agent().unwrap();
    agent.on_next(4);
    observers.return_agent(agent);
    assert_eq!(checker_1.values(), [1, 2, 3]);
    assert!(checker_1.is_dropped());
    assert_eq!(checker_2.values(), [2]);
    assert!(checker_2.is_dropped());
}

#[test]
fn test_remove_between_using_agent() {
    let mut observers = ObserverCollection::default();
    let (checker_1, observer_1) = Checker::<_, Infallible>::new();
    let (checker_2, observer_2) = Checker::new();

    let key1 = observers.insert(observer_1);
    let mut agent = observers.borrow_agent().unwrap();
    agent.on_next(1);
    observers.return_agent(agent);
    assert_eq!(checker_1.values(), [1]);
    assert!(checker_1.is_active());
    assert!(checker_2.values().is_empty());
    assert!(checker_2.is_active());

    let key2 = observers.insert(observer_2);
    let mut agent = observers.borrow_agent().unwrap();
    agent.on_next(2);
    observers.return_agent(agent);
    assert_eq!(checker_1.values(), [1, 2]);
    assert!(checker_1.is_active());
    assert_eq!(checker_2.values(), [2]);
    assert!(checker_2.is_active());

    let mut agent = observers.borrow_agent().unwrap();
    assert!(observers.remove(key2).is_none());
    agent.on_next(-1);
    observers.return_agent(agent);
    assert_eq!(checker_1.values(), [1, 2, -1]);
    assert!(checker_1.is_active());
    assert_eq!(checker_2.values(), [2, -1]);
    assert!(checker_2.is_dropped());

    let mut agent = observers.borrow_agent().unwrap();
    agent.on_next(3);
    observers.return_agent(agent);
    assert_eq!(checker_1.values(), [1, 2, -1, 3]);
    assert!(checker_1.is_active());
    assert_eq!(checker_2.values(), [2, -1]);
    assert!(checker_2.is_dropped());

    assert!(observers.remove(key1).is_some());
    let mut agent = observers.borrow_agent().unwrap();
    agent.on_next(4);
    observers.return_agent(agent);
    assert_eq!(checker_1.values(), [1, 2, -1, 3]);
    assert!(checker_1.is_dropped());
    assert_eq!(checker_2.values(), [2, -1]);
    assert!(checker_2.is_dropped());
}
