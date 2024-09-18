pub mod base_subject;
pub mod behavior_subject;
pub mod publish_subject;

use crate::{observable::Observable, observer::Observer};

pub trait Subject<T, E>: Observable<T, E> + Observer<T, E> {}
