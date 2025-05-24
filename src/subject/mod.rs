pub mod behavior_subject;
pub mod publish_subject;

use crate::{observable::Observable, observer::Observer};

pub trait Subject<'or, 'sub, T, E, OE>: Observable<'or, 'sub, T, E> + Observer<T, E> {
    fn into_observable(self) -> OE;
}
