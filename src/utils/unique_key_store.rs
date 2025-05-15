pub(crate) struct ID(usize);

pub(crate) struct UniqueKeyStore<T>(Vec<Option<T>>);

impl<T> Default for UniqueKeyStore<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> UniqueKeyStore<T> {
    pub(crate) fn new() -> Self {
        Self(Vec::new())
    }

    pub(crate) fn insert(&mut self, value: T) -> ID {
        // Reuse empty slot if available
        for (i, slot) in self.0.iter_mut().enumerate() {
            if slot.is_none() {
                *slot = Some(value);
                return ID(i);
            }
        }

        self.0.push(Some(value));
        ID(self.0.len() - 1)
    }

    pub(crate) fn remove(&mut self, key: ID) -> Option<T> {
        self.0.get_mut(key.0).and_then(Option::take)
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = &T> {
        self.0.iter().filter_map(Option::as_ref)
    }

    pub(crate) fn iter_mut(&mut self) -> impl Iterator<Item = &mut T> {
        self.0.iter_mut().filter_map(Option::as_mut)
    }

    pub(crate) fn drain(&mut self) -> impl Iterator<Item = T> {
        self.0.drain(..).flatten()
    }
}

impl<T> IntoIterator for UniqueKeyStore<T> {
    type Item = T;
    type IntoIter = std::vec::IntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter().flatten().collect::<Vec<_>>().into_iter()
    }
}

#[cfg(test)]
mod tests {
    use super::UniqueKeyStore;
    use crate::utils::unique_key_store::ID;

    #[test]
    fn test_insert_and_iter() {
        let mut vec = UniqueKeyStore::new();
        let id1 = vec.insert(10);
        let id2 = vec.insert(20);
        let id3 = vec.insert(30);

        let collected: Vec<_> = vec.iter().cloned().collect();
        assert_eq!(collected, vec![10, 20, 30]);

        assert_eq!(id1.0, 0);
        assert_eq!(id2.0, 1);
        assert_eq!(id3.0, 2);
    }

    #[test]
    fn test_remove() {
        let mut vec = UniqueKeyStore::new();
        let id1 = vec.insert("a");
        vec.insert("b");

        assert_eq!(vec.remove(ID(id1.0)), Some("a"));
        assert_eq!(vec.remove(id1), None); // Already removed
        assert_eq!(vec.remove(ID(100)), None); // Out of bounds

        let remaining: Vec<_> = vec.iter().cloned().collect();
        assert_eq!(remaining, vec!["b"]);
    }

    #[test]
    fn test_insert_reuse_slot() {
        let mut vec = UniqueKeyStore::new();
        let id1 = vec.insert(1);
        vec.insert(2);
        vec.remove(ID(id1.0));
        let id3 = vec.insert(3);

        assert_eq!(id3.0, id1.0); // reused slot
        let values: Vec<_> = vec.iter().cloned().collect();
        assert_eq!(values, vec![3, 2]);
    }

    #[test]
    fn test_iter_mut() {
        let mut vec = UniqueKeyStore::new();
        vec.insert(1);
        vec.insert(2);
        vec.insert(3);

        for val in vec.iter_mut() {
            *val *= 2;
        }

        let values: Vec<_> = vec.iter().cloned().collect();
        assert_eq!(values, vec![2, 4, 6]);
    }

    #[test]
    fn test_drain() {
        let mut vec = UniqueKeyStore::new();
        vec.insert("x");
        vec.insert("y");
        vec.insert("z");

        let drained: Vec<_> = vec.drain().collect();
        assert_eq!(drained, vec!["x", "y", "z"]);
        assert_eq!(vec.iter().count(), 0);
    }

    #[test]
    fn test_into_iter() {
        let mut vec = UniqueKeyStore::new();
        vec.insert("hello");
        vec.insert("world");

        let collected: Vec<_> = vec.into_iter().collect();
        assert_eq!(collected, vec!["hello", "world"]);
    }

    #[test]
    fn test_default() {
        let vec: UniqueKeyStore<i32> = Default::default();
        assert_eq!(vec.iter().count(), 0);
    }

    #[test]
    fn test_remove_and_iter_mut_combo() {
        let mut vec = UniqueKeyStore::new();
        let _id = vec.insert(1);
        let id = vec.insert(2);
        let _id = vec.insert(3);
        vec.remove(id);

        let mut iter_mut = vec.iter_mut();
        assert_eq!(iter_mut.next(), Some(&mut 1));
        assert_eq!(iter_mut.next(), Some(&mut 3));
        assert_eq!(iter_mut.next(), None);
    }

    #[test]
    fn test_drain_empty() {
        let mut vec: UniqueKeyStore<u8> = UniqueKeyStore::new();
        let drained: Vec<_> = vec.drain().collect();
        assert!(drained.is_empty());
    }

    #[test]
    fn test_ref() {
        let value = 1;
        let mut store = UniqueKeyStore::default();
        let key = store.insert(&value);
        {
            let mut iter = store.iter();
            assert!(std::ptr::eq(*iter.next().unwrap(), &value));
            assert_eq!(iter.next(), None);
        }
        assert!(std::ptr::eq(store.remove(key).unwrap(), &value));
    }

    #[test]
    fn test_order() {
        let mut store = UniqueKeyStore::default();
        for i in 0..100 {
            store.insert(format!("value{}", i));
        }
        for (i, value) in store.into_iter().enumerate() {
            assert_eq!(value, format!("value{}", i));
        }
    }
}
