use fslib::hash::HashTable;

fn simple_hash(k: &i32) -> usize {
    *k as usize
}

#[test]
fn test_hash_new() {
    let h: HashTable<i32, i32, _> = HashTable::new(simple_hash, 16);
    assert_eq!(h.n_items, 0);
    assert_eq!(h.buckets.len(), 16);
}

#[test]
fn test_hash_insert_get() {
    let mut h: HashTable<i32, i32, _> = HashTable::new(simple_hash, 16);
    h.insert(1, 100);
    h.insert(2, 200);
    h.insert(3, 300);
    assert_eq!(h.n_items, 3);
    assert_eq!(h.get(&1), Some(&100));
    assert_eq!(h.get(&2), Some(&200));
    assert_eq!(h.get(&3), Some(&300));
    assert_eq!(h.get(&999), None);
}

#[test]
fn test_hash_insert_overwrite() {
    let mut h: HashTable<i32, i32, _> = HashTable::new(simple_hash, 16);
    h.insert(1, 100);
    h.insert(1, 200);
    assert_eq!(h.n_items, 1);
    assert_eq!(h.get(&1), Some(&200));
}

#[test]
fn test_hash_remove() {
    let mut h: HashTable<i32, i32, _> = HashTable::new(simple_hash, 16);
    h.insert(1, 100);
    h.insert(2, 200);
    h.remove(&1);
    assert_eq!(h.n_items, 1);
    assert_eq!(h.get(&1), None);
    assert_eq!(h.get(&2), Some(&200));
    // remove non-existent does not change n_items
    h.remove(&999);
    assert_eq!(h.n_items, 1);
}

#[test]
fn test_hash_resize_threshold() {
    // Resize triggers when load factor > 0.75
    let mut h: HashTable<i32, i32, _> = HashTable::new(simple_hash, 4);
    h.insert(1, 1);
    h.insert(2, 2);
    h.insert(3, 3); // load factor = 3/4 = 0.75, no resize
    assert_eq!(h.buckets.len(), 4);
    h.insert(4, 4); // load factor > 0.75 triggers resize
    assert_eq!(h.buckets.len(), 8);
    // values still accessible
    assert_eq!(h.get(&1), Some(&1));
    assert_eq!(h.get(&2), Some(&2));
    assert_eq!(h.get(&3), Some(&3));
    assert_eq!(h.get(&4), Some(&4));
}

#[test]
fn test_hash_collisions() {
    // Hash maps everything to bucket 0
    let mut h: HashTable<i32, i32, _> = HashTable::new(|_| 0usize, 8);
    h.insert(1, 100);
    h.insert(2, 200);
    h.insert(3, 300);
    assert_eq!(h.get(&1), Some(&100));
    assert_eq!(h.get(&2), Some(&200));
    assert_eq!(h.get(&3), Some(&300));
    h.remove(&2);
    assert_eq!(h.get(&1), Some(&100));
    assert_eq!(h.get(&2), None);
    assert_eq!(h.get(&3), Some(&300));
}

#[test]
fn test_hash_empty_buckets_get() {
    let h: HashTable<i32, i32, _> = HashTable::new(simple_hash, 8);
    assert_eq!(h.get(&1), None);
}

fn main() {}
