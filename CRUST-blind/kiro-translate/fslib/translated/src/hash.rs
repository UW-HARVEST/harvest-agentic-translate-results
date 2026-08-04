use std::collections::LinkedList;
use std::hash::{Hash, Hasher};
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
const HASH_ALPHA: f64 = 0.75;
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
        let bnum = (self.hash_f)(&key) % self.buckets.len();
        for item in self.buckets[bnum].items.iter_mut() {
            if item.key == key {
                item.key = key.clone();
                item.value = value;
                return;
            }
        }
        self.buckets[bnum].items.push_back(HashItem { key, value });
        self.n_items += 1;
        if (self.n_items as f64 / self.buckets.len() as f64) > HASH_ALPHA {
            self.resize();
        }
    }
    pub fn get(&self, key: &K) -> Option<&V> {
        let bnum = (self.hash_f)(key) % self.buckets.len();
        for item in self.buckets[bnum].items.iter() {
            if item.key == *key {
                return Some(&item.value);
            }
        }
        None
    }
    pub fn remove(&mut self, key: &K) {
        let bnum = (self.hash_f)(key) % self.buckets.len();
        let bucket = &mut self.buckets[bnum];
        let mut new_list = LinkedList::new();
        let mut found = false;
        for item in bucket.items.iter() {
            if !found && item.key == *key {
                found = true;
                continue;
            }
            new_list.push_back(HashItem { key: item.key.clone(), value: item.value.clone() });
        }
        bucket.items = new_list;
        if found {
            self.n_items -= 1;
        }
    }
    pub fn resize(&mut self) {
        let old_size = self.buckets.len();
        let new_size = old_size * 2;
        let mut new_buckets = Vec::with_capacity(new_size);
        for _ in 0..new_size {
            new_buckets.push(Bucket { items: LinkedList::new() });
        }
        let old_buckets = std::mem::replace(&mut self.buckets, new_buckets);
        self.n_items = 0;
        for bucket in old_buckets {
            for item in bucket.items {
                self.insert(item.key, item.value);
            }
        }
    }
}
