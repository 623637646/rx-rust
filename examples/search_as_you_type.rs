//! Search as you type: search only once the user pauses, and drop the answer of a search that a
//! newer query has made stale.
//!
//! ```bash
//! cargo run --example search_as_you_type
//! ```

use rx_rust::{
    observable::ObservableExt, observer::Observer, operators::creating::from_future::FromFuture,
    scheduler::runtime::tokio::TokioScheduler, subject::publish_subject::PublishSubject,
};
use std::time::Duration;
use tokio::time::sleep;

/// A slow search backend.
async fn search(query: &str) -> String {
    println!("searching {query:?}");
    sleep(Duration::from_secs(1)).await;
    format!("results for {query:?}")
}

#[tokio::main]
async fn main() {
    let scheduler = TokioScheduler::current();
    let input = PublishSubject::shared();

    let search_scheduler = scheduler.clone();
    let _subscription = input
        .clone()
        // Wait until the user has stopped typing for 300 ms.
        .debounce(Duration::from_millis(300), scheduler)
        // A new query cancels the search still running.
        .switch_map(move |query| FromFuture::new(search(query), search_scheduler.clone()))
        .subscribe_with_callback(|results| println!("{results}"), |_| {});

    // What the user types, and how long they pause after it.
    let mut keyboard = input;
    for (text, pause) in [("r", 100), ("ru", 500), ("rus", 100), ("rust", 1500)] {
        println!("typed {text:?}");
        let _ = keyboard.on_next(text);
        sleep(Duration::from_millis(pause)).await;
    }
    // "ru" is searched during the pause, then cancelled by "rust": only "rust" gets results.
}
