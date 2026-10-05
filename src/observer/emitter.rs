//! [`Emitter`], the downstream observer handed unboxed to user code.

use super::{Flow, Observer, Termination};
use crate::thread_mode::ThreadMode;
use std::marker::PhantomData;

/// The downstream observer itself, unboxed, as user code gets it: the builder of
/// [`Create::local`](crate::operators::creating::create::Create::local) /
/// [`Create::shared`](crate::operators::creating::create::Create::shared) and the callbacks of
/// [`hook_on_subscription`](crate::observable::ObservableExt::hook_on_subscription) and
/// [`hook_on_termination`](crate::observable::ObservableExt::hook_on_termination), in the mode of
/// their source.
///
/// It is a concrete type rather than the bare `OR`, so the callback's parameter needs no
/// annotation: method lookup only needs the type constructor, and `OR` is inferred from the
/// subscription later. It implements [`Observer`], so the callback can also hand it to another
/// source: `source.subscribe(emitter)`. Its `PhantomData<M>` makes it `!Send + !Sync` for
/// [`Local`](crate::thread_mode::Local), and as `Send` as the observer for
/// [`Shared`](crate::thread_mode::Shared).
pub struct Emitter<OR, M: ThreadMode> {
    observer: OR,
    /// Enforces the `local` declaration; memory safety does not need it. Without it, a `Local`
    /// builder that moves the emitter to another thread compiles as long as nothing downstream is
    /// `!Send` (`.map(..)` into a callback), and fails once something is (`.merge_with(..)`, whose
    /// `Rc` state is `!Send`). That failure would point into the operator's internals instead of at
    /// the declaration, and whether the builder compiles would depend on how the operators
    /// downstream keep their state, which they may change. With it, the builder never compiles, and
    /// the error names `Emitter<_, Local>`. `Shared` is `Send + Sync`, so it adds nothing there.
    _mode: PhantomData<M>,
}

impl<OR, M: ThreadMode> Emitter<OR, M> {
    /// Wraps `observer` for user code that runs in mode `M`.
    ///
    /// There is no way back to the bare observer: under [`Local`](crate::thread_mode::Local) that
    /// would let the callback send a `Send` observer to another thread after all.
    pub fn new(observer: OR) -> Self {
        Self {
            observer,
            _mode: PhantomData,
        }
    }
}

impl<T, E, OR: Observer<T, E>, M: ThreadMode> Observer<T, E> for Emitter<OR, M> {
    fn on_next(&mut self, value: T) -> Flow {
        self.observer.on_next(value)
    }

    fn on_termination(self, termination: Termination<E>) {
        self.observer.on_termination(termination);
    }
}
