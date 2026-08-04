use hamta::hamta::*;

// Test methods on `HamtNode` directly.

#[test]
fn test_is_leaf_for_empty_leaf() {
    let mut node: HamtNode<i32, i32> = HamtNode::Leaf(None);
    assert!(node.is_leaf());
}

#[test]
fn test_is_leaf_for_populated_leaf() {
    let mut k = 1i32;
    let mut v = 10i32;
    let mut node: HamtNode<i32, i32> = HamtNode::Leaf(Some(KeyValue {
        key: &mut k,
        value: &mut v,
    }));
    assert!(node.is_leaf());
}

#[test]
fn test_is_leaf_for_sub() {
    let mut node: HamtNode<i32, i32> = HamtNode::Sub(SubNode {
        bitmap: 0,
        children: None,
    });
    assert!(!node.is_leaf());
}

#[test]
fn test_get_children_pointer_returns_leaf_none() {
    // The implementation returns Leaf(None) as a placeholder.
    let mut node: HamtNode<i32, i32> = HamtNode::Sub(SubNode {
        bitmap: 0,
        children: None,
    });
    let mut returned = node.get_children_pointer();
    assert!(returned.is_leaf());
    if let HamtNode::Leaf(opt) = returned {
        assert!(opt.is_none());
    } else {
        panic!("expected Leaf");
    }
}

#[test]
fn test_node_search_on_matching_leaf() {
    let mut k = 5i32;
    let mut v = 50i32;
    let mut node: HamtNode<i32, i32> = HamtNode::Leaf(Some(KeyValue {
        key: &mut k,
        value: &mut v,
    }));
    let mut search_key = 5i32;
    let result = node.hamt_node_search(0, 0, &mut search_key, hamt_int_equals);
    // Result should be a Leaf with Some(matched kv)
    if let HamtNode::Leaf(opt) = result {
        let kv = opt.expect("expected matching kv");
        assert_eq!(*kv.key, 5);
        assert_eq!(*kv.value, 50);
    } else {
        panic!("expected Leaf(Some(_))");
    }
}

#[test]
fn test_node_search_on_non_matching_leaf() {
    let mut k = 5i32;
    let mut v = 50i32;
    let mut node: HamtNode<i32, i32> = HamtNode::Leaf(Some(KeyValue {
        key: &mut k,
        value: &mut v,
    }));
    let mut search_key = 99i32;
    let result = node.hamt_node_search(0, 0, &mut search_key, hamt_int_equals);
    if let HamtNode::Leaf(opt) = result {
        assert!(opt.is_none());
    } else {
        panic!("expected Leaf(None)");
    }
}

#[test]
fn test_node_search_empty_leaf() {
    let mut node: HamtNode<i32, i32> = HamtNode::Leaf(None);
    let mut search_key = 0i32;
    let result = node.hamt_node_search(0, 0, &mut search_key, hamt_int_equals);
    if let HamtNode::Leaf(opt) = result {
        assert!(opt.is_none());
    } else {
        panic!("expected Leaf(None)");
    }
}

#[test]
fn test_node_insert_into_empty_leaf() {
    let mut node: HamtNode<i32, i32> = HamtNode::Leaf(None);
    let mut k = 1i32;
    let mut v = 10i32;
    let mut ck = 0i32;
    let mut cv = 0i32;
    let mut conflict_kv = KeyValue { key: &mut ck, value: &mut cv };
    let hash = {
        let mut x = 1i32;
        hamt_int_hash(&mut x)
    };

    let inserted = node.hamt_node_insert(
        hash, 0, &mut k, &mut v, hamt_int_hash, hamt_int_equals, &mut conflict_kv,
    );
    assert!(inserted);

    // Now searching on this node should find the entry
    let mut sk = 1i32;
    let result = node.hamt_node_search(hash, 0, &mut sk, hamt_int_equals);
    if let HamtNode::Leaf(opt) = result {
        let kv = opt.expect("expected matching kv");
        assert_eq!(*kv.key, 1);
        assert_eq!(*kv.value, 10);
    } else {
        panic!("expected Leaf(Some(_))");
    }
}

#[test]
fn test_node_insert_replace_in_leaf() {
    // Start with a populated leaf, then insert with same key
    let mut k_orig = 1i32;
    let mut v_orig = 10i32;
    let mut node: HamtNode<i32, i32> = HamtNode::Leaf(Some(KeyValue {
        key: &mut k_orig,
        value: &mut v_orig,
    }));
    let mut k_new = 1i32;
    let mut v_new = 100i32;
    let mut ck = 0i32;
    let mut cv = 0i32;
    let mut conflict_kv = KeyValue { key: &mut ck, value: &mut cv };
    let hash = {
        let mut x = 1i32;
        hamt_int_hash(&mut x)
    };

    let inserted = node.hamt_node_insert(
        hash, 0, &mut k_new, &mut v_new, hamt_int_hash, hamt_int_equals, &mut conflict_kv,
    );
    assert!(!inserted);
    // conflict_kv should hold the original
    assert_eq!(*conflict_kv.key, 1);
    assert_eq!(*conflict_kv.value, 10);

    // Searching should yield the new value
    let mut sk = 1i32;
    let result = node.hamt_node_search(hash, 0, &mut sk, hamt_int_equals);
    if let HamtNode::Leaf(opt) = result {
        let kv = opt.expect("expected matching kv");
        assert_eq!(*kv.value, 100);
    } else {
        panic!("expected Leaf(Some(_))");
    }
}

#[test]
fn test_node_insert_split_leaf_then_recurse() {
    // Insert a different key into a populated leaf — should split into Sub
    let mut k_orig = 1i32;
    let mut v_orig = 10i32;
    let mut node: HamtNode<i32, i32> = HamtNode::Leaf(Some(KeyValue {
        key: &mut k_orig,
        value: &mut v_orig,
    }));
    let mut k_new = 2i32;
    let mut v_new = 20i32;
    let mut ck = 0i32;
    let mut cv = 0i32;
    let mut conflict_kv = KeyValue { key: &mut ck, value: &mut cv };
    let hash = {
        let mut x = 2i32;
        hamt_int_hash(&mut x)
    };

    let inserted = node.hamt_node_insert(
        hash, 0, &mut k_new, &mut v_new, hamt_int_hash, hamt_int_equals, &mut conflict_kv,
    );
    assert!(inserted);
    // Node should now be Sub
    assert!(!node.is_leaf());

    // Both keys should be searchable through the (now Sub) node
    let mut sk1 = 1i32;
    let h1 = {
        let mut x = 1i32;
        hamt_int_hash(&mut x)
    };
    let r1 = node.hamt_node_search(h1, 0, &mut sk1, hamt_int_equals);
    if let HamtNode::Leaf(opt) = r1 {
        let kv = opt.expect("expected key 1 found");
        assert_eq!(*kv.value, 10);
    } else {
        panic!("expected Leaf(Some(_))");
    }

    let mut sk2 = 2i32;
    let r2 = node.hamt_node_search(hash, 0, &mut sk2, hamt_int_equals);
    if let HamtNode::Leaf(opt) = r2 {
        let kv = opt.expect("expected key 2 found");
        assert_eq!(*kv.value, 20);
    } else {
        panic!("expected Leaf(Some(_))");
    }
}

#[test]
fn test_node_remove_from_sub() {
    // Build sub-node by inserting into a leaf
    let mut k_orig = 1i32;
    let mut v_orig = 10i32;
    let mut node: HamtNode<i32, i32> = HamtNode::Leaf(Some(KeyValue {
        key: &mut k_orig,
        value: &mut v_orig,
    }));
    let mut k2 = 2i32;
    let mut v2 = 20i32;
    let mut k3 = 3i32;
    let mut v3 = 30i32;
    let mut ck = 0i32;
    let mut cv = 0i32;
    {
        let mut conflict_kv = KeyValue { key: &mut ck, value: &mut cv };
        let h2 = { let mut x = 2i32; hamt_int_hash(&mut x) };
        node.hamt_node_insert(h2, 0, &mut k2, &mut v2, hamt_int_hash, hamt_int_equals, &mut conflict_kv);
        let h3 = { let mut x = 3i32; hamt_int_hash(&mut x) };
        node.hamt_node_insert(h3, 0, &mut k3, &mut v3, hamt_int_hash, hamt_int_equals, &mut conflict_kv);
    }
    // node should be Sub now
    assert!(!node.is_leaf());

    // Remove key 2
    let mut rk = 2i32;
    let mut rkk = 0i32;
    let mut rkv_v = 0i32;
    let mut removed_kv = KeyValue { key: &mut rkk, value: &mut rkv_v };
    let h2 = { let mut x = 2i32; hamt_int_hash(&mut x) };
    let removed = node.hamt_node_remove(h2, 0, &mut rk, hamt_int_equals, &mut removed_kv);
    assert!(removed);
    assert_eq!(*removed_kv.key, 2);
    assert_eq!(*removed_kv.value, 20);

    // The other two keys should still be searchable
    let mut sk1 = 1i32;
    let h1 = { let mut x = 1i32; hamt_int_hash(&mut x) };
    let r1 = node.hamt_node_search(h1, 0, &mut sk1, hamt_int_equals);
    if let HamtNode::Leaf(opt) = r1 {
        let kv = opt.expect("key 1 missing");
        assert_eq!(*kv.value, 10);
    } else {
        panic!("expected Leaf");
    }

    let mut sk3 = 3i32;
    let h3 = { let mut x = 3i32; hamt_int_hash(&mut x) };
    let r3 = node.hamt_node_search(h3, 0, &mut sk3, hamt_int_equals);
    if let HamtNode::Leaf(opt) = r3 {
        let kv = opt.expect("key 3 missing");
        assert_eq!(*kv.value, 30);
    } else {
        panic!("expected Leaf");
    }

    // Removed key shouldn't be found anymore
    let mut sk2 = 2i32;
    let r2 = node.hamt_node_search(h2, 0, &mut sk2, hamt_int_equals);
    if let HamtNode::Leaf(opt) = r2 {
        assert!(opt.is_none());
    } else {
        panic!("expected Leaf(None)");
    }
}

#[test]
fn test_node_remove_on_leaf_returns_false() {
    // hamt_node_remove on a leaf node should return false (per implementation).
    let mut k = 1i32;
    let mut v = 10i32;
    let mut node: HamtNode<i32, i32> = HamtNode::Leaf(Some(KeyValue {
        key: &mut k,
        value: &mut v,
    }));
    let mut rk = 1i32;
    let mut rkk = 0i32;
    let mut rkv_v = 0i32;
    let mut removed_kv = KeyValue { key: &mut rkk, value: &mut rkv_v };
    let h = { let mut x = 1i32; hamt_int_hash(&mut x) };
    let removed = node.hamt_node_remove(h, 0, &mut rk, hamt_int_equals, &mut removed_kv);
    assert!(!removed);
}

fn no_dealloc(_: &mut i32) {}

#[test]
fn test_node_destroy_leaf() {
    let mut k = 1i32;
    let mut v = 10i32;
    let mut node: HamtNode<i32, i32> = HamtNode::Leaf(Some(KeyValue {
        key: &mut k,
        value: &mut v,
    }));
    node.hamt_node_destroy(no_dealloc, no_dealloc);
    // After destroy, the leaf should be empty
    if let HamtNode::Leaf(opt) = &node {
        assert!(opt.is_none());
    } else {
        panic!("expected Leaf(None) after destroy");
    }
}

#[test]
fn test_node_destroy_empty_leaf() {
    let mut node: HamtNode<i32, i32> = HamtNode::Leaf(None);
    node.hamt_node_destroy(no_dealloc, no_dealloc);
    if let HamtNode::Leaf(opt) = &node {
        assert!(opt.is_none());
    } else {
        panic!("expected Leaf(None) after destroy");
    }
}

fn int_to_str(v: &mut i32) -> String {
    format!("{}", *v)
}

#[test]
fn test_node_print_leaf() {
    let mut k = 5i32;
    let mut v = 50i32;
    let mut node: HamtNode<i32, i32> = HamtNode::Leaf(Some(KeyValue {
        key: &mut k,
        value: &mut v,
    }));
    // Just exercise the print path (no panic)
    node.hamt_node_print(0, int_to_str, int_to_str);
}

#[test]
fn test_node_print_empty_leaf() {
    let mut node: HamtNode<i32, i32> = HamtNode::Leaf(None);
    node.hamt_node_print(2, int_to_str, int_to_str);
}

fn main() {}
