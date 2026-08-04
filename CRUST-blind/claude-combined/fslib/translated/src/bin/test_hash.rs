use fslib::hash::HashTable;

fn simple_hash(s: &String) -> usize {
    let mut h: usize = 0;
    for b in s.bytes() {
        h = h.wrapping_mul(31).wrapping_add(b as usize);
    }
    h
}

#[test]
fn test_insert_get() {
    let mut h: HashTable<String, i32, fn(&String) -> usize> = HashTable::new(simple_hash, 16);
    h.insert("apple".to_string(), 1);
    h.insert("banana".to_string(), 2);
    h.insert("cherry".to_string(), 3);
    assert_eq!(h.get(&"apple".to_string()), Some(&1));
    assert_eq!(h.get(&"banana".to_string()), Some(&2));
    assert_eq!(h.get(&"cherry".to_string()), Some(&3));
    assert_eq!(h.get(&"missing".to_string()), None);
    assert_eq!(h.n_items, 3);
}

#[test]
fn test_update() {
    let mut h: HashTable<String, i32, fn(&String) -> usize> = HashTable::new(simple_hash, 16);
    h.insert("k".to_string(), 1);
    h.insert("k".to_string(), 99);
    assert_eq!(h.get(&"k".to_string()), Some(&99));
    assert_eq!(h.n_items, 1);
}

#[test]
fn test_remove() {
    let mut h: HashTable<String, i32, fn(&String) -> usize> = HashTable::new(simple_hash, 16);
    h.insert("a".to_string(), 1);
    h.insert("b".to_string(), 2);
    h.remove(&"a".to_string());
    assert_eq!(h.get(&"a".to_string()), None);
    assert_eq!(h.get(&"b".to_string()), Some(&2));
    assert_eq!(h.n_items, 1);
}

fn main() {}
