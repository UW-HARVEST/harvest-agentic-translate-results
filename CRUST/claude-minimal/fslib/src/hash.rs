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
    F: Fn(&K) -> usize + Clone,
{
    pub fn new(hash_f: F, size: usize) -> Self {
        let mut buckets = Vec::with_capacity(size);
        for _ in 0..size {
            buckets.push(Bucket { items: LinkedList::new() });
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
        let bnum = (self.hash_f)(&key) % size;
        // Try to update existing
        for item in self.buckets[bnum].items.iter_mut() {
            if item.key == key {
                item.value = value;
                return;
            }
        }
        self.buckets[bnum].items.push_back(HashItem { key, value });
        self.n_items += 1;

        let load = self.n_items as f32 / size as f32;
        if load > HASH_ALPHA {
            self.resize();
        }
    }
    pub fn get(&self, key: &K) -> Option<&V> {
        let size = self.buckets.len();
        let bnum = (self.hash_f)(key) % size;
        for item in self.buckets[bnum].items.iter() {
            if &item.key == key {
                return Some(&item.value);
            }
        }
        None
    }
    pub fn remove(&mut self, key: &K) {
        let size = self.buckets.len();
        let bnum = (self.hash_f)(key) % size;
        let bucket = &mut self.buckets[bnum];
        let mut found_idx: Option<usize> = None;
        for (i, item) in bucket.items.iter().enumerate() {
            if &item.key == key {
                found_idx = Some(i);
                break;
            }
        }
        if let Some(idx) = found_idx {
            // Split the list at idx, then drop one element
            let mut split = bucket.items.split_off(idx);
            split.pop_front();
            bucket.items.append(&mut split);
            self.n_items -= 1;
        }
    }
    pub fn resize(&mut self) {
        let new_size = self.buckets.len() * 2;
        let mut new_buckets: Vec<Bucket<K, V>> = Vec::with_capacity(new_size);
        for _ in 0..new_size {
            new_buckets.push(Bucket { items: LinkedList::new() });
        }
        let old_buckets = std::mem::replace(&mut self.buckets, new_buckets);
        self.n_items = 0;
        for bucket in old_buckets {
            for item in bucket.items {
                let bnum = (self.hash_f)(&item.key) % new_size;
                self.buckets[bnum].items.push_back(item);
                self.n_items += 1;
            }
        }
    }
}
