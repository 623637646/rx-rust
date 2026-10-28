//! The basics: build a pipeline from operators, then subscribe to it. Everything here runs
//! synchronously, so no async runtime is needed.
//!
//! ```bash
//! cargo run --example basics
//! ```

use rx_rust::{observable::ObservableExt, operators::creating::range::Range};

fn main() {
    // A synchronous source has sent everything by the time `subscribe` returns, so its
    // subscription can be dropped at once. A source that sends later must be kept subscribed.
    let _ = Range::new(1..=10)
        .filter(|n| n % 2 == 0) // 2, 4, 6, 8, 10
        .map(|n| n * n) // 4, 16, 36, 64, 100
        .take(3) // 4, 16, 36
        .subscribe_with_callback(
            |value| println!("next: {value}"),
            |termination| println!("done: {termination:?}"),
        );
}
