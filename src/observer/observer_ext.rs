// use super::{event::Event, Observer};
// use std::sync::Arc;

// impl<T, E> Observer<T, E> for Box<dyn Observer<T, E>>
// where
//     T: 'static,
//     E: 'static,
// {
//     fn on_next(&mut self, value: T) {
//         self.as_mut().on_next(value);
//     }

//     fn on_terminal(self, terminal: super::Terminal<E>) {
//         let observer = *self;
//         observer.on_terminal(terminal);
//     }
// }

// impl<T, E, O> Observer<T, E> for Arc<O>
// where
//     O: Observer<T, E>,
// {
//     fn on(&self, event: Event<T, E>) {
//         self.as_ref().on(event);
//     }

//     fn terminated(&self) -> bool {
//         self.as_ref().terminated()
//     }

//     fn set_terminated(&self, terminated: bool) {
//         self.as_ref().set_terminated(terminated);
//     }
// }
