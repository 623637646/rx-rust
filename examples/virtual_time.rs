//! Testing time-based code without waiting: on `VirtualTime`, the clock moves only when the test
//! moves it, and seconds pass instantly.
//!
//! ```bash
//! cargo run --example virtual_time
//! ```

use rx_rust::{
    observable::ObservableExt, observer::Observer, scheduler::virtual_time::VirtualTime,
    subject::publish_subject::PublishSubject,
};
use std::{
    convert::Infallible,
    sync::{Arc, Mutex},
    time::Duration,
};

fn main() {
    let time = VirtualTime::new();
    let edits = PublishSubject::<_, Infallible, _>::shared();
    let mut editor = edits.clone();

    // Autosave: save once the user has stopped editing for two seconds.
    let saved = Arc::new(Mutex::new(Vec::new()));
    let saved_observer = Arc::clone(&saved);
    let _subscription = edits
        .debounce(Duration::from_secs(2), time.scheduler())
        .subscribe_with_callback(
            move |document| saved_observer.lock().unwrap().push(document),
            |_| {},
        );

    // An edit every second: nothing is saved while the user keeps typing.
    for document in ["H", "He", "Hel", "Hell", "Hello"] {
        let _ = editor.on_next(document);
        time.advance_by(Duration::from_secs(1));
    }
    assert!(saved.lock().unwrap().is_empty());

    // Two seconds after the last edit.
    time.advance_by(Duration::from_secs(1));
    assert_eq!(*saved.lock().unwrap(), ["Hello"]);

    println!("saved {:?}, without waiting", saved.lock().unwrap());
}
