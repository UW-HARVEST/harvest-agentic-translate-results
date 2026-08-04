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
        let size = self.buckets.len();
        if size == 0 {
            return;
        }
        let bnum = (self.hash_f)(&key) % size;
        // Check if key exists, update
        for item in self.buckets[bnum].items.iter_mut() {
            if item.key == key {
                item.value = value;
                return;
            }
        }
        self.buckets[bnum].items.push_back(HashItem { key, value });
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
        if size == 0 {
            return;
        }
        let bnum = (self.hash_f)(key) % size;
        let bucket = &mut self.buckets[bnum];
        let mut new_list: LinkedList<HashItem<K, V>> = LinkedList::new();
        let mut removed = false;
        while let Some(item) = bucket.items.pop_front() {
            if !removed && &item.key == key {
                removed = true;
                continue;
            }
            new_list.push_back(item);
        }
        bucket.items = new_list;
        if removed {
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
        for mut bucket in old_buckets {
            while let Some(item) = bucket.items.pop_front() {
                let bnum = (self.hash_f)(&item.key) % new_size;
                self.buckets[bnum].items.push_back(item);
            }
        }
    }
}
