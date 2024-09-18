use super::base_subject::BaseSubject;

pub type PublishSubject<T, E> = BaseSubject<T, E>;

// pub struct PublishSubject<T, E> {
//     base_subject: BaseSubject<T, E>,
// }

// impl<T, E> PublishSubject<T, E> {
//     pub fn new() -> Self {
//         PublishSubject {
//             base_subject: BaseSubject::new(),
//         }
//     }
// }

// impl<T, E> Default for PublishSubject<T, E> {
//     fn default() -> Self {
//         Self::new()
//     }
// }

// impl<T, E> Clone for PublishSubject<T, E> {
//     fn clone(&self) -> Self {
//         PublishSubject {
//             base_subject: self.base_subject.clone(),
//         }
//     }
// }

// impl<T, E> Observable<T, E> for PublishSubject<T, E>
// where
//     T: 'static,
//     E: 'static,
// {
//     fn subscribe(self, observer: impl Observer<T, E>) -> Subscription {
//         self.base_subject.subscribe(observer)
//     }
// }

// impl<T, E> Observer<T, E> for PublishSubject<T, E>
// where
//     T: Clone + 'static,
//     E: Clone + 'static,
// {
//     fn on(&self, event: Event<T, E>) {
//         self.base_subject.on(event);
//     }

//     fn terminated(&self) -> bool {
//         self.base_subject.terminated()
//     }

//     fn set_terminated(&self, terminated: bool) {
//         self.base_subject.set_terminated(terminated);
//     }
// }

// impl<T, E> Subject<T, E> for PublishSubject<T, E>
// where
//     T: Clone + 'static,
//     E: Clone + 'static,
// {
// }
