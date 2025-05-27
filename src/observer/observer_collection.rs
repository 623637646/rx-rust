use super::Observer;
use std::collections::BTreeMap;

pub struct Key(usize);

pub struct ObserverCollection<OR> {
    map: BTreeMap<usize, Option<OR>>,
    count: usize,
    borrowed: bool,
}

impl<OR> ObserverCollection<OR> {
    pub fn new() -> Self {
        Self {
            map: BTreeMap::new(),
            count: 0,
            borrowed: false,
        }
    }

    pub fn insert(&mut self, observer: OR) -> Key {
        let key = self.count;
        self.count += 1;
        self.map.insert(key, Some(observer));
        Key(key)
    }

    pub fn remove(&mut self, key: Key) -> Option<OR> {
        self.map.remove(&key.0).unwrap() // It's safe to use unwrap() here because key can't be cloned.
    }

    pub fn borrow_agent(&mut self) -> Option<ObserverCollectionAgent<OR>> {
        if self.borrowed {
            None
        } else {
            self.borrowed = true;
            let map = self
                .map
                .iter_mut()
                .map(|(key, value)| (*key, value.take().unwrap()))
                .collect();
            Some(ObserverCollectionAgent(map))
        }
    }

    pub fn return_agent(&mut self, mut agent: ObserverCollectionAgent<OR>) {
        assert!(self.borrowed);
        self.map
            .iter_mut()
            .for_each(|(key, value)| *value = agent.0.remove(key));
        self.borrowed = false;
    }
}

impl<OR> Default for ObserverCollection<OR> {
    fn default() -> Self {
        Self::new()
    }
}

pub struct ObserverCollectionAgent<OR>(BTreeMap<usize, OR>);

impl<T, E, OR> Observer<T, E> for ObserverCollectionAgent<OR>
where
    T: Clone,
    E: Clone,
    OR: Observer<T, E>,
{
    fn on_next(&mut self, value: T) {
        self.0.values_mut().for_each(|v| v.on_next(value.clone()));
    }

    fn on_termination(self, termination: super::Termination<E>) {
        self.0
            .into_values()
            .for_each(|v| v.on_termination(termination.clone()));
    }
}
