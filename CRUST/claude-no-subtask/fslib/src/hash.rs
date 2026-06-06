use std::collections::LinkedList;
use std::hash::Hash;
use std::marker::PhantomData;

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

const HASH_ALPHA: f32 = 0.75;

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
        let size = self.buckets.len();
        let idx = (self.hash_f)(&key) % size;
        // check existing
        for item in self.buckets[idx].items.iter_mut() {
            if item.key == key {
                item.value = value;
                return;
            }
        }
        self.buckets[idx]
            .items
            .push_back(HashItem { key, value });
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
        for item in self.buckets[idx].items.iter() {
            if &item.key == key {
                return Some(&item.value);
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
        // Find position
        let mut split_at = None;
        for (i, item) in self.buckets[idx].items.iter().enumerate() {
            if &item.key == key {
                split_at = Some(i);
                break;
            }
        }
        if let Some(pos) = split_at {
            let mut new_list = LinkedList::new();
            // move items
            let bucket = &mut self.buckets[idx];
            let mut tail = bucket.items.split_off(pos);
            tail.pop_front();
            new_list.append(&mut bucket.items);
            new_list.append(&mut tail);
            bucket.items = new_list;
            self.n_items -= 1;
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
        // move all items
        for bucket in self.buckets.drain(..) {
            for item in bucket.items {
                let idx = (self.hash_f)(&item.key) % new_size;
                new_buckets[idx].items.push_back(item);
            }
        }
        self.buckets = new_buckets;
    }
}
