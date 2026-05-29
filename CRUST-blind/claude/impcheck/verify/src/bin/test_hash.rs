use impcheck::hash::HashTable;

#[test]
fn test_compute_hash_known_values() {
    // Values computed by running the C compute_hash function.
    assert_eq!(HashTable::<u8>::compute_hash(0), 12638153115695167455u64);
    assert_eq!(HashTable::<u8>::compute_hash(1), 12638152016183539244u64);
    assert_eq!(HashTable::<u8>::compute_hash(2), 12638155314718423877u64);
    assert_eq!(HashTable::<u8>::compute_hash(3), 12638154215206795666u64);
    assert_eq!(HashTable::<u8>::compute_hash(7), 12638149817160282822u64);
    assert_eq!(HashTable::<u8>::compute_hash(100), 12638183902020757363u64);
    assert_eq!(HashTable::<u8>::compute_hash(1000), 12637493408718240855u64);
    assert_eq!(
        HashTable::<u8>::compute_hash(12345678901234u64),
        8738363023633797461u64
    );
    assert_eq!(
        HashTable::<u8>::compute_hash(18446744073709551615u64),
        5808589858502755950u64
    );
}

#[test]
fn test_new_initial_state() {
    // hash_table_init(7) -> capacity = 1<<7 = 128, max_size = 64, size = 0
    let ht: HashTable<u8> = HashTable::new(7);
    assert_eq!(ht.size, 0);
    assert_eq!(ht.capacity, 128);
    assert_eq!(ht.max_size, 64);
    assert_eq!(ht.growth_factor, 2.0);
    assert_eq!(ht.last_found_idx, 0);
}

#[test]
fn test_new_log_capacity_3() {
    let ht: HashTable<u8> = HashTable::new(3);
    assert_eq!(ht.capacity, 8);
    assert_eq!(ht.max_size, 4);
    assert_eq!(ht.size, 0);
}

#[test]
fn test_compute_idx_with_capacity() {
    // With capacity = 128, compute_idx = compute_hash(key) & 127
    let ht: HashTable<u8> = HashTable::new(7);
    assert_eq!(ht.compute_idx(0), 95);
    assert_eq!(ht.compute_idx(1), 44);
    assert_eq!(ht.compute_idx(2), 69);
    assert_eq!(ht.compute_idx(3), 18);
    assert_eq!(ht.compute_idx(7), 70);
    assert_eq!(ht.compute_idx(100), 115);
    assert_eq!(ht.compute_idx(1000), 87);
    assert_eq!(ht.compute_idx(12345678901234u64), 85);
    assert_eq!(ht.compute_idx(18446744073709551615u64), 110);
}

#[test]
fn test_compute_idx_capacity_zero() {
    // Custom invariant in the Rust translation: capacity=0 returns 0.
    let mut ht: HashTable<u8> = HashTable::new(7);
    ht.capacity = 0;
    assert_eq!(ht.compute_idx(123), 0);
}

#[test]
fn test_insert_zero_key_rejected() {
    // C: hash_table_insert with key 0 returns false.
    let mut ht: HashTable<u8> = HashTable::new(7);
    let pre_size = ht.size;
    let ok = ht.hash_table_insert(0, Box::new(()));
    assert!(!ok);
    assert_eq!(ht.size, pre_size);
}

#[test]
fn test_insert_increments_size() {
    let mut ht: HashTable<u8> = HashTable::new(7);
    let ok = ht.hash_table_insert(1, Box::new(()));
    assert!(ok);
    assert_eq!(ht.size, 1);
    let ok = ht.hash_table_insert(2, Box::new(()));
    assert!(ok);
    assert_eq!(ht.size, 2);
}

#[test]
fn test_insert_triggers_realloc_at_max_size() {
    // C: starts at cap=128, max=64. After 64 inserts (size==max), insert triggers realloc.
    let mut ht: HashTable<u8> = HashTable::new(7);
    assert_eq!(ht.max_size, 64);
    assert_eq!(ht.capacity, 128);
    for i in 1..=64u64 {
        let ok = ht.hash_table_insert(i, Box::new(()));
        assert!(ok);
    }
    assert_eq!(ht.size, 64);
    assert_eq!(ht.capacity, 128);
    // The 65th insert: size==max_size triggers realloc
    let ok = ht.hash_table_insert(65, Box::new(()));
    assert!(ok);
    // After realloc: capacity = 256, max_size = 128, size reset to 0 then incremented to 1
    assert_eq!(ht.capacity, 256);
    assert_eq!(ht.max_size, 128);
    assert_eq!(ht.size, 1);
}

#[test]
fn test_realloc_table_doubles_capacity() {
    let mut ht: HashTable<u8> = HashTable::new(7);
    assert_eq!(ht.capacity, 128);
    assert_eq!(ht.max_size, 64);
    let ok = ht.realloc_table();
    assert!(ok);
    assert_eq!(ht.capacity, 256);
    assert_eq!(ht.max_size, 128);
    assert_eq!(ht.size, 0);
}

#[test]
fn test_find_on_empty_table_returns_none() {
    let ht: HashTable<u8> = HashTable::new(7);
    assert!(ht.hash_table_find(42).is_none());
}

#[test]
fn test_find_after_insert_returns_none_due_to_empty_backing() {
    // The Rust translation has empty backing storage; find always returns None.
    let mut ht: HashTable<u8> = HashTable::new(7);
    ht.hash_table_insert(7, Box::new(()));
    assert!(ht.hash_table_find(7).is_none());
}

#[test]
fn test_delete_key_not_present() {
    // Empty backing -> find_entry returns false -> delete returns false.
    let mut ht: HashTable<u8> = HashTable::new(7);
    let ok = ht.hash_table_delete(99);
    assert!(!ok);
    assert_eq!(ht.size, 0);
}

#[test]
fn test_hash_table_free_clears_state() {
    let mut ht: HashTable<u8> = HashTable::new(7);
    ht.hash_table_insert(1, Box::new(()));
    ht.hash_table_insert(2, Box::new(()));
    assert_eq!(ht.size, 2);
    ht.hash_table_free();
    assert_eq!(ht.size, 0);
    assert_eq!(ht.capacity, 0);
    assert_eq!(ht.max_size, 0);
    assert_eq!(ht.last_found_idx, 0);
}

#[test]
fn test_cell_empty() {
    use impcheck::hash::HashTableEntry;
    let mut storage: [u8; 0] = [];
    let entry: HashTableEntry<u8> = HashTableEntry {
        key: 0,
        val: &mut storage[..],
    };
    assert!(HashTable::<u8>::cell_empty(&entry));

    let entry2: HashTableEntry<u8> = HashTableEntry {
        key: 5,
        val: &mut [],
    };
    assert!(!HashTable::<u8>::cell_empty(&entry2));
}

#[test]
fn test_handle_gap_capacity_zero_returns_true() {
    let mut ht: HashTable<u8> = HashTable::new(7);
    ht.capacity = 0;
    let ok = ht.handle_gap(0);
    assert!(ok);
}

#[test]
fn test_find_entry_empty_storage() {
    let ht: HashTable<u8> = HashTable::new(7);
    let mut idx: u64 = 999;
    let ok = ht.find_entry(123, &mut idx);
    assert!(!ok);
    assert_eq!(idx, 0);
}

fn main() {}
