use std::collections::LinkedList;
use std::hash::Hash;
use std::marker::PhantomData;

const HASH_ALPHA: f32 = 0.75;

#[derive(Default)]
pub struct Bucket<K, V> {
    pub items: LinkedList<HashItem<K, V>>,
}

pub struct HashItem<K, V> {
    pub key: K,
    pub value: V,
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
    K: Eq + Hash + Clone,
    V: Clone,
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
        let size = self.buckets.len();
        let idx = (self.hash_f)(&key) % size;
        let bucket = &mut self.buckets[idx];
        // Update if already present
        for it in bucket.items.iter_mut() {
            if it.key == key {
                it.value = value;
                return;
            }
        }
        bucket.items.push_back(HashItem { key, value });
        self.n_items += 1;
        if (self.n_items as f32) / (size as f32) > HASH_ALPHA {
            self.resize();
        }
    }
    pub fn get(&self, key: &K) -> Option<&V> {
        let size = self.buckets.len();
        if size == 0 {
            return None;
        }
        let idx = (self.hash_f)(key) % size;
        let bucket = &self.buckets[idx];
        for it in bucket.items.iter() {
            if &it.key == key {
                return Some(&it.value);
            }
        }
        None
    }
    pub fn remove(&mut self, key: &K) {
        let size = self.buckets.len();
        if size == 0 {
            return;
        }
        let idx = (self.hash_f)(key) % size;
        let bucket = &mut self.buckets[idx];
        let mut found_idx: Option<usize> = None;
        for (i, it) in bucket.items.iter().enumerate() {
            if &it.key == key {
                found_idx = Some(i);
                break;
            }
        }
        if let Some(i) = found_idx {
            // LinkedList lacks a "remove at index" API, so rebuild.
            let mut new_items = LinkedList::new();
            for (j, it) in bucket.items.iter().enumerate() {
                if j != i {
                    new_items.push_back(HashItem {
                        key: it.key.clone(),
                        value: it.value.clone(),
                    });
                }
            }
            bucket.items = new_items;
            if self.n_items > 0 {
                self.n_items -= 1;
            }
        }
    }
    pub fn resize(&mut self) {
        let new_size = self.buckets.len() * 2;
        let mut new_buckets: Vec<Bucket<K, V>> = Vec::with_capacity(new_size);
        for _ in 0..new_size {
            new_buckets.push(Bucket {
                items: LinkedList::new(),
            });
        }
        let old_buckets = std::mem::replace(&mut self.buckets, new_buckets);
        self.n_items = 0;
        for bucket in old_buckets.into_iter() {
            for item in bucket.items.into_iter() {
                self.insert(item.key, item.value);
            }
        }
    }
}
