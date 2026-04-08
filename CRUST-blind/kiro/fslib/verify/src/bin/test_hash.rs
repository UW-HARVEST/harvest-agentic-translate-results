use fslib::hash::HashTable;

fn int_hash(k: &i32) -> usize {
    *k as usize
}

#[test]
fn test_insert_get() {
    let mut h: HashTable<i32, i32, _> = HashTable::new(int_hash, 16);
    assert_eq!(h.n_items, 0);
    h.insert(1, 100);
    h.insert(2, 200);
    h.insert(3, 300);
    assert_eq!(h.n_items, 3);
    assert_eq!(h.get(&1), Some(&100));
    assert_eq!(h.get(&2), Some(&200));
    assert_eq!(h.get(&3), Some(&300));
}

#[test]
fn test_update() {
    let mut h: HashTable<i32, i32, _> = HashTable::new(int_hash, 16);
    h.insert(1, 100);
    h.insert(1, 999);
    assert_eq!(h.get(&1), Some(&999));
    assert_eq!(h.n_items, 1);
}

#[test]
fn test_remove() {
    let mut h: HashTable<i32, i32, _> = HashTable::new(int_hash, 16);
    h.insert(1, 100);
    h.insert(2, 200);
    h.insert(3, 300);
    h.remove(&2);
    assert_eq!(h.n_items, 2);
    assert_eq!(h.get(&2), None);
    assert_eq!(h.get(&1), Some(&100));
    assert_eq!(h.get(&3), Some(&300));
}

#[test]
fn test_get_missing() {
    let h: HashTable<i32, i32, _> = HashTable::new(int_hash, 16);
    assert_eq!(h.get(&42), None);
}

fn main() {}
