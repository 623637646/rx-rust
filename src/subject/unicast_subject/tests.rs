use super::{SenderState, new};
use crate::{
    observable::ObservableExt,
    observer::{Observer, Termination},
    thread_mode::{Local, Shared, ThreadMode},
};
use std::convert::Infallible;

#[test]
fn test_handoff_releases_shared_state() {
    fn check<M: ThreadMode>() {
        let (mut sender, observable) = new::<_, Infallible, M, _>();
        let SenderState::ViaPipe(state) = &sender.state else {
            unreachable!()
        };
        let weak = M::downgrade(state);

        assert!(sender.on_next(1).is_continue());
        let _subscription = observable.subscribe_with_callback(|_| {}, |_| {});
        // The observer is parked until the first event after subscribing.
        assert!(M::upgrade(&weak).is_some());

        assert!(sender.on_next(2).is_continue());
        assert!(M::upgrade(&weak).is_none());
        // Direct delivery and termination no longer need the shared allocation.
        assert!(sender.on_next(3).is_continue());
        sender.on_termination(Termination::Completed);
    }

    check::<Local>();
    check::<Shared>();
}
