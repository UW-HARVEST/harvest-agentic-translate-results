use impcheck::hash::HashTable;

#[test]
fn test_compute_hash_values() {
    // Verified values from running the C compute_hash function:
    assert_eq!(HashTable::<i32>::compute_hash(1), 12638152016183539244u64);
    assert_eq!(HashTable::<i32>::compute_hash(2), 12638155314718423877u64);
    assert_eq!(HashTable::<i32>::compute_hash(3), 12638154215206795666u64);
    assert_eq!(HashTable::<i32>::compute_hash(100), 12638183902020757363u64);
    assert_eq!(HashTable::<i32>::compute_hash(1000), 12637493408718240855u64);
    assert_eq!(HashTable::<i32>::compute_hash(0), 12638153115695167455u64);
    assert_eq!(HashTable::<i32>::compute_hash(12345), 12633639620461361300u64);
}

#[test]
fn test_init_basic() {
    let mut ht: HashTable<i32> = HashTable::new(7);
    assert_eq!(ht.size, 0);
    assert_eq!(ht.capacity, 1u64 << 7);
    assert_eq!(ht.max_size, (1u64 << 7) >> 1);
    assert_eq!(ht.growth_factor, 2.0);
    ht.hash_table_free();
}

#[test]
fn test_compute_idx() {
    let mut ht: HashTable<i32> = HashTable::new(7);
    let key = 1u64;
    let h = HashTable::<i32>::compute_hash(key);
    let expected = h & (ht.capacity - 1);
    assert_eq!(ht.compute_idx(key), expected);
    ht.hash_table_free();
}

#[test]
fn test_insert_find_simple() {
    let mut ht: HashTable<i32> = HashTable::new(7);
    let ok = ht.hash_table_insert(1, Box::new(42i32));
    assert!(ok);
    assert_eq!(ht.size, 1);
    let res = ht.hash_table_find(1);
    assert!(res.is_some());
    let bx = res.unwrap();
    let v = bx.downcast_ref::<i32>().unwrap();
    assert_eq!(*v, 42);
    ht.hash_table_free();
}

#[test]
fn test_insert_zero_key() {
    let mut ht: HashTable<i32> = HashTable::new(4);
    let ok = ht.hash_table_insert(0, Box::new(99i32));
    assert!(!ok);
    assert_eq!(ht.size, 0);
    ht.hash_table_free();
}

#[test]
fn test_insert_duplicate() {
    let mut ht: HashTable<i32> = HashTable::new(4);
    let ok1 = ht.hash_table_insert(5, Box::new(1i32));
    assert!(ok1);
    let ok2 = ht.hash_table_insert(5, Box::new(2i32));
    assert!(!ok2);
    assert_eq!(ht.size, 1);
    ht.hash_table_free();
}

#[test]
fn test_find_missing() {
    let mut ht: HashTable<i32> = HashTable::new(4);
    let res = ht.hash_table_find(7);
    assert!(res.is_none());
    ht.hash_table_free();
}

#[test]
fn test_delete() {
    let mut ht: HashTable<i32> = HashTable::new(5);
    let _ = ht.hash_table_insert(1, Box::new(10i32));
    let _ = ht.hash_table_insert(2, Box::new(20i32));
    assert_eq!(ht.size, 2);
    let ok = ht.hash_table_delete(1);
    assert!(ok);
    assert_eq!(ht.size, 1);
    assert!(ht.hash_table_find(1).is_none());
    let bx = ht.hash_table_find(2).unwrap();
    assert_eq!(*bx.downcast_ref::<i32>().unwrap(), 20);
    ht.hash_table_free();
}

#[test]
fn test_delete_missing() {
    let mut ht: HashTable<i32> = HashTable::new(4);
    let ok = ht.hash_table_delete(99);
    assert!(!ok);
    ht.hash_table_free();
}

#[test]
fn test_grow_at_load_factor() {
    // Mimic the C test: cap=128, max=64, fits 64 -> grow to 256 max=128
    let mut ht: HashTable<i32> = HashTable::new(7);
    assert_eq!(ht.capacity, 128);
    for i in 1..=63 {
        let ok = ht.hash_table_insert(i as u64, Box::new(i as i32));
        assert!(ok);
        assert_eq!(ht.size, i as u64);
        assert_eq!(ht.capacity, 128);
    }
    // Insert 64th element: triggers resize at size==max_size
    let ok = ht.hash_table_insert(64, Box::new(64i32));
    assert!(ok);
    assert_eq!(ht.size, 64);
    assert_eq!(ht.capacity, 128);

    let ok = ht.hash_table_insert(65, Box::new(65i32));
    assert!(ok);
    assert_eq!(ht.size, 65);
    assert_eq!(ht.capacity, 256);

    // Verify all stored
    for i in 1..=65u64 {
        let res = ht.hash_table_find(i);
        assert!(res.is_some(), "missing key {}", i);
        assert_eq!(*res.unwrap().downcast_ref::<i32>().unwrap(), i as i32);
    }

    ht.hash_table_free();
}

#[test]
fn test_delete_last_found() {
    let mut ht: HashTable<i32> = HashTable::new(4);
    let _ = ht.hash_table_insert(10, Box::new(100i32));
    let _ = ht.hash_table_insert(20, Box::new(200i32));
    assert!(ht.hash_table_find(10).is_some());
    let ok = ht.hash_table_delete_last_found();
    assert!(ok);
    assert!(ht.hash_table_find(10).is_none());
    assert_eq!(ht.size, 1);
    ht.hash_table_free();
}

#[test]
fn test_cell_empty() {
    use impcheck::hash::HashTableEntry;
    let mut empty_arr: Vec<i32> = Vec::new();
    let entry = HashTableEntry {
        key: 0u64,
        val: empty_arr.as_mut_slice(),
    };
    assert!(HashTable::<i32>::cell_empty(&entry));
    let entry2 = HashTableEntry {
        key: 1u64,
        val: empty_arr.as_mut_slice(),
    };
    assert!(!HashTable::<i32>::cell_empty(&entry2));
}

#[test]
fn test_find_entry_returns_idx_for_missing() {
    let mut ht: HashTable<i32> = HashTable::new(7);
    let mut idx: u64 = 0;
    let found = ht.find_entry(123, &mut idx);
    assert!(!found);
    // For empty table, idx should equal compute_idx
    let expected = ht.compute_idx(123);
    assert_eq!(idx, expected);
    ht.hash_table_free();
}

fn main() {}
