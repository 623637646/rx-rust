use std::collections::HashMap;

/// A store that assigns a unique key to each inserted value.
pub struct UniqueKeyStore<T> {
    data: HashMap<usize, T>,
    counter: usize, // Auto-incrementing key generator
}

impl<T> Default for UniqueKeyStore<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> UniqueKeyStore<T> {
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
            counter: 0,
        }
    }

    pub fn insert(&mut self, value: T) -> usize {
        let key = self.counter;
        self.data.insert(key, value);
        self.counter += 1;
        key
    }

    pub fn remove(&mut self, key: usize) -> Option<T> {
        self.data.remove(&key)
    }

    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.data.values()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut T> {
        self.data.values_mut()
    }

    pub fn drain(&mut self) -> impl Iterator<Item = T> {
        self.data.drain().map(|(_, v)| v)
    }
}

impl<T> IntoIterator for UniqueKeyStore<T> {
    type Item = T;
    type IntoIter = std::collections::hash_map::IntoValues<usize, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.data.into_values()
    }
}

#[cfg(test)]
mod tests {
    use super::UniqueKeyStore;
    use std::ptr;

    #[test]
    fn test_insert() {
        let mut store = UniqueKeyStore::default();
        let key1 = store.insert("value1");
        let key2 = store.insert("value2");
        assert_ne!(key1, key2);
    }

    #[test]
    fn test_remove() {
        let mut store = UniqueKeyStore::default();
        let key = store.insert("value");
        assert_eq!(store.remove(key), Some("value"));
        assert_eq!(store.remove(key), None); // Ensure it's really removed
    }

    #[test]
    fn test_iter() {
        let mut store = UniqueKeyStore::default();
        store.insert("a");
        store.insert("b");
        let values: Vec<_> = store.iter().cloned().collect();
        assert!(values.contains(&"a"));
        assert!(values.contains(&"b"));
    }

    #[test]
    fn test_iter_mut() {
        let mut store = UniqueKeyStore::default();
        let key = store.insert(String::from("old"));
        for value in store.iter_mut() {
            *value = String::from("new");
        }
        assert_eq!(store.remove(key), Some(String::from("new")));
    }

    #[test]
    fn test_drain() {
        let mut store = UniqueKeyStore::default();
        store.insert("x");
        store.insert("y");
        let drained: Vec<_> = store.drain().collect();
        assert_eq!(drained.len(), 2);
        assert_eq!(store.iter().count(), 0); // Ensure it's empty
    }

    #[test]
    fn test_into_iterator() {
        let mut store = UniqueKeyStore::default();
        store.insert("first");
        store.insert("second");
        let values: Vec<_> = store.into_iter().collect();
        assert_eq!(values.len(), 2);
        assert!(values.contains(&"first"));
        assert!(values.contains(&"second"));
    }

    #[test]
    fn test_ref() {
        let value = 1;
        let mut store = UniqueKeyStore::default();
        let key = store.insert(&value);
        {
            let mut iter = store.iter();
            assert!(ptr::eq(*iter.next().unwrap(), &value));
            assert_eq!(iter.next(), None);
        }
        assert!(ptr::eq(store.remove(key).unwrap(), &value));
    }
}
