use btree_map::btree::{
    calc_key_hash, min_size, BTree, Node, BTREE_KEY_SIZE,
};

// ---------- Helpers ----------

fn s(s: &str) -> Vec<u8> {
    // Mimic C string literal with trailing null terminator (matches sizeof("foo"))
    let mut v = s.as_bytes().to_vec();
    v.push(0);
    v
}

// ---------- Tests for calc_key_hash ----------

#[test]
fn test_calc_key_hash_empty() {
    let key: Vec<u8> = vec![];
    assert_eq!(calc_key_hash(&key, 0), 0);
}

#[test]
fn test_calc_key_hash_single_byte() {
    // 'a' = 97; 97 * 1 = 97
    let key: Vec<u8> = vec![b'a'];
    assert_eq!(calc_key_hash(&key, 1), 97);
}

#[test]
fn test_calc_key_hash_with_null_term() {
    // 'a','\0' (len 2) -> 97*1 + 0*2 = 97
    let key: Vec<u8> = vec![b'a', 0];
    assert_eq!(calc_key_hash(&key, 2), 97);
}

#[test]
fn test_calc_key_hash_abc() {
    // 'a' (97)*1 + 'b'(98)*2 + 'c'(99)*3 = 97 + 196 + 297 = 590
    let key: Vec<u8> = vec![b'a', b'b', b'c'];
    assert_eq!(calc_key_hash(&key, 3), 590);
}

#[test]
fn test_calc_key_hash_entry_keys() {
    // Match values verified from running C
    assert_eq!(calc_key_hash(&s("entry_1"), 8), 2643);
    assert_eq!(calc_key_hash(&s("entry_2"), 8), 2650);
    assert_eq!(calc_key_hash(&s("entry_3"), 8), 2657);
    assert_eq!(calc_key_hash(&s("entry_4"), 8), 2664);
    assert_eq!(calc_key_hash(&s("entry_5"), 8), 2671);
}

#[test]
fn test_calc_key_hash_all_ff() {
    // 0xff * (1 + 2 + ... + 10) = 255 * 55 = 14025
    let key: Vec<u8> = vec![0xff; 10];
    assert_eq!(calc_key_hash(&key, 10), 14025);
}

// ---------- Tests for min_size ----------

#[test]
fn test_min_size_a_lt_b() {
    assert_eq!(min_size(3, 5), 3);
}

#[test]
fn test_min_size_b_lt_a() {
    assert_eq!(min_size(7, 2), 2);
}

#[test]
fn test_min_size_equal() {
    assert_eq!(min_size(4, 4), 4);
}

#[test]
fn test_min_size_zero() {
    assert_eq!(min_size(0, 5), 0);
    assert_eq!(min_size(5, 0), 0);
}

// ---------- Tests for BTREE_KEY_SIZE constant ----------

#[test]
fn test_btree_key_size_constant() {
    assert_eq!(BTREE_KEY_SIZE, 10);
}

// ---------- Tests for BTree::new_btree ----------

#[test]
fn test_new_btree_empty() {
    let bt = BTree::new_btree();
    assert!(bt.node.is_none());
    assert_eq!(bt.get_entry_count(), 0);
}

// ---------- Tests for BTree::add_entry / find_entry ----------

#[test]
fn test_add_and_find_single_entry() {
    let mut bt = BTree::new_btree();
    bt.add_entry(s("entry_1"), 8, s("value_1"), 8);
    let v = bt.find_entry(&s("entry_1"), 8);
    assert!(v.is_some());
    let v = v.unwrap();
    assert_eq!(v.len, 8);
    assert_eq!(v.value, s("value_1"));
}

#[test]
fn test_find_on_empty_returns_none() {
    let bt = BTree::new_btree();
    let v = bt.find_entry(&s("anything"), 9);
    assert!(v.is_none());
}

#[test]
fn test_find_missing_key_returns_none() {
    let mut bt = BTree::new_btree();
    bt.add_entry(s("entry_1"), 8, s("value_1"), 8);
    let v = bt.find_entry(&s("missing"), 8);
    assert!(v.is_none());
}

#[test]
fn test_add_multiple_and_find_all() {
    let mut bt = BTree::new_btree();
    bt.add_entry(s("entry_1"), 8, s("value_1"), 8);
    bt.add_entry(s("entry_2"), 8, s("value_2"), 8);
    bt.add_entry(s("entry_3"), 8, s("value_3"), 8);
    bt.add_entry(s("entry_4"), 8, s("value_4"), 8);
    bt.add_entry(s("entry_5"), 8, s("value_5"), 8);

    let v1 = bt.find_entry(&s("entry_1"), 8).unwrap();
    assert_eq!(v1.len, 8);
    assert_eq!(v1.value, s("value_1"));
    let v2 = bt.find_entry(&s("entry_2"), 8).unwrap();
    assert_eq!(v2.len, 8);
    assert_eq!(v2.value, s("value_2"));
    let v3 = bt.find_entry(&s("entry_3"), 8).unwrap();
    assert_eq!(v3.len, 8);
    assert_eq!(v3.value, s("value_3"));
    let v4 = bt.find_entry(&s("entry_4"), 8).unwrap();
    assert_eq!(v4.len, 8);
    assert_eq!(v4.value, s("value_4"));
    let v5 = bt.find_entry(&s("entry_5"), 8).unwrap();
    assert_eq!(v5.len, 8);
    assert_eq!(v5.value, s("value_5"));
}

#[test]
fn test_add_entry_updates_existing_value() {
    let mut bt = BTree::new_btree();
    bt.add_entry(s("key"), 4, s("v1"), 3);
    bt.add_entry(s("key"), 4, s("v2"), 3);
    let v = bt.find_entry(&s("key"), 4).unwrap();
    assert_eq!(v.len, 3);
    assert_eq!(v.value, s("v2"));
    assert_eq!(bt.get_entry_count(), 1);
}

#[test]
fn test_add_long_key_truncated_to_btree_key_size() {
    // Keys longer than BTREE_KEY_SIZE should be truncated to first 10 bytes for storage.
    let mut bt = BTree::new_btree();
    let long_key: Vec<u8> = b"abcdefghijklmnop\0".to_vec(); // 17 bytes
    bt.add_entry(long_key.clone(), long_key.len(), s("lv"), 3);

    // Find with the same long key works
    let v = bt.find_entry(&long_key, long_key.len()).unwrap();
    assert_eq!(v.len, 3);
    assert_eq!(v.value, s("lv"));

    // Another key sharing the first 10 bytes should also find the same entry,
    // because both are truncated to the same 10-byte prefix.
    let other: Vec<u8> = b"abcdefghijZZZZZZ\0".to_vec();
    let v2 = bt.find_entry(&other, other.len()).unwrap();
    assert_eq!(v2.len, 3);
    assert_eq!(v2.value, s("lv"));
}

// ---------- Tests for BTree::get_entry_count ----------

#[test]
fn test_get_entry_count_empty() {
    let bt = BTree::new_btree();
    assert_eq!(bt.get_entry_count(), 0);
}

#[test]
fn test_get_entry_count_grows() {
    let mut bt = BTree::new_btree();
    assert_eq!(bt.get_entry_count(), 0);
    bt.add_entry(s("entry_1"), 8, s("v"), 2);
    assert_eq!(bt.get_entry_count(), 1);
    bt.add_entry(s("entry_2"), 8, s("v"), 2);
    assert_eq!(bt.get_entry_count(), 2);
    bt.add_entry(s("entry_3"), 8, s("v"), 2);
    assert_eq!(bt.get_entry_count(), 3);
}

// ---------- Tests for BTree::list_entries ----------

#[test]
fn test_list_entries_empty() {
    let bt = BTree::new_btree();
    let list = bt.list_entries();
    assert_eq!(list.len, 0);
    assert_eq!(list.cap, 0);
    assert_eq!(list.entries.len(), 0);
}

#[test]
fn test_list_entries_inorder() {
    // Hashes (sorted ascending in inorder traversal):
    // entry_1=2643 < entry_2=2650 < entry_3=2657 < entry_4=2664 < entry_5=2671
    let mut bt = BTree::new_btree();
    bt.add_entry(s("entry_1"), 8, s("value_1"), 8);
    bt.add_entry(s("entry_2"), 8, s("value_2"), 8);
    bt.add_entry(s("entry_3"), 8, s("value_3"), 8);
    bt.add_entry(s("entry_4"), 8, s("value_4"), 8);
    bt.add_entry(s("entry_5"), 8, s("value_5"), 8);

    let list = bt.list_entries();
    assert_eq!(list.len, 5);
    assert_eq!(list.cap, 5);
    assert_eq!(list.entries.len(), 5);

    let expected_keys = ["entry_1", "entry_2", "entry_3", "entry_4", "entry_5"];
    let expected_values = ["value_1", "value_2", "value_3", "value_4", "value_5"];
    for i in 0..5 {
        assert_eq!(list.entries[i].key.len, 8);
        assert_eq!(list.entries[i].key.key, s(expected_keys[i]));
        assert_eq!(list.entries[i].value.len, 8);
        assert_eq!(list.entries[i].value.value, s(expected_values[i]));
    }
}

#[test]
fn test_list_entries_long_key_truncates_to_10_bytes() {
    let mut bt = BTree::new_btree();
    let long_key: Vec<u8> = b"abcdefghijklmnop\0".to_vec();
    bt.add_entry(long_key.clone(), long_key.len(), s("lv"), 3);

    let list = bt.list_entries();
    assert_eq!(list.len, 1);
    assert_eq!(list.cap, 1);
    assert_eq!(list.entries[0].key.len, 10);
    assert_eq!(
        list.entries[0].key.key,
        b"abcdefghij".to_vec()
    );
    assert_eq!(list.entries[0].value.len, 3);
    assert_eq!(list.entries[0].value.value, s("lv"));
}

// ---------- Tests for BTree::remove_entry ----------

#[test]
fn test_remove_entry_leaf() {
    let mut bt = BTree::new_btree();
    bt.add_entry(s("entry_1"), 8, s("value_1"), 8);
    bt.add_entry(s("entry_2"), 8, s("value_2"), 8);
    bt.add_entry(s("entry_3"), 8, s("value_3"), 8);
    bt.add_entry(s("entry_4"), 8, s("value_4"), 8);
    bt.add_entry(s("entry_5"), 8, s("value_5"), 8);

    bt.remove_entry(&s("entry_3"), 8);
    assert_eq!(bt.get_entry_count(), 4);
    let list = bt.list_entries();
    assert_eq!(list.len, 4);
    assert_eq!(list.entries[0].key.key, s("entry_1"));
    assert_eq!(list.entries[0].value.value, s("value_1"));
    assert_eq!(list.entries[1].key.key, s("entry_2"));
    assert_eq!(list.entries[1].value.value, s("value_2"));
    assert_eq!(list.entries[2].key.key, s("entry_4"));
    assert_eq!(list.entries[2].value.value, s("value_4"));
    assert_eq!(list.entries[3].key.key, s("entry_5"));
    assert_eq!(list.entries[3].value.value, s("value_5"));
}

#[test]
fn test_remove_entry_root() {
    let mut bt = BTree::new_btree();
    bt.add_entry(s("entry_1"), 8, s("value_1"), 8);
    bt.add_entry(s("entry_2"), 8, s("value_2"), 8);
    bt.add_entry(s("entry_3"), 8, s("value_3"), 8);
    bt.add_entry(s("entry_4"), 8, s("value_4"), 8);
    bt.add_entry(s("entry_5"), 8, s("value_5"), 8);

    bt.remove_entry(&s("entry_1"), 8);
    assert_eq!(bt.get_entry_count(), 4);
    let list = bt.list_entries();
    assert_eq!(list.len, 4);
    assert_eq!(list.entries[0].key.key, s("entry_2"));
    assert_eq!(list.entries[0].value.value, s("value_2"));
    assert_eq!(list.entries[1].key.key, s("entry_3"));
    assert_eq!(list.entries[1].value.value, s("value_3"));
    assert_eq!(list.entries[2].key.key, s("entry_4"));
    assert_eq!(list.entries[2].value.value, s("value_4"));
    assert_eq!(list.entries[3].key.key, s("entry_5"));
    assert_eq!(list.entries[3].value.value, s("value_5"));
}

#[test]
fn test_remove_entry_nonexistent() {
    let mut bt = BTree::new_btree();
    bt.add_entry(s("abc"), 4, s("val"), 4);
    bt.remove_entry(&s("xyz"), 4);
    assert_eq!(bt.get_entry_count(), 1);
    let v = bt.find_entry(&s("abc"), 4).unwrap();
    assert_eq!(v.len, 4);
    assert_eq!(v.value, s("val"));
}

#[test]
fn test_remove_entry_only_node() {
    let mut bt = BTree::new_btree();
    bt.add_entry(s("abc"), 4, s("val"), 4);
    bt.remove_entry(&s("abc"), 4);
    assert_eq!(bt.get_entry_count(), 0);
    let v = bt.find_entry(&s("abc"), 4);
    assert!(v.is_none());
}

#[test]
fn test_remove_entry_two_children() {
    // mmm hash=654, aaa hash=582 (left child of mmm), zzz hash=732 (right child)
    let mut bt = BTree::new_btree();
    bt.add_entry(b"mmm".to_vec(), 3, b"M\0".to_vec(), 2);
    bt.add_entry(b"aaa".to_vec(), 3, b"A\0".to_vec(), 2);
    bt.add_entry(b"zzz".to_vec(), 3, b"Z\0".to_vec(), 2);
    assert_eq!(bt.get_entry_count(), 3);
    bt.remove_entry(&b"mmm".to_vec(), 3);
    assert_eq!(bt.get_entry_count(), 2);

    let list = bt.list_entries();
    assert_eq!(list.len, 2);
    assert_eq!(list.cap, 2);
    // After removal, inorder traversal yields aaa then zzz
    assert_eq!(list.entries[0].key.len, 3);
    assert_eq!(list.entries[0].key.key, b"aaa".to_vec());
    assert_eq!(list.entries[0].value.len, 2);
    assert_eq!(list.entries[0].value.value, b"A\0".to_vec());
    assert_eq!(list.entries[1].key.len, 3);
    assert_eq!(list.entries[1].key.key, b"zzz".to_vec());
    assert_eq!(list.entries[1].value.len, 2);
    assert_eq!(list.entries[1].value.value, b"Z\0".to_vec());
}

// ---------- Tests for BTree::free_tree ----------

#[test]
fn test_free_tree_clears() {
    let mut bt = BTree::new_btree();
    bt.add_entry(s("a"), 2, s("b"), 2);
    bt.free_tree();
    assert_eq!(bt.get_entry_count(), 0);
    assert!(bt.node.is_none());
}

// ---------- Tests for Node::new_node ----------

#[test]
fn test_node_new_node_short_key() {
    let n = Node::new_node(s("k"), 2, s("v"), 2);
    assert_eq!(n.key_len, 2);
    // First two bytes should be 'k', '\0'; rest zeroed.
    assert_eq!(n.p_key[0], b'k');
    assert_eq!(n.p_key[1], 0);
    for i in 2..BTREE_KEY_SIZE {
        assert_eq!(n.p_key[i], 0);
    }
    assert_eq!(n.value.len, 2);
    assert_eq!(n.value.value, s("v"));
    // hash is calc_key_hash over the truncated p_key[..key_len]
    // 'k'*1 + 0*2 = 107
    assert_eq!(n.key_hash, 107);
    assert!(n.child_left.is_none());
    assert!(n.child_right.is_none());
}

#[test]
fn test_node_new_node_truncates_key() {
    let long_key: Vec<u8> = b"abcdefghijklmnop\0".to_vec();
    let n = Node::new_node(long_key.clone(), long_key.len(), s("v"), 2);
    assert_eq!(n.key_len, BTREE_KEY_SIZE);
    assert_eq!(&n.p_key[..], b"abcdefghij");
    // hash over first 10 bytes 'a'..'j' = 97..106
    let expected: u32 = (97u32 * 1)
        + (98 * 2)
        + (99 * 3)
        + (100 * 4)
        + (101 * 5)
        + (102 * 6)
        + (103 * 7)
        + (104 * 8)
        + (105 * 9)
        + (106 * 10);
    assert_eq!(n.key_hash, expected);
}

// ---------- Tests for Node::get_node_count ----------

#[test]
fn test_node_get_node_count_single() {
    let n = Node::new_node(s("k"), 2, s("v"), 2);
    assert_eq!(n.get_node_count(), 1);
}

#[test]
fn test_node_get_node_count_via_btree() {
    let mut bt = BTree::new_btree();
    bt.add_entry(s("entry_1"), 8, s("v"), 2);
    bt.add_entry(s("entry_2"), 8, s("v"), 2);
    bt.add_entry(s("entry_3"), 8, s("v"), 2);
    let root = bt.node.as_ref().unwrap();
    assert_eq!(root.get_node_count(), 3);
}

// ---------- Tests for Node::find_value ----------

#[test]
fn test_node_find_value_hit() {
    let mut bt = BTree::new_btree();
    bt.add_entry(s("entry_1"), 8, s("value_1"), 8);
    bt.add_entry(s("entry_2"), 8, s("value_2"), 8);
    let root = bt.node.as_ref().unwrap();
    let v = root.find_value(2650, s("entry_2"), 8);
    assert!(v.is_some());
    let v = v.unwrap();
    assert_eq!(v.len, 8);
    assert_eq!(v.value, s("value_2"));
}

#[test]
fn test_node_find_value_miss() {
    let mut bt = BTree::new_btree();
    bt.add_entry(s("entry_1"), 8, s("value_1"), 8);
    let root = bt.node.as_ref().unwrap();
    // wrong hash
    let v = root.find_value(99999, s("anything"), 9);
    assert!(v.is_none());
}

// ---------- Tests for Node::list_node_entries ----------

#[test]
fn test_node_list_node_entries_via_root() {
    let mut bt = BTree::new_btree();
    bt.add_entry(s("entry_1"), 8, s("value_1"), 8);
    bt.add_entry(s("entry_2"), 8, s("value_2"), 8);
    let cap = bt.get_entry_count();
    let mut list = btree_map::btree::EntryList {
        entries: Vec::with_capacity(cap),
        len: 0,
        cap,
    };
    bt.node.as_ref().unwrap().list_node_entries(&mut list);
    assert_eq!(list.len, 2);
    assert_eq!(list.cap, 2);
    assert_eq!(list.entries[0].key.key, s("entry_1"));
    assert_eq!(list.entries[0].value.value, s("value_1"));
    assert_eq!(list.entries[1].key.key, s("entry_2"));
    assert_eq!(list.entries[1].value.value, s("value_2"));
}

// ---------- Tests for various key types matching C ----------

#[test]
fn test_int_key() {
    let mut bt = BTree::new_btree();
    let int_key: u32 = 1;
    let key_bytes = int_key.to_le_bytes().to_vec();
    bt.add_entry(key_bytes.clone(), 4, s("value_2"), 8);
    let v = bt.find_entry(&key_bytes, 4).unwrap();
    assert_eq!(v.len, 8);
    assert_eq!(v.value, s("value_2"));
}

#[test]
fn test_byte_key() {
    let mut bt = BTree::new_btree();
    let byte_key: Vec<u8> = vec![9];
    bt.add_entry(byte_key.clone(), 1, s("value_4"), 8);
    let v = bt.find_entry(&byte_key, 1).unwrap();
    assert_eq!(v.len, 8);
    assert_eq!(v.value, s("value_4"));
}

#[test]
fn test_custom_struct_key() {
    // Replicates C struct {uint32_t key=1, uint32_t key2=2} packed little-endian
    let mut bytes = vec![];
    bytes.extend_from_slice(&1u32.to_le_bytes());
    bytes.extend_from_slice(&2u32.to_le_bytes());
    let mut bt = BTree::new_btree();
    bt.add_entry(bytes.clone(), 8, s("value_1"), 8);
    let v = bt.find_entry(&bytes, 8).unwrap();
    assert_eq!(v.len, 8);
    assert_eq!(v.value, s("value_1"));
}

fn main() {}
