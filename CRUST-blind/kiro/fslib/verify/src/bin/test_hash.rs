use fslib::hash::HashTable;

fn simple_hash(k: &u32) -> usize {
    *k as usize
}

#[test]
fn test_insert_and_get() {
    let mut ht: HashTable<u32, String, _> = HashTable::new(simple_hash, 16);
    ht.insert(1, "one".to_string());
    ht.insert(2, "two".to_string());
    assert_eq!(ht.get(&1), Some(&"one".to_string()));
    assert_eq!(ht.get(&2), Some(&"two".to_string()));
    assert_eq!(ht.get(&3), None);
}

#[test]
fn test_update_existing() {
    let mut ht: HashTable<u32, String, _> = HashTable::new(simple_hash, 16);
    ht.insert(1, "one".to_string());
    ht.insert(1, "ONE".to_string());
    assert_eq!(ht.get(&1), Some(&"ONE".to_string()));
    assert_eq!(ht.n_items, 1);
}

#[test]
fn test_remove() {
    let mut ht: HashTable<u32, String, _> = HashTable::new(simple_hash, 16);
    ht.insert(1, "one".to_string());
    ht.insert(2, "two".to_string());
    ht.remove(&1);
    assert_eq!(ht.get(&1), None);
    assert_eq!(ht.get(&2), Some(&"two".to_string()));
    assert_eq!(ht.n_items, 1);
}

#[test]
fn test_resize() {
    let mut ht: HashTable<u32, u32, _> = HashTable::new(simple_hash, 4);
    for i in 0..20 {
        ht.insert(i, i * 10);
    }
    for i in 0..20 {
        assert_eq!(ht.get(&i), Some(&(i * 10)));
    }
    assert_eq!(ht.n_items, 20);
}

#[test]
fn test_empty_table() {
    let ht: HashTable<u32, u32, _> = HashTable::new(simple_hash, 8);
    assert_eq!(ht.get(&0), None);
    assert_eq!(ht.n_items, 0);
}

fn main() {}
