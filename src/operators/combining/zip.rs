use crate::utils::subscribe_with_shared_model::{
    self, Context, ModificationResult, subscribe_with_shared_model,
};
use crate::utils::types::MaybeSend;
use crate::{delegate_disposal, utils::subscribe_unsub_after_termination};
use crate::{
    observable::{Observable, Subscription},
    observer::{Observer, Termination},
    utils::subscribe_unsub_after_termination::subscribe_unsub_after_termination,
};
use educe::Educe;
use std::collections::VecDeque;

/// Combines the emissions of multiple Observables together via a specified function and emits single items for each combination based on the sequence of their emissions.
/// See <https://reactivex.io/documentation/operators/zip.html>
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt,
///     observer::Termination,
///     operators::{
///         combining::zip::Zip,
///         creating::from_iter::FromIter,
///     },
/// };
///
/// let mut values = Vec::new();
/// let mut terminations = Vec::new();
///
/// let observable = Zip::new(
///     FromIter::new(vec![1, 2]),
///     FromIter::new(vec![10, 20]),
/// );
/// observable.subscribe_with_callback(
///     |value| values.push(value),
///     |termination| terminations.push(termination),
/// );
///
/// assert_eq!(values, vec![(1, 10), (2, 20)]);
/// assert_eq!(terminations, vec![Termination::Completed]);
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct Zip<OE1, OE2> {
    source_1: OE1,
    source_2: OE2,
}

impl<OE1, OE2> Zip<OE1, OE2> {
    pub fn new<'or, T1, T2, E>(source_1: OE1, source_2: OE2) -> Self
    where
        OE1: Observable<'or, T = T1, E = E>,
        OE2: Observable<'or, T = T2, E = E>,
    {
        Self { source_1, source_2 }
    }
}

delegate_disposal!(
    Disposal<'or>,
    subscribe_unsub_after_termination::Disposal<subscribe_with_shared_model::Disposal<'or>>
);

impl<'or, T1, T2, E, OE1, OE2> Observable<'or> for Zip<OE1, OE2>
where
    T1: MaybeSend + 'or,
    T2: MaybeSend + 'or,
    E: MaybeSend + 'or,
    OE1: Observable<'or, T = T1, E = E>,
    OE1::D: MaybeSend + 'or,
    OE2: Observable<'or, T = T2, E = E>,
    OE2::D: MaybeSend + 'or,
{
    type T = (T1, T2);
    type E = E;
    type D = Disposal<'or>;

    fn subscribe(
        self,
        observer: impl Observer<(T1, T2), E> + MaybeSend + 'or,
    ) -> Subscription<Self::D> {
        subscribe_unsub_after_termination(observer, |observer| {
            let model = Model {
                first: (VecDeque::new(), false),
                second: (VecDeque::new(), false),
            };
            subscribe_with_shared_model(observer, model, |context| {
                let subscription_1 = self.source_1.subscribe(ZipObserver1(context.clone()));
                let subscription_2 = self.source_2.subscribe(ZipObserver2(context));
                subscription_1.preceded_by_bound(subscription_2)
            })
        })
        .map_into()
    }
}

struct Model<T1, T2> {
    first: (VecDeque<T1>, bool),  // bool means completed
    second: (VecDeque<T2>, bool), // bool means completed
}

macro_rules! impl_zip_observer {
    ($name:ident, $input_t:ty, $this_field:ident, $other_field:ident, $make_pair:expr) => {
        struct $name<T1, T2, E, OR>(Context<(T1, T2), E, OR, Model<T1, T2>>);

        impl<T1, T2, E, OR> Observer<$input_t, E> for $name<T1, T2, E, OR>
        where
            OR: Observer<(T1, T2), E>,
        {
            fn on_next(&mut self, value: $input_t) {
                let _ = self.0.modify_model(|model| {
                    if let Some(other) = model.$other_field.0.pop_front() {
                        if model.$other_field.1 && model.$other_field.0.is_empty() {
                            ModificationResult::new_send_next_and_termination(
                                $make_pair(value, other),
                                Termination::Completed,
                            )
                        } else {
                            ModificationResult::new_send_next($make_pair(value, other))
                        }
                    } else {
                        model.$this_field.0.push_back(value);
                        ModificationResult::new_without_result()
                    }
                });
            }

            fn on_termination(self, termination: Termination<E>) {
                match termination {
                    Termination::Completed => {
                        let _ = self.0.modify_model(|model| {
                            model.$this_field.1 = true;
                            if model.$this_field.0.is_empty() {
                                ModificationResult::new_send_termination(termination)
                            } else {
                                ModificationResult::new_without_result()
                            }
                        });
                    }
                    Termination::Error(_) => {
                        self.0.send_termination(termination);
                    }
                };
            }
        }
    };
}

impl_zip_observer!(ZipObserver1, T1, first, second, |this, other| (this, other));
impl_zip_observer!(ZipObserver2, T2, second, first, |this, other| (other, this));
