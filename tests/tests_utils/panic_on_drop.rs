use std::{
    cell::Cell,
    panic::{self, AssertUnwindSafe},
    sync::Once,
};

struct PanicOnDropPayload;

pub(crate) struct PanicOnDrop(());

impl Drop for PanicOnDrop {
    fn drop(&mut self) {
        panic::panic_any(PanicOnDropPayload);
    }
}

thread_local! {
    static SUPPRESSION_DEPTH: Cell<usize> = const { Cell::new(0) };
}

static INSTALL_HOOK: Once = Once::new();

fn install_hook() {
    INSTALL_HOOK.call_once(|| {
        let previous_hook = panic::take_hook();
        panic::set_hook(Box::new(move |info| {
            let suppress = info.payload().is::<PanicOnDropPayload>()
                && SUPPRESSION_DEPTH
                    .try_with(|depth| depth.get() > 0)
                    .unwrap_or(false);
            if !suppress {
                previous_hook(info);
            }
        }));
    });
}

struct SuppressionGuard;

impl SuppressionGuard {
    fn new() -> Self {
        SUPPRESSION_DEPTH.with(|depth| depth.set(depth.get() + 1));
        Self
    }
}

impl Drop for SuppressionGuard {
    fn drop(&mut self) {
        SUPPRESSION_DEPTH.with(|depth| {
            let current = depth.get();
            debug_assert!(current > 0);
            depth.set(current - 1);
        });
    }
}

pub(crate) fn expect_panic_on_drop<R>(callback: impl FnOnce(PanicOnDrop) -> R) {
    install_hook();
    let result = panic::catch_unwind(AssertUnwindSafe(|| {
        let _guard = SuppressionGuard::new();
        callback(PanicOnDrop(()))
    }));
    match result {
        Err(payload) if payload.is::<PanicOnDropPayload>() => {}
        Err(payload) => panic::resume_unwind(payload),
        Ok(_) => panic!("PanicOnDrop was not dropped by the callback"),
    }
}
