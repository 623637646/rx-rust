//! Retrying a failed request: after an error, `retry` decides whether to subscribe again.
//!
//! ```bash
//! cargo run --example retry
//! ```

use rx_rust::{
    observable::ObservableExt,
    operators::{creating::from_try_future::FromTryFuture, error_handling::retry::RetryAction},
    scheduler::runtime::tokio::TokioScheduler,
};

/// A server that fails twice, then answers.
async fn fetch(attempt: u32) -> Result<String, String> {
    if attempt < 3 {
        Err(format!("attempt {attempt} failed"))
    } else {
        Ok(format!("attempt {attempt} succeeded"))
    }
}

#[tokio::main]
async fn main() {
    let scheduler = TokioScheduler::current();
    let mut attempt = 0;
    let mut request = move || {
        attempt += 1;
        FromTryFuture::new(fetch(attempt), scheduler.clone())
    };

    let mut retries = 0;
    let response = request()
        .retry(move |error| {
            println!("{error}");
            retries += 1;
            if retries <= 3 {
                RetryAction::Retry(request())
            } else {
                RetryAction::Stop(error)
            }
        })
        // Await the pipeline like any future.
        .into_try_future()
        .await;

    println!("{response:?}"); // Ok(Some("attempt 3 succeeded"))
}
