use crate::{
    disposable::{Disposable, callback_disposal::CallbackDisposal},
    thread_mode::mutable::MutableHelper,
};
use std::cell::RefCell;

#[test]
fn test_tuple_disposal_disposes_the_last_first() {
    let order = RefCell::new(Vec::new());
    let callback = |index| {
        let order = &order;
        CallbackDisposal::new(move || order.with_mut(|order| order.push(index)))
    };
    let check = |expected: &[usize]| {
        order.with_mut(|order| {
            assert_eq!(order, expected);
            order.clear();
        });
    };

    (callback(0),).dispose();
    check(&[0]);
    (callback(0), callback(1)).dispose();
    check(&[1, 0]);
    (callback(0), callback(1), callback(2)).dispose();
    check(&[2, 1, 0]);
    (callback(0), callback(1), callback(2), callback(3)).dispose();
    check(&[3, 2, 1, 0]);
    (
        callback(0),
        callback(1),
        callback(2),
        callback(3),
        callback(4),
    )
        .dispose();
    check(&[4, 3, 2, 1, 0]);
    (
        callback(0),
        callback(1),
        callback(2),
        callback(3),
        callback(4),
        callback(5),
    )
        .dispose();
    check(&[5, 4, 3, 2, 1, 0]);
    (
        callback(0),
        callback(1),
        callback(2),
        callback(3),
        callback(4),
        callback(5),
        callback(6),
    )
        .dispose();
    check(&[6, 5, 4, 3, 2, 1, 0]);
    (
        callback(0),
        callback(1),
        callback(2),
        callback(3),
        callback(4),
        callback(5),
        callback(6),
        callback(7),
    )
        .dispose();
    check(&[7, 6, 5, 4, 3, 2, 1, 0]);
}

#[cfg(panic = "unwind")]
#[test]
fn test_tuple_disposal_disposes_remaining_on_panic() {
    use std::panic::{AssertUnwindSafe, catch_unwind, resume_unwind};

    struct DisposalPanic;

    let order = RefCell::new(Vec::new());
    for panic_index in 0..8 {
        let callback = |index| {
            let order = &order;
            CallbackDisposal::new(move || {
                order.with_mut(|order| order.push(index));
                if index == panic_index {
                    resume_unwind(Box::new(DisposalPanic));
                }
            })
        };
        let result = catch_unwind(AssertUnwindSafe(|| {
            (
                callback(0),
                callback(1),
                callback(2),
                callback(3),
                callback(4),
                callback(5),
                callback(6),
                callback(7),
            )
                .dispose();
        }));
        let payload = result.expect_err("the selected disposal must panic");
        assert!(payload.is::<DisposalPanic>());
        order.with_mut(|order| {
            assert_eq!(*order, [7, 6, 5, 4, 3, 2, 1, 0]);
            order.clear();
        });
    }
}
