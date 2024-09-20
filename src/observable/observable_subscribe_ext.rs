// use super::Observable;
// use crate::{
//     observer::{anonymous_observer::AnonymousObserver, Observer, Terminal},
//     utils::disposal::Disposal,
// };

// /// Extension trait for `Observable`
// pub trait ObservableSubscribeExt<T, E, ObserverType> {
//     /**
//     Subscribes to the observable with the given `on_event` callback.

//     # Example
//     ```rust
//     use rx_rust::{
//         observable::observable_subscribe_ext::ObservableSubscribeExt, operators::just::Just,
//     };
//     use rx_rust::utils::never::Never;
//     use rx_rust::observer::event::Event;
//     let observable = Just::new(123);
//     observable.subscribe_on_event(move |event: Event<i32, Never>| {
//         println!("{:?}", event);
//     });
//     ```
//     */
//     fn subscribe_on(
//         self,
//         on_next: impl Fn(T) + Sync + Send + 'static,
//         on_terminal: impl FnOnce(Terminal<E>) + Sync + Send + 'static,
//     ) -> Disposal;

//     /**
//     Subscribes to the observable with the given `on_next` callback.

//     # Example
//     ```rust
//     use rx_rust::{
//         observable::observable_subscribe_ext::ObservableSubscribeExt, operators::just::Just,
//     };
//     let observable = Just::new(123);
//     observable.subscribe_on_next(move |value: i32| {
//         println!("{:?}", value);
//     });
//     ```
//     */
//     fn subscribe_on_next(self, on_next: impl Fn(T) + Sync + Send + 'static) -> Disposal;
// }

// impl<T, E, O, ObserverType> ObservableSubscribeExt<T, E, ObserverType> for O
// where
//     O: Observable<T, E, ObserverType>,
//     ObserverType: Observer<T, E>,
// {
//     fn subscribe_on(
//         self,
//         on_next: impl Fn(T) + Sync + Send + 'static,
//         on_terminal: impl FnOnce(Terminal<E>) + Sync + Send + 'static,
//     ) -> Disposal {
//         let observer = AnonymousObserver::new(on_next, on_terminal);
//         self.subscribe(observer)
//     }

//     fn subscribe_on_next(self, on_next: impl Fn(T) + Sync + Send + 'static) -> Disposal {
//         self.subscribe_on(on_next, |_| {})
//     }
// }

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use crate::{
//         observer::Observer, operators::just::Just, utils::checking_observer::CheckingObserver,
//     };

//     #[test]
//     fn test_on_event() {
//         let observable = Just::new(123);
//         let checker = CheckingObserver::new();
//         let checker_cloned = checker.clone();
//         observable.subscribe_on_event(move |event| {
//             checker_cloned.notify_if_unterminated(event);
//         });
//         assert!(checker.is_values_matched(&[123]));
//         assert!(checker.is_completed());
//     }

//     #[test]
//     fn test_on_next() {
//         let observable = Just::new(123);
//         let checker = CheckingObserver::<i32, String>::new();
//         let checker_cloned = checker.clone();
//         observable.subscribe_on_next(move |value| {
//             checker_cloned.notify_if_unterminated(Event::Next(value));
//         });
//         assert!(checker.is_values_matched(&[123]));
//         assert!(checker.is_unterminated());
//     }
// }
