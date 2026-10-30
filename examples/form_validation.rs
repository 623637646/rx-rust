//! Form validation: the submit button follows the latest value of every field.
//!
//! The subjects are `local()`: everything stays on this thread, so nothing needs to be `Send`
//! and no mutex is needed.
//!
//! ```bash
//! cargo run --example form_validation
//! ```

use rx_rust::{
    observable::ObservableExt, observer::Observer, subject::behavior_subject::BehaviorSubject,
};
use std::convert::Infallible;

fn main() {
    // A `BehaviorSubject` holds the current value of a field, empty to begin with.
    let email = BehaviorSubject::local("");
    let password = BehaviorSubject::<_, Infallible, _>::local("");

    let _subscription = email
        .clone()
        .combine_latest(password.clone())
        .map(|(email, password)| email.contains('@') && password.len() >= 8)
        // Report the button only when its state changes, not on every keystroke.
        .distinct_until_changed()
        .subscribe_with_callback(|valid| println!("can submit: {valid}"), |_| {});

    let mut email_field = email;
    let mut password_field = password;
    let _ = email_field.on_next("ann@example.com");
    let _ = password_field.on_next("hunter");
    let _ = password_field.on_next("hunter22"); // can submit: true
    let _ = password_field.on_next("hunter222"); // still true: nothing printed
    let _ = password_field.on_next("hunt"); // can submit: false
}
