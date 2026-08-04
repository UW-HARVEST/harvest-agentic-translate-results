use std::collections::LinkedList;
use std::hash::Hash;
use std::marker::PhantomData;

#[derive(Default)]
pub struct Bucket<K, V> {
    items: LinkedList<HashItem<K, V>>,
}

struct HashItem<K, V> {
    key: K,
    value: V,
}

pub struct HashTable<K, V, F>
where
    K: Eq + Hash,
    F: Fn(&K) -> usize,
{
    pub buckets: Vec<Bucket<K, V>>,
    pub hash_f: F,
    pub n_items: usize,
    pub _marker: PhantomData<K>,
}

impl<K, V, F> HashTable<K, V, F>
where
    K: Eq + Hash,
    F: Fn(&K) -> usize,
{
    pub fn new(hash_f: F, size: usize) -> Self {
        let mut buckets = Vec::with_capacity(size);
        for _ in 0..size {
            buckets.push(Bucket {
                items: LinkedList::new(),
            });
        }
        Self {
            buckets,
            hash_f,
            n_items: 0,
            _marker: PhantomData,
        }
    }

    pub fn insert(&mut self, key: K, value: V) {
        let index = (self.hash_f)(&key) % self.buckets.len();
        for item in &mut self.buckets[index].items {
            if item.key == key {
                item.value = value;
                return;
            }
        }

        self.buckets[index].items.push_back(HashItem { key, value });
        self.n_items += 1;

        if (self.n_items as f32) / (self.buckets.len() as f32) > 0.75 {
            self.resize();
        }
    }

    pub fn get(&self, key: &K) -> Option<&V> {
        let index = (self.hash_f)(key) % self.buckets.len();
        self.buckets[index]
            .items
            .iter()
            .find(|item| &item.key == key)
            .map(|item| &item.value)
    }

    pub fn remove(&mut self, key: &K) {
        let index = (self.hash_f)(key) % self.buckets.len();
        let mut new_items = LinkedList::new();
        let mut removed = false;

        while let Some(item) = self.buckets[index].items.pop_front() {
            if !removed && &item.key == key {
                removed = true;
            } else {
                new_items.push_back(item);
            }
        }

        if removed {
            self.n_items -= 1;
        }

        self.buckets[index].items = new_items;
    }

    pub fn resize(&mut self) {
        let new_size = (self.buckets.len().max(1)) * 2;
        let mut new_buckets = Vec::with_capacity(new_size);
        for _ in 0..new_size {
            new_buckets.push(Bucket {
                items: LinkedList::new(),
            });
        }

        let mut old_buckets = std::mem::replace(&mut self.buckets, new_buckets);
        self.n_items = 0;

        for bucket in &mut old_buckets {
            while let Some(item) = bucket.items.pop_front() {
                self.insert(item.key, item.value);
            }
        }
    }
}
