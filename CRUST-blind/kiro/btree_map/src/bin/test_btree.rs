use btree_map::btree::{BTree, calc_key_hash, min_size};

// Helper: C sizeof("entry_1") = 8 (includes null terminator)
fn c_str(s: &str) -> Vec<u8> {
    let mut v = s.as_bytes().to_vec();
    v.push(0); // null terminator like C sizeof includes
    v
}

// ==================== calc_key_hash tests ====================

#[test]
fn test_hash_empty() {
    assert_eq!(calc_key_hash(&vec![], 0), 0);
}

#[test]
fn test_hash_single_char() {
    assert_eq!(calc_key_hash(&vec![b'a'], 1), 97);
}

#[test]
fn test_hash_two_chars() {
    assert_eq!(calc_key_hash(&vec![b'a', b'b'], 2), 293);
}

#[test]
fn test_hash_three_chars() {
    assert_eq!(calc_key_hash(&vec![b'a', b'b', b'c'], 3), 590);
}

#[test]
fn test_hash_entry_strings() {
    assert_eq!(calc_key_hash(&c_str("entry_1"), 8), 2643);
    assert_eq!(calc_key_hash(&c_str("entry_2"), 8), 2650);
    assert_eq!(calc_key_hash(&c_str("entry_3"), 8), 2657);
    assert_eq!(calc_key_hash(&c_str("entry_4"), 8), 2664);
    assert_eq!(calc_key_hash(&c_str("entry_5"), 8), 2671);
}

#[test]
fn test_hash_binary_keys() {
    // uint32_t key = 1 -> little-endian bytes [1, 0, 0, 0]
    let int_key: Vec<u8> = 1u32.to_le_bytes().to_vec();
    assert_eq!(calc_key_hash(&int_key, 4), 1);

    // uint64_t key = 10 -> little-endian bytes [10, 0, 0, 0, 0, 0, 0, 0]
    let long_key: Vec<u8> = 10u64.to_le_bytes().to_vec();
    assert_eq!(calc_key_hash(&long_key, 8), 10);

    // uint8_t key = 9
    let byte_key: Vec<u8> = vec![9u8];
    assert_eq!(calc_key_hash(&byte_key, 1), 9);
}

#[test]
fn test_hash_struct_key() {
    // struct { uint32_t k1=1, uint32_t k2=2 } -> [1,0,0,0, 2,0,0,0]
    let mut key = 1u32.to_le_bytes().to_vec();
    key.extend_from_slice(&2u32.to_le_bytes());
    assert_eq!(calc_key_hash(&key, 8), 11);
}

// ==================== min_size tests ====================

#[test]
fn test_min_size() {
    assert_eq!(min_size(3, 5), 3);
    assert_eq!(min_size(5, 3), 3);
    assert_eq!(min_size(0, 0), 0);
    assert_eq!(min_size(10, 10), 10);
}

// ==================== BTree basic tests ====================

#[test]
fn test_new_btree_empty() {
    let bt = BTree::new_btree();
    assert_eq!(bt.get_entry_count(), 0);
    let list = bt.list_entries();
    assert_eq!(list.len, 0);
}

#[test]
fn test_find_in_empty_tree() {
    let bt = BTree::new_btree();
    let key = c_str("x");
    assert!(bt.find_entry(&key, key.len()).is_none());
}

#[test]
fn test_remove_from_empty_tree() {
    let mut bt = BTree::new_btree();
    let key = vec![b'x'];
    bt.remove_entry(&key, 1);
    assert_eq!(bt.get_entry_count(), 0);
}

// ==================== add_entry / find_entry tests ====================

#[test]
fn test_add_and_find_single() {
    let mut bt = BTree::new_btree();
    let key = c_str("entry_1");
    let val = c_str("value_1");
    bt.add_entry(key.clone(), key.len(), val.clone(), val.len());

    let found = bt.find_entry(&key, key.len());
    assert!(found.is_some());
    let v = found.unwrap();
    assert_eq!(v.value, val);
    assert_eq!(v.len, val.len());
}

#[test]
fn test_add_five_entries_count() {
    let mut bt = BTree::new_btree();
    for i in 1..=5 {
        let key = c_str(&format!("entry_{}", i));
        let val = c_str(&format!("value_{}", i));
        bt.add_entry(key.clone(), key.len(), val.clone(), val.len());
    }
    assert_eq!(bt.get_entry_count(), 5);
}

#[test]
fn test_find_all_five_entries() {
    let mut bt = BTree::new_btree();
    for i in 1..=5 {
        let key = c_str(&format!("entry_{}", i));
        let val = c_str(&format!("value_{}", i));
        bt.add_entry(key.clone(), key.len(), val.clone(), val.len());
    }
    for i in 1..=5 {
        let key = c_str(&format!("entry_{}", i));
        let val = c_str(&format!("value_{}", i));
        let found = bt.find_entry(&key, key.len()).unwrap();
        assert_eq!(found.value, val);
        assert_eq!(found.len, val.len());
    }
}

#[test]
fn test_find_nonexistent() {
    let mut bt = BTree::new_btree();
    let key = c_str("key_1");
    let val = c_str("val_1");
    bt.add_entry(key.clone(), key.len(), val.clone(), val.len());

    let no_key = c_str("no_key");
    assert!(bt.find_entry(&no_key, no_key.len()).is_none());
}

// ==================== Update existing key ====================

#[test]
fn test_update_existing_key() {
    let mut bt = BTree::new_btree();
    let key = c_str("key_1");
    let old_val = c_str("old_val");
    let new_val = c_str("new_val");

    bt.add_entry(key.clone(), key.len(), old_val.clone(), old_val.len());
    bt.add_entry(key.clone(), key.len(), new_val.clone(), new_val.len());

    assert_eq!(bt.get_entry_count(), 1);
    let found = bt.find_entry(&key, key.len()).unwrap();
    assert_eq!(found.value, new_val);
}

// ==================== list_entries tests ====================

#[test]
fn test_list_entries_ordering() {
    let mut bt = BTree::new_btree();
    for i in 1..=5 {
        let key = c_str(&format!("entry_{}", i));
        let val = c_str(&format!("value_{}", i));
        bt.add_entry(key.clone(), key.len(), val.clone(), val.len());
    }
    let list = bt.list_entries();
    assert_eq!(list.len, 5);
    assert_eq!(list.cap, 5);
    assert_eq!(list.entries.len(), 5);

    // In-order traversal should be sorted by key_hash ascending
    // entry_1(2643) < entry_2(2650) < entry_3(2657) < entry_4(2664) < entry_5(2671)
    let expected_keys: Vec<Vec<u8>> = (1..=5).map(|i| c_str(&format!("entry_{}", i))).collect();
    let expected_vals: Vec<Vec<u8>> = (1..=5).map(|i| c_str(&format!("value_{}", i))).collect();

    for (idx, entry) in list.entries.iter().enumerate() {
        assert_eq!(entry.key.key, expected_keys[idx]);
        assert_eq!(entry.key.len, expected_keys[idx].len());
        assert_eq!(entry.value.value, expected_vals[idx]);
        assert_eq!(entry.value.len, expected_vals[idx].len());
    }
}

#[test]
fn test_list_entries_empty() {
    let bt = BTree::new_btree();
    let list = bt.list_entries();
    assert_eq!(list.len, 0);
    assert_eq!(list.cap, 0);
    assert!(list.entries.is_empty());
}

// ==================== remove_entry tests ====================

#[test]
fn test_remove_middle_entry() {
    let mut bt = BTree::new_btree();
    for i in 1..=5 {
        let key = c_str(&format!("entry_{}", i));
        let val = c_str(&format!("value_{}", i));
        bt.add_entry(key.clone(), key.len(), val.clone(), val.len());
    }

    let rm_key = c_str("entry_3");
    bt.remove_entry(&rm_key, rm_key.len());
    assert_eq!(bt.get_entry_count(), 4);

    // Verify entry_3 is gone
    assert!(bt.find_entry(&rm_key, rm_key.len()).is_none());

    // Verify remaining entries in order
    let list = bt.list_entries();
    assert_eq!(list.len, 4);
    let remaining = [1, 2, 4, 5];
    for (idx, &i) in remaining.iter().enumerate() {
        let expected_key = c_str(&format!("entry_{}", i));
        let expected_val = c_str(&format!("value_{}", i));
        assert_eq!(list.entries[idx].key.key, expected_key);
        assert_eq!(list.entries[idx].value.value, expected_val);
    }
}

#[test]
fn test_remove_nonexistent_key() {
    let mut bt = BTree::new_btree();
    let key = vec![b'a'];
    let val = vec![b'v', b'a'];
    bt.add_entry(key.clone(), 1, val.clone(), 2);

    let bad_key = vec![b'b'];
    bt.remove_entry(&bad_key, 1);
    assert_eq!(bt.get_entry_count(), 1);
}

#[test]
fn test_remove_only_entry() {
    let mut bt = BTree::new_btree();
    let key = c_str("only");
    let val = c_str("val");
    bt.add_entry(key.clone(), key.len(), val.clone(), val.len());
    assert_eq!(bt.get_entry_count(), 1);

    bt.remove_entry(&key, key.len());
    assert_eq!(bt.get_entry_count(), 0);
    assert!(bt.find_entry(&key, key.len()).is_none());
}

// ==================== free_tree tests ====================

#[test]
fn test_free_tree() {
    let mut bt = BTree::new_btree();
    let key = c_str("k");
    let val = c_str("v");
    bt.add_entry(key.clone(), key.len(), val.clone(), val.len());
    bt.free_tree();
    assert_eq!(bt.get_entry_count(), 0);
    assert!(bt.find_entry(&key, key.len()).is_none());
}

// ==================== Binary key types (matching C test_multiple_key_types) ====================

#[test]
fn test_multiple_key_types() {
    let mut bt = BTree::new_btree();

    // String key with null terminator
    let str_key = c_str("entry_1");
    let str_val = c_str("value_1");
    bt.add_entry(str_key.clone(), str_key.len(), str_val.clone(), str_val.len());

    // uint32_t key = 1
    let int_key: Vec<u8> = 1u32.to_le_bytes().to_vec();
    let int_val = c_str("value_2");
    bt.add_entry(int_key.clone(), int_key.len(), int_val.clone(), int_val.len());

    // uint64_t key = 10
    let long_key: Vec<u8> = 10u64.to_le_bytes().to_vec();
    let long_val = c_str("value_3");
    bt.add_entry(long_key.clone(), long_key.len(), long_val.clone(), long_val.len());

    // uint8_t key = 9
    let byte_key: Vec<u8> = vec![9u8];
    let byte_val = c_str("value_4");
    bt.add_entry(byte_key.clone(), byte_key.len(), byte_val.clone(), byte_val.len());

    assert_eq!(bt.get_entry_count(), 4);

    let f1 = bt.find_entry(&str_key, str_key.len()).unwrap();
    assert_eq!(f1.value, str_val);

    let f2 = bt.find_entry(&int_key, int_key.len()).unwrap();
    assert_eq!(f2.value, int_val);

    let f3 = bt.find_entry(&long_key, long_key.len()).unwrap();
    assert_eq!(f3.value, long_val);

    let f4 = bt.find_entry(&byte_key, byte_key.len()).unwrap();
    assert_eq!(f4.value, byte_val);
}

// ==================== Struct key/value (matching C tests) ====================

#[test]
fn test_struct_key() {
    let mut bt = BTree::new_btree();
    let mut key = 1u32.to_le_bytes().to_vec();
    key.extend_from_slice(&2u32.to_le_bytes());
    let val = c_str("value_1");
    bt.add_entry(key.clone(), key.len(), val.clone(), val.len());

    let found = bt.find_entry(&key, key.len()).unwrap();
    assert_eq!(found.value, val);
}

#[test]
fn test_struct_value() {
    let mut bt = BTree::new_btree();
    let key = c_str("key_1");
    // custom_value_t { uint32_t value=1, uint32_t value2=2 }
    let mut val = 1u32.to_le_bytes().to_vec();
    val.extend_from_slice(&2u32.to_le_bytes());
    bt.add_entry(key.clone(), key.len(), val.clone(), val.len());

    let found = bt.find_entry(&key, key.len()).unwrap();
    assert_eq!(found.value, val);
    assert_eq!(found.len, 8);
}

// ==================== Key truncation to BTREE_KEY_SIZE ====================

#[test]
fn test_key_truncation() {
    let mut bt = BTree::new_btree();
    // Key longer than 10 bytes gets truncated
    let long_key = b"12345678901234".to_vec(); // 14 bytes
    let val = c_str("val");
    bt.add_entry(long_key.clone(), long_key.len(), val.clone(), val.len());

    // Should be findable with the same long key (find_entry also truncates)
    let found = bt.find_entry(&long_key, long_key.len()).unwrap();
    assert_eq!(found.value, val);

    // The stored key in list should be truncated to 10
    let list = bt.list_entries();
    assert_eq!(list.entries[0].key.len, 10);
    assert_eq!(list.entries[0].key.key, b"1234567890".to_vec());
}

// ==================== Remove all entries one by one ====================

#[test]
fn test_remove_all_entries() {
    let mut bt = BTree::new_btree();
    for i in 1..=3 {
        let key = c_str(&format!("e{}", i));
        let val = c_str(&format!("v{}", i));
        bt.add_entry(key.clone(), key.len(), val.clone(), val.len());
    }
    assert_eq!(bt.get_entry_count(), 3);

    for i in 1..=3 {
        let key = c_str(&format!("e{}", i));
        bt.remove_entry(&key, key.len());
    }
    assert_eq!(bt.get_entry_count(), 0);
    assert!(bt.list_entries().entries.is_empty());
}

fn main() {}
