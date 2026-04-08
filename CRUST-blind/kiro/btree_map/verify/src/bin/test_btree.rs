use btree_map::btree::*;

// Helper: C sizeof("str") includes null terminator, so "entry_1" -> b"entry_1\0" (len 8)
fn c_str(s: &str) -> Vec<u8> {
    let mut v = s.as_bytes().to_vec();
    v.push(0);
    v
}

// === calc_key_hash ===

#[test]
fn test_calc_key_hash_entry_1() {
    assert_eq!(calc_key_hash(&c_str("entry_1"), 8), 2643);
}

#[test]
fn test_calc_key_hash_entry_2() {
    assert_eq!(calc_key_hash(&c_str("entry_2"), 8), 2650);
}

#[test]
fn test_calc_key_hash_entry_3() {
    assert_eq!(calc_key_hash(&c_str("entry_3"), 8), 2657);
}

#[test]
fn test_calc_key_hash_entry_4() {
    assert_eq!(calc_key_hash(&c_str("entry_4"), 8), 2664);
}

#[test]
fn test_calc_key_hash_entry_5() {
    assert_eq!(calc_key_hash(&c_str("entry_5"), 8), 2671);
}

#[test]
fn test_calc_key_hash_hello() {
    assert_eq!(calc_key_hash(&c_str("hello"), 6), 1617);
}

#[test]
fn test_calc_key_hash_single_char() {
    assert_eq!(calc_key_hash(&c_str("a"), 2), 97);
}

#[test]
fn test_calc_key_hash_null_only() {
    // sizeof("") = 1 in C (just the null terminator)
    assert_eq!(calc_key_hash(&c_str(""), 1), 0);
}

#[test]
fn test_calc_key_hash_zero_len() {
    assert_eq!(calc_key_hash(&vec![], 0), 0);
}

#[test]
fn test_calc_key_hash_binary_u32() {
    // uint32_t 1 in little-endian: [1,0,0,0]
    assert_eq!(calc_key_hash(&vec![1, 0, 0, 0], 4), 1);
}

#[test]
fn test_calc_key_hash_binary_u64() {
    // uint64_t 10 in little-endian: [10,0,0,0,0,0,0,0]
    assert_eq!(calc_key_hash(&vec![10, 0, 0, 0, 0, 0, 0, 0], 8), 10);
}

#[test]
fn test_calc_key_hash_binary_u8() {
    assert_eq!(calc_key_hash(&vec![9], 1), 9);
}

// === add_entry + find_entry ===

#[test]
fn test_add_and_find_entry() {
    let mut bt = BTree::new_btree();
    bt.add_entry(c_str("entry_1"), 8, c_str("value_1"), 8);
    let v = bt.find_entry(&c_str("entry_1"), 8).unwrap();
    assert_eq!(v.len, 8);
    assert_eq!(v.value, c_str("value_1"));
}

#[test]
fn test_find_nonexistent() {
    let mut bt = BTree::new_btree();
    bt.add_entry(c_str("entry_1"), 8, c_str("value_1"), 8);
    assert!(bt.find_entry(&c_str("nonexist"), 9).is_none());
}

// === get_entry_count ===

#[test]
fn test_entry_count_increments() {
    let mut bt = BTree::new_btree();
    assert_eq!(bt.get_entry_count(), 0);
    bt.add_entry(c_str("entry_1"), 8, c_str("value_1"), 8);
    assert_eq!(bt.get_entry_count(), 1);
    bt.add_entry(c_str("entry_2"), 8, c_str("value_2"), 8);
    bt.add_entry(c_str("entry_3"), 8, c_str("value_3"), 8);
    bt.add_entry(c_str("entry_4"), 8, c_str("value_4"), 8);
    bt.add_entry(c_str("entry_5"), 8, c_str("value_5"), 8);
    assert_eq!(bt.get_entry_count(), 5);
}

// === list_entries ===

#[test]
fn test_list_entries_order_and_content() {
    let mut bt = BTree::new_btree();
    for i in 1..=5 {
        bt.add_entry(c_str(&format!("entry_{}", i)), 8, c_str(&format!("value_{}", i)), 8);
    }
    let list = bt.list_entries();
    assert_eq!(list.len, 5);
    assert_eq!(list.cap, 5);
    // In-order traversal sorted by hash ascending: entry_1..entry_5
    for i in 0..5 {
        let expected_key = c_str(&format!("entry_{}", i + 1));
        let expected_val = c_str(&format!("value_{}", i + 1));
        assert_eq!(list.entries[i].key.len, 8);
        assert_eq!(list.entries[i].key.key, expected_key);
        assert_eq!(list.entries[i].value.len, 8);
        assert_eq!(list.entries[i].value.value, expected_val);
    }
}

// === remove_entry ===

#[test]
fn test_remove_entry_middle() {
    let mut bt = BTree::new_btree();
    for i in 1..=5 {
        bt.add_entry(c_str(&format!("entry_{}", i)), 8, c_str(&format!("value_{}", i)), 8);
    }
    bt.remove_entry(&c_str("entry_3"), 8);
    assert_eq!(bt.get_entry_count(), 4);
    assert!(bt.find_entry(&c_str("entry_3"), 8).is_none());

    let list = bt.list_entries();
    assert_eq!(list.len, 4);
    let expected_keys = ["entry_1", "entry_2", "entry_4", "entry_5"];
    let expected_vals = ["value_1", "value_2", "value_4", "value_5"];
    for i in 0..4 {
        assert_eq!(list.entries[i].key.key, c_str(expected_keys[i]));
        assert_eq!(list.entries[i].key.len, 8);
        assert_eq!(list.entries[i].value.value, c_str(expected_vals[i]));
        assert_eq!(list.entries[i].value.len, 8);
    }
}

// === update existing key ===

#[test]
fn test_update_existing_key() {
    let mut bt = BTree::new_btree();
    bt.add_entry(c_str("key1"), 5, c_str("old_val"), 8);
    bt.add_entry(c_str("key1"), 5, c_str("new_val"), 8);
    assert_eq!(bt.get_entry_count(), 1);
    let v = bt.find_entry(&c_str("key1"), 5).unwrap();
    assert_eq!(v.len, 8);
    assert_eq!(v.value, c_str("new_val"));
}

// === empty tree operations ===

#[test]
fn test_empty_tree_count() {
    let bt = BTree::new_btree();
    assert_eq!(bt.get_entry_count(), 0);
}

#[test]
fn test_empty_tree_find() {
    let bt = BTree::new_btree();
    assert!(bt.find_entry(&c_str("x"), 2).is_none());
}

#[test]
fn test_empty_tree_list() {
    let bt = BTree::new_btree();
    let list = bt.list_entries();
    assert_eq!(list.len, 0);
    assert_eq!(list.cap, 0);
}

#[test]
fn test_empty_tree_remove() {
    let mut bt = BTree::new_btree();
    bt.remove_entry(&c_str("x"), 2); // should not panic
    assert_eq!(bt.get_entry_count(), 0);
}

// === binary key types ===

#[test]
fn test_binary_keys() {
    let mut bt = BTree::new_btree();
    // uint32_t key = 1 (little-endian)
    bt.add_entry(vec![1, 0, 0, 0], 4, c_str("val_int"), 8);
    // uint64_t key = 10
    bt.add_entry(vec![10, 0, 0, 0, 0, 0, 0, 0], 8, c_str("val_long"), 9);
    // uint8_t key = 9
    bt.add_entry(vec![9], 1, c_str("val_byte"), 9);

    let vi = bt.find_entry(&vec![1, 0, 0, 0], 4).unwrap();
    assert_eq!(vi.len, 8);
    assert_eq!(vi.value, c_str("val_int"));

    let vl = bt.find_entry(&vec![10, 0, 0, 0, 0, 0, 0, 0], 8).unwrap();
    assert_eq!(vl.len, 9);
    assert_eq!(vl.value, c_str("val_long"));

    let vb = bt.find_entry(&vec![9], 1).unwrap();
    assert_eq!(vb.len, 9);
    assert_eq!(vb.value, c_str("val_byte"));

    assert_eq!(bt.get_entry_count(), 3);

    // Verify list order: sorted by hash. hash([1,0,0,0])=1, hash([9])=9, hash([10,...])=10
    let list = bt.list_entries();
    assert_eq!(list.len, 3);
    assert_eq!(list.entries[0].key.key, vec![1, 0, 0, 0]);
    assert_eq!(list.entries[0].key.len, 4);
    assert_eq!(list.entries[1].key.key, vec![9]);
    assert_eq!(list.entries[1].key.len, 1);
    assert_eq!(list.entries[2].key.key, vec![10, 0, 0, 0, 0, 0, 0, 0]);
    assert_eq!(list.entries[2].key.len, 8);
}

// === long key truncation ===

#[test]
fn test_long_key_truncation() {
    let mut bt = BTree::new_btree();
    let long_key = b"abcdefghijklmno".to_vec(); // 15 bytes + we pass key_len=16
    bt.add_entry(long_key.clone(), 16, c_str("long_val"), 9);

    // Findable with same long key
    let v = bt.find_entry(&long_key, 16).unwrap();
    assert_eq!(v.len, 9);
    assert_eq!(v.value, c_str("long_val"));

    // Also findable with just first 10 bytes
    let trunc_key = b"abcdefghij".to_vec();
    let v2 = bt.find_entry(&trunc_key, 10).unwrap();
    assert_eq!(v2.len, 9);
    assert_eq!(v2.value, c_str("long_val"));

    assert_eq!(bt.get_entry_count(), 1);

    // Listed key should be truncated to 10 bytes
    let list = bt.list_entries();
    assert_eq!(list.entries[0].key.len, 10);
    assert_eq!(list.entries[0].key.key, b"abcdefghij".to_vec());
}

// === remove all entries ===

#[test]
fn test_remove_all_entries() {
    let mut bt = BTree::new_btree();
    bt.add_entry(c_str("a"), 2, c_str("va"), 3);
    bt.add_entry(c_str("b"), 2, c_str("vb"), 3);
    bt.remove_entry(&c_str("a"), 2);
    bt.remove_entry(&c_str("b"), 2);
    assert_eq!(bt.get_entry_count(), 0);
    assert!(bt.find_entry(&c_str("a"), 2).is_none());
}

// === free_tree ===

#[test]
fn test_free_tree() {
    let mut bt = BTree::new_btree();
    bt.add_entry(c_str("k"), 2, c_str("v"), 2);
    bt.free_tree();
    assert_eq!(bt.get_entry_count(), 0);
    assert!(bt.find_entry(&c_str("k"), 2).is_none());
}

fn main() {}
