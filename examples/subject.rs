//! A subject is a source that you push values into yourself, and that sends each of them to
//! every current subscriber.
//!
//! ```bash
//! cargo run --example subject
//! ```

use rx_rust::{
    observable::ObservableExt,
    observer::{Observer, Termination},
    subject::publish_subject::PublishSubject,
};
use std::convert::Infallible;

fn main() {
    let subject = PublishSubject::<_, Infallible, _>::local();
    let mut sender = subject.clone();

    let first = subject
        .clone()
        .subscribe_with_callback(|value| println!("first:  {value}"), |_| {});
    let _ = sender.on_next("hello"); // only `first` is subscribed

    let _second = subject.subscribe_with_callback(|value| println!("second: {value}"), |_| {});
    let _ = sender.on_next("world"); // both are

    drop(first); // dropping a subscription unsubscribes
    let _ = sender.on_next("bye"); // only `second` is left

    sender.on_termination(Termination::Completed);
}
