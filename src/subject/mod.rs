pub mod behavior_subject;
pub mod publish_subject;

use crate::{observable::Observable, observer::Observer};

pub trait Subject<'a, T, E, OR>: Observable<'a, T, E, OR> + Observer<T, E>
where
    OR: Observer<T, E>,
{
}
