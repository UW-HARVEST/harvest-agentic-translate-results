use btree_map::btree::{
    btree_free, btree_malloc, calc_key_hash, min_size, BTree, BTreeKey, Entry, EntryList, Node,
    Value, BTREE_KEY_SIZE,
};

// -------------- helper functions --------------

/// Build a Vec<u8> equivalent to the C string literal including its null
/// terminator. e.g. "entry_1" -> 8 bytes "entry_1\0".
fn cstr(s: &str) -> Vec<u8> {
    let mut v = s.as_bytes().to_vec();
    v.push(0);
    v
}

// -------------- BTREE_KEY_SIZE --------------

#[test]
fn test_btree_key_size_constant() {
    assert_eq!(BTREE_KEY_SIZE, 10);
}

// -------------- min_size --------------

#[test]
fn test_min_size_a_smaller() {
    assert_eq!(min_size(3, 5), 3);
}

#[test]
fn test_min_size_b_smaller() {
    assert_eq!(min_size(7, 2), 2);
}

#[test]
fn test_min_size_equal() {
    assert_eq!(min_size(4, 4), 4);
}

#[test]
fn test_min_size_zero_a() {
    assert_eq!(min_size(0, 5), 0);
}

#[test]
fn test_min_size_zero_b() {
    assert_eq!(min_size(5, 0), 0);
}

// -------------- calc_key_hash --------------
// Reference values obtained from running the C calc_key_hash:
//   "entry_1\0" (len 8) -> 2643
//   "entry_2\0" (len 8) -> 2650
//   "entry_3\0" (len 8) -> 2657
//   "entry_4\0" (len 8) -> 2664
//   "entry_5\0" (len 8) -> 2671
//   "key_1\0"   (len 6) -> 1297
//   "hello\0"   (len 6) -> 1617
//   "hello"     (len 5) -> 1617
//   ""          (len 0) -> 0
//   [9]         (len 1) -> 9
//   [1,0,0,0]   (len 4) -> 1
//   [10,0,0,0,0,0,0,0] (len 8) -> 10
//   "0123456789ABC\0" (len 14) -> 5348
//   "0123456789ABC\0" first 10 (len 10) -> 2970

#[test]
fn test_calc_key_hash_entry_1() {
    let k = cstr("entry_1");
    assert_eq!(calc_key_hash(&k, k.len()), 2643);
}

#[test]
fn test_calc_key_hash_entry_2() {
    let k = cstr("entry_2");
    assert_eq!(calc_key_hash(&k, k.len()), 2650);
}

#[test]
fn test_calc_key_hash_entry_3() {
    let k = cstr("entry_3");
    assert_eq!(calc_key_hash(&k, k.len()), 2657);
}

#[test]
fn test_calc_key_hash_entry_4() {
    let k = cstr("entry_4");
    assert_eq!(calc_key_hash(&k, k.len()), 2664);
}

#[test]
fn test_calc_key_hash_entry_5() {
    let k = cstr("entry_5");
    assert_eq!(calc_key_hash(&k, k.len()), 2671);
}

#[test]
fn test_calc_key_hash_key_1() {
    let k = cstr("key_1");
    assert_eq!(calc_key_hash(&k, k.len()), 1297);
}

#[test]
fn test_calc_key_hash_hello_with_null() {
    let k = cstr("hello");
    assert_eq!(calc_key_hash(&k, k.len()), 1617);
}

#[test]
fn test_calc_key_hash_hello_without_null() {
    let k = b"hello".to_vec();
    assert_eq!(calc_key_hash(&k, 5), 1617);
}

#[test]
fn test_calc_key_hash_empty_zero_len() {
    let k = vec![];
    assert_eq!(calc_key_hash(&k, 0), 0);
}

#[test]
fn test_calc_key_hash_single_byte() {
    let k = vec![9u8];
    assert_eq!(calc_key_hash(&k, 1), 9);
}

#[test]
fn test_calc_key_hash_int_le() {
    // uint32_t = 1 in little-endian = [1, 0, 0, 0]
    let k = vec![1u8, 0, 0, 0];
    assert_eq!(calc_key_hash(&k, 4), 1);
}

#[test]
fn test_calc_key_hash_long_le() {
    // uint64_t = 10 in little-endian
    let k = vec![10u8, 0, 0, 0, 0, 0, 0, 0];
    assert_eq!(calc_key_hash(&k, 8), 10);
}

#[test]
fn test_calc_key_hash_long_key_full() {
    let k = cstr("0123456789ABC"); // 14 bytes
    assert_eq!(calc_key_hash(&k, k.len()), 5348);
}

#[test]
fn test_calc_key_hash_long_key_truncated_at_10() {
    let k = cstr("0123456789ABC"); // 14 bytes
    assert_eq!(calc_key_hash(&k, 10), 2970);
}

// -------------- BTree::new_btree --------------

#[test]
fn test_new_btree_is_empty() {
    let bt = BTree::new_btree();
    assert!(bt.node.is_none());
    assert_eq!(bt.get_entry_count(), 0);
}

// -------------- BTree::add_entry / find_entry --------------

#[test]
fn test_add_then_find_entry_basic() {
    let mut bt = BTree::new_btree();
    let key = cstr("entry_1");
    let val = cstr("value_1");
    bt.add_entry(key.clone(), key.len(), val.clone(), val.len());

    let v = bt.find_entry(&key, key.len()).expect("entry must exist");
    assert_eq!(v.len, val.len()); // 8
    assert_eq!(v.value.len(), val.len());
    assert_eq!(v.value, val);
}

#[test]
fn test_find_entry_missing_returns_none() {
    let mut bt = BTree::new_btree();
    let key = cstr("x");
    bt.add_entry(key.clone(), key.len(), cstr("X"), cstr("X").len());
    let other = cstr("y");
    let v = bt.find_entry(&other, other.len());
    assert!(v.is_none());
}

#[test]
fn test_find_entry_on_empty_tree_returns_none() {
    let bt = BTree::new_btree();
    let key = cstr("any");
    assert!(bt.find_entry(&key, key.len()).is_none());
}

#[test]
fn test_get_entry_count_empty() {
    let bt = BTree::new_btree();
    assert_eq!(bt.get_entry_count(), 0);
}

#[test]
fn test_get_entry_count_increments() {
    let mut bt = BTree::new_btree();
    let k1 = cstr("entry_1");
    bt.add_entry(k1.clone(), k1.len(), cstr("v1"), cstr("v1").len());
    assert_eq!(bt.get_entry_count(), 1);
    let k2 = cstr("entry_2");
    bt.add_entry(k2.clone(), k2.len(), cstr("v2"), cstr("v2").len());
    assert_eq!(bt.get_entry_count(), 2);
    let k3 = cstr("entry_3");
    bt.add_entry(k3.clone(), k3.len(), cstr("v3"), cstr("v3").len());
    assert_eq!(bt.get_entry_count(), 3);
}

// -------------- BTree::list_entries --------------

#[test]
fn test_list_entries_five_in_order() {
    // Mirrors test_entry_list in c_src/tests/test.c
    let mut bt = BTree::new_btree();
    let pairs = [
        ("entry_1", "value_1"),
        ("entry_2", "value_2"),
        ("entry_3", "value_3"),
        ("entry_4", "value_4"),
        ("entry_5", "value_5"),
    ];
    for (k, v) in &pairs {
        let kb = cstr(k);
        let vb = cstr(v);
        bt.add_entry(kb.clone(), kb.len(), vb.clone(), vb.len());
    }
    let list = bt.list_entries();
    assert_eq!(list.len, 5);
    assert_eq!(list.cap, 5);
    assert_eq!(list.entries.len(), 5);
    for (i, (k, v)) in pairs.iter().enumerate() {
        let kb = cstr(k);
        let vb = cstr(v);
        // key
        assert_eq!(list.entries[i].key.len, kb.len());
        assert_eq!(list.entries[i].key.key.len(), kb.len());
        assert_eq!(list.entries[i].key.key, kb);
        // value
        assert_eq!(list.entries[i].value.len, vb.len());
        assert_eq!(list.entries[i].value.value.len(), vb.len());
        assert_eq!(list.entries[i].value.value, vb);
    }
}

#[test]
fn test_list_entries_empty_tree() {
    let bt = BTree::new_btree();
    let list = bt.list_entries();
    assert_eq!(list.len, 0);
    assert_eq!(list.cap, 0);
    assert_eq!(list.entries.len(), 0);
}

// -------------- BTree::remove_entry --------------

#[test]
fn test_remove_entry_middle() {
    // Mirrors test_remove_entry in c_src/tests/test.c
    let mut bt = BTree::new_btree();
    let pairs = [
        ("entry_1", "value_1"),
        ("entry_2", "value_2"),
        ("entry_3", "value_3"),
        ("entry_4", "value_4"),
        ("entry_5", "value_5"),
    ];
    for (k, v) in &pairs {
        let kb = cstr(k);
        let vb = cstr(v);
        bt.add_entry(kb.clone(), kb.len(), vb.clone(), vb.len());
    }
    let key3 = cstr("entry_3");
    bt.remove_entry(&key3, key3.len());
    let list = bt.list_entries();
    assert_eq!(list.len, 4);
    assert_eq!(list.cap, 4);
    let expected = [
        ("entry_1", "value_1"),
        ("entry_2", "value_2"),
        ("entry_4", "value_4"),
        ("entry_5", "value_5"),
    ];
    for (i, (k, v)) in expected.iter().enumerate() {
        let kb = cstr(k);
        let vb = cstr(v);
        assert_eq!(list.entries[i].key.key, kb);
        assert_eq!(list.entries[i].key.len, kb.len());
        assert_eq!(list.entries[i].value.value, vb);
        assert_eq!(list.entries[i].value.len, vb.len());
    }
}

#[test]
fn test_remove_nonexistent_key_no_change() {
    let mut bt = BTree::new_btree();
    for i in 1..=3 {
        let kb = cstr(&format!("entry_{}", i));
        let vb = cstr(&format!("value_{}", i));
        bt.add_entry(kb.clone(), kb.len(), vb.clone(), vb.len());
    }
    assert_eq!(bt.get_entry_count(), 3);
    let zzz = cstr("zzz");
    bt.remove_entry(&zzz, zzz.len());
    assert_eq!(bt.get_entry_count(), 3);
}

#[test]
fn test_remove_root_only_node() {
    let mut bt = BTree::new_btree();
    let k = cstr("only");
    let v = cstr("v");
    bt.add_entry(k.clone(), k.len(), v.clone(), v.len());
    assert_eq!(bt.get_entry_count(), 1);
    bt.remove_entry(&k, k.len());
    assert_eq!(bt.get_entry_count(), 0);
    assert!(bt.find_entry(&k, k.len()).is_none());
    assert!(bt.node.is_none());
}

#[test]
fn test_remove_then_readd_entry() {
    let mut bt = BTree::new_btree();
    let k = cstr("entry_1");
    let v1 = cstr("value_1");
    bt.add_entry(k.clone(), k.len(), v1.clone(), v1.len());
    bt.remove_entry(&k, k.len());
    assert!(bt.find_entry(&k, k.len()).is_none());
    let v2 = cstr("value_2");
    bt.add_entry(k.clone(), k.len(), v2.clone(), v2.len());
    let found = bt.find_entry(&k, k.len()).expect("must exist");
    assert_eq!(found.value, v2);
    assert_eq!(found.len, v2.len());
}

// -------------- update existing key --------------

#[test]
fn test_update_existing_key_same_size() {
    // hash for "alpha\0" is 7*1+? but we don't depend on hash values directly.
    // Re-adding with the same key updates the value (same size avoids C UB).
    let mut bt = BTree::new_btree();
    let k = cstr("entry_1");
    let v_old = cstr("value_1"); // 8 bytes
    let v_new = cstr("VALUE_X"); // 8 bytes
    assert_eq!(v_old.len(), v_new.len());

    bt.add_entry(k.clone(), k.len(), v_old.clone(), v_old.len());
    bt.add_entry(k.clone(), k.len(), v_new.clone(), v_new.len());
    assert_eq!(bt.get_entry_count(), 1); // still one entry

    let found = bt.find_entry(&k, k.len()).expect("must exist");
    assert_eq!(found.len, v_new.len());
    assert_eq!(found.value, v_new);
}

// -------------- multiple key types --------------

#[test]
fn test_multiple_key_types() {
    // Mirrors test_multiple_key_types in c_src/tests/test.c
    let mut bt = BTree::new_btree();

    let k_str = cstr("entry_1");
    let v1 = cstr("value_1");
    bt.add_entry(k_str.clone(), k_str.len(), v1.clone(), v1.len());

    // uint32_t int_key = 1; little-endian
    let k_int = vec![1u8, 0, 0, 0];
    let v2 = cstr("value_2");
    bt.add_entry(k_int.clone(), k_int.len(), v2.clone(), v2.len());

    // uint64_t long_key = 10; little-endian
    let k_long = vec![10u8, 0, 0, 0, 0, 0, 0, 0];
    let v3 = cstr("value_3");
    bt.add_entry(k_long.clone(), k_long.len(), v3.clone(), v3.len());

    // uint8_t byte_key = 9
    let k_byte = vec![9u8];
    let v4 = cstr("value_4");
    bt.add_entry(k_byte.clone(), k_byte.len(), v4.clone(), v4.len());

    let f = bt.find_entry(&k_str, k_str.len()).expect("string key");
    assert_eq!(f.len, v1.len());
    assert_eq!(f.value, v1);

    let f = bt.find_entry(&k_int, k_int.len()).expect("int key");
    assert_eq!(f.len, v2.len());
    assert_eq!(f.value, v2);

    let f = bt.find_entry(&k_long, k_long.len()).expect("long key");
    assert_eq!(f.len, v3.len());
    assert_eq!(f.value, v3);

    let f = bt.find_entry(&k_byte, k_byte.len()).expect("byte key");
    assert_eq!(f.len, v4.len());
    assert_eq!(f.value, v4);
}

// -------------- custom struct as key/value --------------

#[test]
fn test_custom_struct_key() {
    // C struct { uint32_t key; uint32_t key2; } = {1, 2}
    // little-endian: [1,0,0,0,2,0,0,0] = 8 bytes
    let key_bytes = vec![1u8, 0, 0, 0, 2, 0, 0, 0];
    let mut bt = BTree::new_btree();
    let v = cstr("value_1"); // 8 bytes
    bt.add_entry(key_bytes.clone(), key_bytes.len(), v.clone(), v.len());
    let f = bt
        .find_entry(&key_bytes, key_bytes.len())
        .expect("custom struct key");
    assert_eq!(f.len, v.len());
    assert_eq!(f.value, v);
}

#[test]
fn test_custom_struct_value() {
    // C struct { uint32_t value; uint32_t value2; } = {1, 2}
    let value_bytes = vec![1u8, 0, 0, 0, 2, 0, 0, 0];
    let key = cstr("key_1"); // 6 bytes
    let mut bt = BTree::new_btree();
    bt.add_entry(
        key.clone(),
        key.len(),
        value_bytes.clone(),
        value_bytes.len(),
    );
    let f = bt.find_entry(&key, key.len()).expect("custom struct value");
    assert_eq!(f.len, value_bytes.len());
    assert_eq!(f.value, value_bytes);
}

// -------------- long-key truncation behavior --------------

#[test]
fn test_long_key_truncated_keys_collide() {
    // Mirrors C test in our reference: a key longer than BTREE_KEY_SIZE only
    // stores its first 10 bytes, so two keys that share the first 10 bytes
    // are indistinguishable.
    let mut bt = BTree::new_btree();
    let k1 = cstr("0123456789ABC"); // 14
    let v1 = cstr("VLONG"); // 6
    bt.add_entry(k1.clone(), k1.len(), v1.clone(), v1.len());

    // Same first 10 chars, different suffix.
    let k2 = cstr("0123456789XYZ"); // 14
    let f = bt
        .find_entry(&k2, k2.len())
        .expect("truncated keys should collide");
    assert_eq!(f.len, v1.len());
    assert_eq!(f.value, v1);
}

// -------------- free_tree --------------

#[test]
fn test_free_tree_clears_root() {
    let mut bt = BTree::new_btree();
    let k = cstr("entry_1");
    let v = cstr("value_1");
    bt.add_entry(k.clone(), k.len(), v.clone(), v.len());
    assert_eq!(bt.get_entry_count(), 1);
    bt.free_tree();
    assert!(bt.node.is_none());
    assert_eq!(bt.get_entry_count(), 0);
    assert!(bt.find_entry(&k, k.len()).is_none());
    let list = bt.list_entries();
    assert_eq!(list.len, 0);
    assert_eq!(list.cap, 0);
}

// -------------- Node::new_node --------------

#[test]
fn test_node_new_node_basic() {
    let key = cstr("entry_1"); // 8 bytes
    let value = cstr("value_1"); // 8 bytes
    let n = Node::new_node(key.clone(), key.len(), value.clone(), value.len());
    assert_eq!(n.key_len, 8);
    // p_key bytes 0..8 should equal key
    assert_eq!(&n.p_key[..8], &key[..]);
    // bytes 8..10 are zero
    assert_eq!(n.p_key[8], 0);
    assert_eq!(n.p_key[9], 0);
    assert_eq!(n.value.len, 8);
    assert_eq!(n.value.value, value);
    assert_eq!(n.key_hash, 2643);
    assert!(n.child_left.is_none());
    assert!(n.child_right.is_none());
}

#[test]
fn test_node_new_node_truncates_long_key() {
    let key = cstr("0123456789ABC"); // 14 bytes
    let value = cstr("v"); // 2 bytes
    let n = Node::new_node(key.clone(), key.len(), value.clone(), value.len());
    assert_eq!(n.key_len, BTREE_KEY_SIZE);
    assert_eq!(&n.p_key[..BTREE_KEY_SIZE], &key[..BTREE_KEY_SIZE]);
    assert_eq!(n.key_hash, 2970);
    assert_eq!(n.value.len, 2);
    assert_eq!(n.value.value, value);
}

// -------------- Node::add_node --------------

#[test]
fn test_node_add_node_right_child_higher_hash() {
    let mut root = Node::new_node(cstr("entry_1"), 8, cstr("v1"), 3);
    // entry_1 hash = 2643
    let root_mut = std::sync::Arc::get_mut(&mut root).unwrap();
    let n2 = Node::new_node(cstr("entry_2"), 8, cstr("v2"), 3);
    // entry_2 hash = 2650 > 2643
    root_mut.add_node(n2);
    assert!(root_mut.child_right.is_some());
    assert!(root_mut.child_left.is_none());
    let r = root_mut.child_right.as_ref().unwrap();
    assert_eq!(r.key_hash, 2650);
    assert_eq!(&r.p_key[..8], &cstr("entry_2")[..]);
}

#[test]
fn test_node_add_node_left_child_lower_hash() {
    // Use values where new node has *lower* hash than root.
    // We need root hash > new node hash. Construct manually.
    // entry_5 hash 2671, entry_1 hash 2643 => add entry_1 to entry_5 root.
    let mut root = Node::new_node(cstr("entry_5"), 8, cstr("v5"), 3);
    let root_mut = std::sync::Arc::get_mut(&mut root).unwrap();
    let n1 = Node::new_node(cstr("entry_1"), 8, cstr("v1"), 3);
    root_mut.add_node(n1);
    assert!(root_mut.child_left.is_some());
    assert!(root_mut.child_right.is_none());
    let l = root_mut.child_left.as_ref().unwrap();
    assert_eq!(l.key_hash, 2643);
}

#[test]
fn test_node_add_node_replaces_value_on_same_key() {
    let mut root = Node::new_node(cstr("entry_1"), 8, cstr("value_1"), 8);
    let root_mut = std::sync::Arc::get_mut(&mut root).unwrap();
    let dup = Node::new_node(cstr("entry_1"), 8, cstr("VALUE_X"), 8);
    root_mut.add_node(dup);
    // Tree shape unchanged
    assert!(root_mut.child_left.is_none());
    assert!(root_mut.child_right.is_none());
    assert_eq!(root_mut.value.len, 8);
    assert_eq!(root_mut.value.value, cstr("VALUE_X"));
}

// -------------- Node::find_value --------------

#[test]
fn test_node_find_value_direct_root() {
    let root = Node::new_node(cstr("entry_1"), 8, cstr("value_1"), 8);
    let key = cstr("entry_1");
    let h = calc_key_hash(&key, key.len());
    let v = root.find_value(h, key.clone(), key.len()).expect("found");
    assert_eq!(v.len, 8);
    assert_eq!(v.value, cstr("value_1"));
}

#[test]
fn test_node_find_value_missing() {
    let root = Node::new_node(cstr("entry_1"), 8, cstr("value_1"), 8);
    let key = cstr("zzzz_zz");
    let h = calc_key_hash(&key, key.len());
    let v = root.find_value(h, key.clone(), key.len());
    assert!(v.is_none());
}

// -------------- Node::get_node_count --------------

#[test]
fn test_node_get_node_count_single() {
    let root = Node::new_node(cstr("k"), 2, cstr("v"), 2);
    assert_eq!(root.get_node_count(), 1);
}

#[test]
fn test_node_get_node_count_with_children() {
    // Build a tree manually using add_node.
    let mut root = Node::new_node(cstr("entry_3"), 8, cstr("v3"), 3);
    {
        let r = std::sync::Arc::get_mut(&mut root).unwrap();
        r.add_node(Node::new_node(cstr("entry_1"), 8, cstr("v1"), 3));
        r.add_node(Node::new_node(cstr("entry_5"), 8, cstr("v5"), 3));
    }
    assert_eq!(root.get_node_count(), 3);
}

// -------------- Node::list_node_entries --------------

#[test]
fn test_node_list_node_entries_in_order() {
    // Build root=entry_3, left=entry_1, right=entry_5 (hashes 2643<2657<2671)
    let mut root = Node::new_node(cstr("entry_3"), 8, cstr("value_3"), 8);
    {
        let r = std::sync::Arc::get_mut(&mut root).unwrap();
        r.add_node(Node::new_node(cstr("entry_1"), 8, cstr("value_1"), 8));
        r.add_node(Node::new_node(cstr("entry_5"), 8, cstr("value_5"), 8));
    }
    let mut list = EntryList {
        entries: Vec::with_capacity(3),
        len: 0,
        cap: 3,
    };
    root.list_node_entries(&mut list);
    assert_eq!(list.len, 3);
    assert_eq!(list.cap, 3);
    assert_eq!(list.entries.len(), 3);
    let expected = [("entry_1", "value_1"), ("entry_3", "value_3"), ("entry_5", "value_5")];
    for (i, (k, v)) in expected.iter().enumerate() {
        assert_eq!(list.entries[i].key.key, cstr(k));
        assert_eq!(list.entries[i].key.len, cstr(k).len());
        assert_eq!(list.entries[i].value.value, cstr(v));
        assert_eq!(list.entries[i].value.len, cstr(v).len());
    }
}

#[test]
fn test_node_list_node_entries_respects_cap() {
    let mut root = Node::new_node(cstr("entry_3"), 8, cstr("value_3"), 8);
    {
        let r = std::sync::Arc::get_mut(&mut root).unwrap();
        r.add_node(Node::new_node(cstr("entry_1"), 8, cstr("value_1"), 8));
        r.add_node(Node::new_node(cstr("entry_5"), 8, cstr("value_5"), 8));
    }
    let mut list = EntryList {
        entries: Vec::with_capacity(2),
        len: 0,
        cap: 2,
    };
    root.list_node_entries(&mut list);
    // Only first two in-order entries (entry_1, entry_3) recorded.
    assert_eq!(list.len, 2);
    assert_eq!(list.cap, 2);
    assert_eq!(list.entries[0].key.key, cstr("entry_1"));
    assert_eq!(list.entries[1].key.key, cstr("entry_3"));
}

// -------------- Node::free_node --------------

#[test]
fn test_node_free_node_clears_children() {
    let mut root = Node::new_node(cstr("entry_3"), 8, cstr("v3"), 3);
    {
        let r = std::sync::Arc::get_mut(&mut root).unwrap();
        r.add_node(Node::new_node(cstr("entry_1"), 8, cstr("v1"), 3));
        r.add_node(Node::new_node(cstr("entry_5"), 8, cstr("v5"), 3));
    }
    let r = std::sync::Arc::get_mut(&mut root).unwrap();
    assert!(r.child_left.is_some());
    assert!(r.child_right.is_some());
    r.free_node();
    assert!(r.child_left.is_none());
    assert!(r.child_right.is_none());
}

// -------------- Node::delete_node --------------

#[test]
fn test_node_delete_node_two_children() {
    // Build tree: root=entry_3, left=entry_1, right=entry_5
    let mut root = Node::new_node(cstr("entry_3"), 8, cstr("value_3"), 8);
    {
        let r = std::sync::Arc::get_mut(&mut root).unwrap();
        r.add_node(Node::new_node(cstr("entry_1"), 8, cstr("value_1"), 8));
        r.add_node(Node::new_node(cstr("entry_5"), 8, cstr("value_5"), 8));
    }
    let key3 = cstr("entry_3");
    let h3 = calc_key_hash(&key3, key3.len());
    let new_root = Node::delete_node(&mut root, h3, key3.clone(), key3.len()).unwrap();
    // After delete, root should be replaced by inorder successor (entry_5).
    assert_eq!(new_root.key_hash, 2671);
    assert_eq!(&new_root.p_key[..8], &cstr("entry_5")[..]);
    // Left child entry_1 still present, right child gone (was successor).
    assert!(new_root.child_left.is_some());
    assert!(new_root.child_right.is_none());
    let l = new_root.child_left.as_ref().unwrap();
    assert_eq!(l.key_hash, 2643);
}

#[test]
fn test_node_delete_node_leaf() {
    // Root=entry_3, right=entry_5 (no left).
    let mut root = Node::new_node(cstr("entry_3"), 8, cstr("value_3"), 8);
    {
        let r = std::sync::Arc::get_mut(&mut root).unwrap();
        r.add_node(Node::new_node(cstr("entry_5"), 8, cstr("value_5"), 8));
    }
    let key5 = cstr("entry_5");
    let h5 = calc_key_hash(&key5, key5.len());
    let new_root = Node::delete_node(&mut root, h5, key5.clone(), key5.len()).unwrap();
    // Root unchanged; right child cleared.
    assert_eq!(new_root.key_hash, 2657);
    assert!(new_root.child_right.is_none());
    assert!(new_root.child_left.is_none());
}

#[test]
fn test_node_delete_node_root_no_children() {
    let mut root = Node::new_node(cstr("only"), 5, cstr("v"), 2);
    let key = cstr("only");
    let h = calc_key_hash(&key, key.len());
    let result = Node::delete_node(&mut root, h, key.clone(), key.len());
    // No children -> Some(child_right.take()) which is None.
    assert!(result.is_none());
}

// -------------- struct construction and pub field access --------------

#[test]
fn test_struct_field_access() {
    let bk = BTreeKey {
        key: vec![1, 2, 3],
        len: 3,
    };
    assert_eq!(bk.key, vec![1u8, 2, 3]);
    assert_eq!(bk.len, 3);

    let v = Value {
        value: vec![4, 5],
        len: 2,
    };
    assert_eq!(v.value, vec![4u8, 5]);
    assert_eq!(v.len, 2);

    let e = Entry {
        key: BTreeKey {
            key: vec![7],
            len: 1,
        },
        value: Value {
            value: vec![8],
            len: 1,
        },
    };
    assert_eq!(e.key.len, 1);
    assert_eq!(e.value.len, 1);

    let el = EntryList {
        entries: vec![],
        len: 0,
        cap: 0,
    };
    assert_eq!(el.len, 0);
    assert_eq!(el.cap, 0);
    assert_eq!(el.entries.len(), 0);
}

// -------------- btree_malloc / btree_free (parity stubs) --------------

#[test]
fn test_btree_malloc_returns_zeroed_value() {
    // The function exists for API parity. It returns a zeroed value.
    let v: u32 = btree_malloc::<u32>(4);
    assert_eq!(v, 0);
    let v: u64 = btree_malloc::<u64>(8);
    assert_eq!(v, 0);
}

#[test]
fn test_btree_free_no_op() {
    // No-op on Rust side; just verify it can be called.
    let x: u32 = 42;
    btree_free(&x);
    // Value still usable since it's a no-op.
    assert_eq!(x, 42);
}

fn main() {}
