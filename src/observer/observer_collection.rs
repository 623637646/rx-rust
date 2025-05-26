use super::Observer;
use std::collections::BTreeMap;

pub struct Key(usize);

pub struct ObserverCollection<OR> {
    map: BTreeMap<usize, OR>,
    count: usize,
}

impl<OR> ObserverCollection<OR> {
    pub fn new() -> Self {
        Self {
            map: BTreeMap::new(),
            count: 0,
        }
    }

    pub fn insert(&mut self, observer: OR) -> Key {
        let key = self.count;
        self.count += 1;
        self.map.insert(key, observer);
        Key(key)
    }

    pub fn remove(&mut self, key: Key) -> OR {
        self.map.remove(&key.0).unwrap() // use unwrap() here, because key can't be cloned. So it's safe. 
    }
}

impl<OR> Default for ObserverCollection<OR> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T, E, OR> Observer<T, E> for ObserverCollection<OR>
where
    T: Clone,
    E: Clone,
    OR: Observer<T, E>,
{
    fn on_next(&mut self, value: T) {
        self.map.values_mut().for_each(|v| v.on_next(value.clone()));
    }

    fn on_termination(self, termination: super::Termination<E>) {
        self.map
            .into_values()
            .for_each(|v| v.on_termination(termination.clone()));
    }
}
