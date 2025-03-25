pub mod publish_subject;

use crate::{observable::Observable, observer::Observer};

pub trait Subject<T, E, OR>: Observable<T, E, OR> + Observer<T, E>
where
    OR: Observer<T, E>,
{
}
