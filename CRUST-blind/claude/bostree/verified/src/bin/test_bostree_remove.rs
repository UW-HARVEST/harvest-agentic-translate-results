use Bostree::bostree::{BOSTree};
use std::rc::Rc;

fn cmp(a: &str, b: &str) -> i32 {
    use std::cmp::Ordering;
    match a.cmp(b) {
        Ordering::Less => -1,
        Ordering::Equal => 0,
        Ordering::Greater => 1,
    }
}

#[test]
fn test_remove_only_node() {
    let mut tree = BOSTree::bostree_new(cmp, None);
    tree.bostree_insert(String::from("only"), None);
    let n = tree.bostree_lookup("only").unwrap();
    tree.bostree_remove(&n);
    assert_eq!(tree.bostree_node_count(), 0);
    assert!(tree.root_node.is_none());
    assert!(tree.bostree_lookup("only").is_none());
}

#[test]
fn test_remove_root_two_children() {
    // Insert B, A, C; remove B (the root with both children)
    let mut tree = BOSTree::bostree_new(cmp, None);
    tree.bostree_insert(String::from("B"), None);
    tree.bostree_insert(String::from("A"), None);
    tree.bostree_insert(String::from("C"), None);
    let b = tree.bostree_lookup("B").unwrap();
    tree.bostree_remove(&b);
    assert_eq!(tree.bostree_node_count(), 2);

    // From C output:
    // A (lcc=0,rcc=1,depth=1) is root, A->C right child
    let root = tree.root_node.as_ref().unwrap();
    assert_eq!(root.borrow().key, "A");
    let a = tree.bostree_select(0).unwrap();
    let c = tree.bostree_select(1).unwrap();
    {
        let ab = a.borrow();
        assert_eq!(ab.key, "A");
        assert_eq!(ab.left_child_count, 0);
        assert_eq!(ab.right_child_count, 1);
        assert_eq!(ab.depth, 1);
        assert!(ab.parent_node.is_none());
        assert!(ab.left_child_node.is_none());
        assert!(Rc::ptr_eq(ab.right_child_node.as_ref().unwrap(), &c));
    }
    {
        let cb = c.borrow();
        assert_eq!(cb.key, "C");
        assert_eq!(cb.left_child_count, 0);
        assert_eq!(cb.right_child_count, 0);
        assert_eq!(cb.depth, 0);
        let parent = cb.parent_node.as_ref().unwrap().upgrade().unwrap();
        assert!(Rc::ptr_eq(&parent, &a));
    }
}

#[test]
fn test_remove_leaf() {
    // Insert B, A, C; remove C (leaf)
    let mut tree = BOSTree::bostree_new(cmp, None);
    tree.bostree_insert(String::from("B"), None);
    tree.bostree_insert(String::from("A"), None);
    tree.bostree_insert(String::from("C"), None);
    let c = tree.bostree_lookup("C").unwrap();
    tree.bostree_remove(&c);
    assert_eq!(tree.bostree_node_count(), 2);
    let root = tree.root_node.as_ref().unwrap();
    assert_eq!(root.borrow().key, "B");

    let a = tree.bostree_select(0).unwrap();
    let b = tree.bostree_select(1).unwrap();
    {
        let ab = a.borrow();
        assert_eq!(ab.key, "A");
        assert_eq!(ab.left_child_count, 0);
        assert_eq!(ab.right_child_count, 0);
        assert_eq!(ab.depth, 0);
    }
    {
        let bb = b.borrow();
        assert_eq!(bb.key, "B");
        assert_eq!(bb.left_child_count, 1);
        assert_eq!(bb.right_child_count, 0);
        assert_eq!(bb.depth, 1);
        assert!(bb.parent_node.is_none());
        assert!(Rc::ptr_eq(bb.left_child_node.as_ref().unwrap(), &a));
        assert!(bb.right_child_node.is_none());
    }
}

#[test]
fn test_remove_root_one_child() {
    // Insert B, A; remove B
    let mut tree = BOSTree::bostree_new(cmp, None);
    tree.bostree_insert(String::from("B"), None);
    tree.bostree_insert(String::from("A"), None);
    let b = tree.bostree_lookup("B").unwrap();
    tree.bostree_remove(&b);
    assert_eq!(tree.bostree_node_count(), 1);
    let root = tree.root_node.as_ref().unwrap();
    assert_eq!(root.borrow().key, "A");
    let a = tree.bostree_select(0).unwrap();
    let ab = a.borrow();
    assert_eq!(ab.left_child_count, 0);
    assert_eq!(ab.right_child_count, 0);
    assert_eq!(ab.depth, 0);
    assert!(ab.parent_node.is_none());
    assert!(ab.left_child_node.is_none());
    assert!(ab.right_child_node.is_none());
}

#[test]
fn test_remove_invalidates_node() {
    let mut tree = BOSTree::bostree_new(cmp, None);
    tree.bostree_insert(String::from("only"), None);
    let n = tree.bostree_lookup("only").unwrap();
    // Take an additional weak ref so we can examine after removal
    let saved = Rc::clone(&n);
    tree.bostree_remove(&n);
    // After removal, node should have weak_ref_node_valid == 0
    assert_eq!(saved.borrow().weak_ref_node_valid, 0);
}

#[test]
fn test_alphabet_remove_g_then_h() {
    // Build A..Y, remove G, then H, verify final structure
    let mut tree = BOSTree::bostree_new(cmp, None);
    for c in b'A'..b'Z' {
        tree.bostree_insert((c as char).to_string(), None);
    }
    let g = tree.bostree_lookup("G").unwrap();
    tree.bostree_remove(&g);
    let h = tree.bostree_lookup("H").unwrap();
    tree.bostree_remove(&h);

    assert_eq!(tree.bostree_node_count(), 23);
    // From C output, root is P
    let root = tree.root_node.as_ref().unwrap();
    assert_eq!(root.borrow().key, "P");

    let expected: &[(&str, u32, u32, u32)] = &[
        ("A", 0, 0, 0),
        ("B", 1, 1, 1),
        ("C", 0, 0, 0),
        ("D", 3, 1, 2),
        ("E", 0, 0, 0),
        ("F", 5, 7, 3),
        ("I", 0, 0, 0),
        ("J", 1, 1, 1),
        ("K", 0, 0, 0),
        ("L", 3, 3, 2),
        ("M", 0, 0, 0),
        ("N", 1, 1, 1),
        ("O", 0, 0, 0),
        ("P", 13, 9, 4),
        ("Q", 0, 0, 0),
        ("R", 1, 1, 1),
        ("S", 0, 0, 0),
        ("T", 3, 5, 3),
        ("U", 0, 0, 0),
        ("V", 1, 3, 2),
        ("W", 0, 0, 0),
        ("X", 1, 1, 1),
        ("Y", 0, 0, 0),
    ];

    for (i, &(k, lcc, rcc, depth)) in expected.iter().enumerate() {
        let n = tree.bostree_select(i as u32).unwrap();
        let nb = n.borrow();
        assert_eq!(nb.key, k, "[{}] key", i);
        assert_eq!(nb.left_child_count, lcc, "{} lcc", k);
        assert_eq!(nb.right_child_count, rcc, "{} rcc", k);
        assert_eq!(nb.depth, depth, "{} depth", k);
    }

    // E must still be present
    assert!(tree.bostree_lookup("E").is_some());
    assert!(tree.bostree_lookup("G").is_none());
    assert!(tree.bostree_lookup("H").is_none());
}

#[test]
fn test_remove_each_node_from_alphabet() {
    // For each letter A..Y, build A..Y then remove that letter, ensuring count drops by 1
    for target in b'A'..b'Z' {
        let mut tree = BOSTree::bostree_new(cmp, None);
        for c in b'A'..b'Z' {
            tree.bostree_insert((c as char).to_string(), None);
        }
        let key = (target as char).to_string();
        let n = tree.bostree_lookup(&key).unwrap();
        tree.bostree_remove(&n);
        assert_eq!(tree.bostree_node_count(), (b'Z' - b'A' - 1) as u32);
        assert!(tree.bostree_lookup(&key).is_none());
        // every other letter should still be there
        for c in b'A'..b'Z' {
            if c == target { continue; }
            let k = (c as char).to_string();
            assert!(tree.bostree_lookup(&k).is_some(), "{} missing", k);
        }
    }
}

#[test]
fn test_remove_all_alphabet_in_order() {
    // Remove every letter A..Z in order; count should drop to 0.
    let mut tree = BOSTree::bostree_new(cmp, None);
    for c in b'A'..=b'Z' {
        tree.bostree_insert((c as char).to_string(), None);
    }
    for c in b'A'..=b'Z' {
        let key = (c as char).to_string();
        let n = tree.bostree_lookup(&key).unwrap();
        tree.bostree_remove(&n);
    }
    assert_eq!(tree.bostree_node_count(), 0);
    assert!(tree.root_node.is_none());
}

fn main() {}
