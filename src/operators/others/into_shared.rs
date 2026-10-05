//! The [`IntoShared`] operator, behind
//! [`ObservableExt::into_shared`](crate::observable::ObservableExt::into_shared).

use crate::{
    observable::{Observable, ObservableTypes, Subscription},
    observer::Observer,
    thread_mode::Shared,
};
use educe::Educe;

/// Declares an Observable [`Shared`], whatever its own mode.
///
/// A synchronous source such as `Just` or `Throw` is `Local`, and erasing an observable keeps its
/// mode, so it cannot share an erased type with a `Shared` source, such as a subject, until it is
/// declared `Shared` too. A chain of `Local` sources also keeps its state in an `Rc`, which this
/// moves to an `Arc`, so that the chain can be erased into a `Send` box.
///
/// Declaring `Shared` is always sound: it only makes the operators downstream pick the thread-safe
/// pointers. Whatever the source itself holds is still checked for `Send` where it crosses a
/// thread. The other way round is not offered: a `Shared` source needs its observer to be `Send`,
/// which a `Local` chain downstream is not.
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::{ObservableExt, boxed_observable::SendBoxedObservable},
///     operators::creating::just::Just,
///     subject::publish_subject::PublishSubject,
///     thread_mode::Shared,
/// };
/// use std::convert::Infallible;
///
/// let subject = PublishSubject::<i32, Infallible, _>::shared();
/// let sources: Vec<SendBoxedObservable<'_, '_, '_, i32, Infallible, Shared>> = vec![
///     Just::new(1).into_shared().into_send_boxed(),
///     subject.into_send_boxed(),
/// ];
/// assert_eq!(sources.len(), 2);
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct IntoShared<OE> {
    source: OE,
}

impl<OE> IntoShared<OE> {
    /// Creates an [`IntoShared`] over `source`;
    /// [`ObservableExt::into_shared`](crate::observable::ObservableExt::into_shared) is the fluent
    /// form.
    pub fn new(source: OE) -> Self {
        Self { source }
    }
}

impl<OE> ObservableTypes for IntoShared<OE>
where
    OE: ObservableTypes,
{
    type Item = OE::Item;
    type Error = OE::Error;
    type Mode = Shared;
    type D = OE::D;
}

impl<OE, OR> Observable<OR> for IntoShared<OE>
where
    OR: Observer<OE::Item, OE::Error>,
    OE: Observable<OR>,
{
    fn subscribe(self, observer: OR) -> Subscription<Self::D> {
        self.source.subscribe(observer)
    }
}
