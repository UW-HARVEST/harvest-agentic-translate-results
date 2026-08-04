use Bostree::bostree::{
    BOSTree, bostree_next_node, bostree_previous_node, bostree_rank, bostree_node_weak_ref,
};
use Bostree::test_tree_sanity;

fn strcmp_cmp(a: &str, b: &str) -> i32 {
    a.cmp(b) as i32
}

fn build_az_tree() -> BOSTree {
    let mut t = BOSTree::bostree_new(strcmp_cmp, None);
    for c in b'A'..=b'Z' {
        t.bostree_insert(String::from(c as char), None);
    }
    t
}

#[test]
fn test_empty_tree() {
    let tree = BOSTree::bostree_new(strcmp_cmp, None);
    assert_eq!(tree.bostree_node_count(), 0);
    assert!(tree.bostree_select(0).is_none());
    assert!(tree.bostree_lookup("A").is_none());
}

#[test]
fn test_single_node() {
    let mut tree = BOSTree::bostree_new(strcmp_cmp, None);
    tree.bostree_insert("X".to_string(), None);
    assert_eq!(tree.bostree_node_count(), 1);
    let node = tree.bostree_select(0).unwrap();
    assert_eq!(node.borrow().key, "X");
    assert_eq!(node.borrow().left_child_count, 0);
    assert_eq!(node.borrow().right_child_count, 0);
    assert_eq!(node.borrow().depth, 0);
    assert_eq!(bostree_rank(&node), 0);
    assert!(bostree_next_node(&node).is_none());
    assert!(bostree_previous_node(&node).is_none());
}

#[test]
fn test_insert_az_count() {
    let mut tree = BOSTree::bostree_new(strcmp_cmp, None);
    for (i, c) in (b'A'..=b'Z').enumerate() {
        tree.bostree_insert(String::from(c as char), None);
        assert_eq!(tree.bostree_node_count(), (i + 1) as u32);
    }
}

#[test]
fn test_root_after_full_insert() {
    let tree = build_az_tree();
    let root = tree.root_node.as_ref().unwrap();
    assert_eq!(root.borrow().key, "P");
    assert_eq!(root.borrow().left_child_count, 15);
    assert_eq!(root.borrow().right_child_count, 10);
    assert_eq!(root.borrow().depth, 4);
}

#[test]
fn test_select_and_rank_all() {
    let tree = build_az_tree();
    for i in 0u32..26 {
        let node = tree.bostree_select(i).unwrap();
        let expected_key = String::from((b'A' + i as u8) as char);
        assert_eq!(node.borrow().key, expected_key);
        assert_eq!(bostree_rank(&node), i);
    }
}

#[test]
fn test_select_out_of_range() {
    let tree = build_az_tree();
    assert!(tree.bostree_select(26).is_none());
    assert!(tree.bostree_select(100).is_none());
}

#[test]
fn test_lookup() {
    let tree = build_az_tree();
    let m = tree.bostree_lookup("M").unwrap();
    assert_eq!(m.borrow().key, "M");
    assert_eq!(m.borrow().left_child_count, 0);
    assert_eq!(m.borrow().right_child_count, 0);
    assert_eq!(m.borrow().depth, 0);

    let a = tree.bostree_lookup("A").unwrap();
    assert_eq!(a.borrow().key, "A");
    assert_eq!(a.borrow().left_child_count, 0);
    assert_eq!(a.borrow().right_child_count, 0);
    assert_eq!(a.borrow().depth, 0);

    let z = tree.bostree_lookup("Z").unwrap();
    assert_eq!(z.borrow().key, "Z");
    assert_eq!(z.borrow().left_child_count, 0);
    assert_eq!(z.borrow().right_child_count, 0);
    assert_eq!(z.borrow().depth, 0);
}

#[test]
fn test_lookup_nonexistent() {
    let tree = build_az_tree();
    assert!(tree.bostree_lookup("0").is_none());
    assert!(tree.bostree_lookup("a").is_none());
}

#[test]
fn test_next_node() {
    let tree = build_az_tree();
    let first = tree.bostree_select(0).unwrap();
    assert_eq!(first.borrow().key, "A");
    let second = bostree_next_node(&first).unwrap();
    assert_eq!(second.borrow().key, "B");
    // Walk entire tree via next_node
    let mut cur = tree.bostree_select(0);
    let mut count = 0u32;
    while let Some(node) = cur {
        let expected = String::from((b'A' + count as u8) as char);
        assert_eq!(node.borrow().key, expected);
        cur = bostree_next_node(&node);
        count += 1;
    }
    assert_eq!(count, 26);
}

#[test]
fn test_previous_node() {
    let tree = build_az_tree();
    let last = tree.bostree_select(25).unwrap();
    assert_eq!(last.borrow().key, "Z");
    assert!(bostree_next_node(&last).is_none());
    let prev = bostree_previous_node(&last).unwrap();
    assert_eq!(prev.borrow().key, "Y");
    let first = tree.bostree_select(0).unwrap();
    assert!(bostree_previous_node(&first).is_none());
}

#[test]
fn test_prev_of_next() {
    let tree = build_az_tree();
    let first = tree.bostree_select(0).unwrap();
    let second = bostree_next_node(&first).unwrap();
    let back = bostree_previous_node(&second).unwrap();
    assert_eq!(back.borrow().key, "A");
}

#[test]
fn test_remove_single() {
    let tree = build_az_tree();
    let mut tree = tree;
    let m = tree.bostree_lookup("M").unwrap();
    tree.bostree_remove(&m);
    assert_eq!(tree.bostree_node_count(), 25);
    assert!(tree.bostree_lookup("M").is_none());
    // L's next should be N
    let l = tree.bostree_lookup("L").unwrap();
    let next = bostree_next_node(&l).unwrap();
    assert_eq!(next.borrow().key, "N");
    test_tree_sanity(&tree);
}

#[test]
fn test_remove_all() {
    let mut tree = build_az_tree();
    for c in b'A'..=b'Z' {
        let key = String::from(c as char);
        let node = tree.bostree_lookup(&key).unwrap();
        tree.bostree_remove(&node);
        test_tree_sanity(&tree);
    }
    assert_eq!(tree.bostree_node_count(), 0);
}

#[test]
fn test_three_nodes() {
    let mut t = BOSTree::bostree_new(strcmp_cmp, None);
    t.bostree_insert("B".to_string(), None);
    t.bostree_insert("A".to_string(), None);
    t.bostree_insert("C".to_string(), None);
    let root = t.root_node.as_ref().unwrap();
    assert_eq!(root.borrow().key, "B");
    assert_eq!(root.borrow().left_child_count, 1);
    assert_eq!(root.borrow().right_child_count, 1);
    assert_eq!(root.borrow().depth, 1);
    assert_eq!(t.bostree_node_count(), 3);
    assert_eq!(t.bostree_select(0).unwrap().borrow().key, "A");
    assert_eq!(bostree_rank(&t.bostree_select(0).unwrap()), 0);
    assert_eq!(t.bostree_select(1).unwrap().borrow().key, "B");
    assert_eq!(bostree_rank(&t.bostree_select(1).unwrap()), 1);
    assert_eq!(t.bostree_select(2).unwrap().borrow().key, "C");
    assert_eq!(bostree_rank(&t.bostree_select(2).unwrap()), 2);
}

#[test]
fn test_rotation_ascending() {
    // Insert A, B, C in ascending order triggers left rotation
    let mut t = BOSTree::bostree_new(strcmp_cmp, None);
    t.bostree_insert("A".to_string(), None);
    t.bostree_insert("B".to_string(), None);
    t.bostree_insert("C".to_string(), None);
    let root = t.root_node.as_ref().unwrap();
    assert_eq!(root.borrow().key, "B");
    assert_eq!(root.borrow().left_child_count, 1);
    assert_eq!(root.borrow().right_child_count, 1);
    assert_eq!(root.borrow().depth, 1);

    t.bostree_insert("D".to_string(), None);
    t.bostree_insert("E".to_string(), None);
    let root = t.root_node.as_ref().unwrap();
    assert_eq!(root.borrow().key, "B");
    assert_eq!(root.borrow().left_child_count, 1);
    assert_eq!(root.borrow().right_child_count, 3);
    assert_eq!(root.borrow().depth, 2);
    assert_eq!(t.bostree_node_count(), 5);
    test_tree_sanity(&t);
}

#[test]
fn test_weak_ref() {
    let mut t = BOSTree::bostree_new(strcmp_cmp, None);
    let node = t.bostree_insert("A".to_string(), None);
    assert_eq!(node.borrow().weak_ref_count, 1);
    assert_eq!(node.borrow().weak_ref_node_valid, 1);
    bostree_node_weak_ref(&node);
    assert_eq!(node.borrow().weak_ref_count, 2);
    let result = t.bostree_node_weak_unref(&node);
    assert_eq!(node.borrow().weak_ref_count, 1);
    assert!(result.is_some());
}

#[test]
fn test_insert_with_data() {
    let mut t = BOSTree::bostree_new(strcmp_cmp, None);
    t.bostree_insert("key1".to_string(), Some("data1".to_string()));
    let node = t.bostree_lookup("key1").unwrap();
    assert_eq!(node.borrow().key, "key1");
    assert_eq!(node.borrow().data.as_deref(), Some("data1"));
}

#[test]
fn test_sanity_after_insert_az() {
    let tree = build_az_tree();
    test_tree_sanity(&tree);
}

#[test]
fn test_remove_each_from_full_tree() {
    // Mirrors remove_bug.c: build A..Y, remove each one individually
    for c in b'A'..b'Z' {
        let mut t = BOSTree::bostree_new(strcmp_cmp, None);
        for k in b'A'..b'Z' {
            t.bostree_insert(String::from(k as char), None);
        }
        let key = String::from(c as char);
        let node = t.bostree_lookup(&key).unwrap();
        t.bostree_remove(&node);
        test_tree_sanity(&t);
        assert_eq!(t.bostree_node_count(), (b'Z' - b'A' - 1) as u32);
    }
}

#[test]
fn test_remove_g_then_h() {
    // Mirrors remove_bug_2.c
    let mut t = BOSTree::bostree_new(strcmp_cmp, None);
    for c in b'A'..b'Z' {
        t.bostree_insert(String::from(c as char), None);
    }
    let g = t.bostree_lookup("G").unwrap();
    t.bostree_remove(&g);
    let h = t.bostree_lookup("H").unwrap();
    t.bostree_remove(&h);
    test_tree_sanity(&t);
    assert!(t.bostree_lookup("E").is_some());
}

#[test]
fn test_node_count_empty() {
    let t = BOSTree::bostree_new(strcmp_cmp, None);
    assert_eq!(t.bostree_node_count(), 0);
}

#[test]
fn test_remove_root_single_node() {
    let mut t = BOSTree::bostree_new(strcmp_cmp, None);
    let node = t.bostree_insert("A".to_string(), None);
    t.bostree_remove(&node);
    assert_eq!(t.bostree_node_count(), 0);
    assert!(t.root_node.is_none());
}

#[test]
fn test_remove_root_with_left_child_only() {
    let mut t = BOSTree::bostree_new(strcmp_cmp, None);
    t.bostree_insert("B".to_string(), None);
    t.bostree_insert("A".to_string(), None);
    let root = t.bostree_lookup("B").unwrap();
    t.bostree_remove(&root);
    assert_eq!(t.bostree_node_count(), 1);
    assert_eq!(t.root_node.as_ref().unwrap().borrow().key, "A");
    test_tree_sanity(&t);
}

#[test]
fn test_remove_root_with_right_child_only() {
    let mut t = BOSTree::bostree_new(strcmp_cmp, None);
    t.bostree_insert("A".to_string(), None);
    t.bostree_insert("B".to_string(), None);
    let root = t.bostree_lookup("A").unwrap();
    t.bostree_remove(&root);
    assert_eq!(t.bostree_node_count(), 1);
    assert_eq!(t.root_node.as_ref().unwrap().borrow().key, "B");
    test_tree_sanity(&t);
}

fn main() {}
