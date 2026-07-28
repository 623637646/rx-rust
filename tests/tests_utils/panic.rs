use std::{
    any::{Any, TypeId, type_name},
    cell::RefCell,
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
    static SUPPRESSED_PANIC_TYPES: RefCell<Vec<TypeId>> = const { RefCell::new(Vec::new()) };
}

static INSTALL_HOOK: Once = Once::new();

fn install_hook() {
    INSTALL_HOOK.call_once(|| {
        let previous_hook = panic::take_hook();
        panic::set_hook(Box::new(move |info| {
            let payload_type = info.payload().type_id();
            let suppress = SUPPRESSED_PANIC_TYPES
                .try_with(|types| types.borrow().contains(&payload_type))
                .unwrap_or(false);
            if !suppress {
                previous_hook(info);
            }
        }));
    });
}

struct SuppressionGuard(TypeId);

impl SuppressionGuard {
    fn new<P: Any>() -> Self {
        let panic_type = TypeId::of::<P>();
        SUPPRESSED_PANIC_TYPES.with(|types| types.borrow_mut().push(panic_type));
        Self(panic_type)
    }
}

impl Drop for SuppressionGuard {
    fn drop(&mut self) {
        SUPPRESSED_PANIC_TYPES.with(|types| {
            let panic_type = types.borrow_mut().pop();
            debug_assert_eq!(panic_type, Some(self.0));
        });
    }
}

pub(crate) fn expect_panic<P, R>(callback: impl FnOnce() -> R)
where
    P: Any + Send,
{
    install_hook();
    let result = panic::catch_unwind(AssertUnwindSafe(|| {
        let _guard = SuppressionGuard::new::<P>();
        callback()
    }));
    match result {
        Err(payload) if payload.is::<P>() => {}
        Err(payload) => panic::resume_unwind(payload),
        Ok(_) => panic!("expected panic payload `{}`", type_name::<P>()),
    }
}

pub(crate) fn expect_panic_on_drop<R>(callback: impl FnOnce(PanicOnDrop) -> R) {
    expect_panic::<PanicOnDropPayload, _>(|| callback(PanicOnDrop(())));
}
