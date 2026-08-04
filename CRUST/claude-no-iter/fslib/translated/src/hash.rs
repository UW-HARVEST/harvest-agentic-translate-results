use std::collections::LinkedList;
use std::hash::Hash;
use std::marker::PhantomData;

const HASH_ALPHA: f32 = 0.75;

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
        let bucket = &mut self.buckets[bnum];
        // check if we already have that key and update it
        for item in bucket.items.iter_mut() {
            if item.key == key {
                item.value = value;
                return;
            }
        }
        bucket.items.push_back(HashItem { key, value });
        self.n_items += 1;

        // resize if alpha exceeded
        if (self.n_items as f32) / (self.buckets.len() as f32) > HASH_ALPHA {
            self.resize();
        }
    }
    pub fn get(&self, key: &K) -> Option<&V> {
        let bnum = (self.hash_f)(key) % self.buckets.len();
        let bucket = &self.buckets[bnum];
        for item in bucket.items.iter() {
            if &item.key == key {
                return Some(&item.value);
            }
        }
        None
    }
    pub fn remove(&mut self, key: &K) {
        let bnum = (self.hash_f)(key) % self.buckets.len();
        let bucket = &mut self.buckets[bnum];
        let len_before = bucket.items.len();
        // Use a workaround since LinkedList doesn't have retain
        let mut new_list: LinkedList<HashItem<K, V>> = LinkedList::new();
        while let Some(item) = bucket.items.pop_front() {
            if &item.key != key {
                new_list.push_back(item);
            }
        }
        bucket.items = new_list;
        if bucket.items.len() < len_before {
            self.n_items -= 1;
        }
    }
    pub fn resize(&mut self) {
        let new_size = self.buckets.len() * 2;
        let mut new_buckets: Vec<Bucket<K, V>> = Vec::with_capacity(new_size);
        for _ in 0..new_size {
            new_buckets.push(Bucket { items: LinkedList::new() });
        }
        // move items
        let old_buckets = std::mem::replace(&mut self.buckets, new_buckets);
        for mut bucket in old_buckets {
            while let Some(item) = bucket.items.pop_front() {
                let bnum = (self.hash_f)(&item.key) % self.buckets.len();
                self.buckets[bnum].items.push_back(item);
            }
        }
    }
}
