use std::panic::{AssertUnwindSafe, catch_unwind, resume_unwind};

struct PanicOnDropPayload;

/// A value whose drop unwinds, which is how a test panics from inside the code under test.
///
/// It unwinds with [`resume_unwind`], which does not run the panic hook, so an expected panic
/// prints nothing; `std::thread::panicking` is still `true` throughout the unwind.
pub(crate) struct PanicOnDrop(());

impl Drop for PanicOnDrop {
    fn drop(&mut self) {
        resume_unwind(Box::new(PanicOnDropPayload));
    }
}

/// Runs `callback` with a [`PanicOnDrop`], and expects the unwind of its drop to come out of it.
/// Any other panic is resumed, and returning without one fails the test.
pub(crate) fn expect_panic_on_drop<R>(callback: impl FnOnce(PanicOnDrop) -> R) {
    match catch_unwind(AssertUnwindSafe(|| callback(PanicOnDrop(())))) {
        Err(payload) if payload.is::<PanicOnDropPayload>() => {}
        Err(payload) => resume_unwind(payload),
        Ok(_) => panic!("expected the PanicOnDrop to be dropped"),
    }
}
