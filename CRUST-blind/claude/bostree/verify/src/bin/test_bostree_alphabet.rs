use Bostree::bostree::{
    bostree_next_node, bostree_previous_node, bostree_rank, BOSTree, BOSNode,
};
use std::cell::RefCell;
use std::rc::Rc;

fn cmp(a: &str, b: &str) -> i32 {
    use std::cmp::Ordering;
    match a.cmp(b) {
        Ordering::Less => -1,
        Ordering::Equal => 0,
        Ordering::Greater => 1,
    }
}

fn build_alphabet() -> BOSTree {
    let mut tree = BOSTree::bostree_new(cmp, None);
    for c in b'A'..=b'Z' {
        tree.bostree_insert((c as char).to_string(), Some(String::from("v")));
    }
    tree
}

fn parent_key(n: &Rc<RefCell<BOSNode>>) -> Option<String> {
    n.borrow()
        .parent_node
        .as_ref()
        .and_then(|w| w.upgrade())
        .map(|p| p.borrow().key.clone())
}

fn left_key(n: &Rc<RefCell<BOSNode>>) -> Option<String> {
    n.borrow().left_child_node.as_ref().map(|c| c.borrow().key.clone())
}

fn right_key(n: &Rc<RefCell<BOSNode>>) -> Option<String> {
    n.borrow().right_child_node.as_ref().map(|c| c.borrow().key.clone())
}

#[test]
fn test_alphabet_structure() {
    let tree = build_alphabet();
    assert_eq!(tree.bostree_node_count(), 26);

    // From C ground truth
    // Each row: (key, lcc, rcc, depth, rank, parent, lc, rc)
    let expected: &[(&str, u32, u32, u32, u32, Option<&str>, Option<&str>, Option<&str>)] = &[
        ("A", 0, 0, 0, 0, Some("B"), None, None),
        ("B", 1, 1, 1, 1, Some("D"), Some("A"), Some("C")),
        ("C", 0, 0, 0, 2, Some("B"), None, None),
        ("D", 3, 3, 2, 3, Some("H"), Some("B"), Some("F")),
        ("E", 0, 0, 0, 4, Some("F"), None, None),
        ("F", 1, 1, 1, 5, Some("D"), Some("E"), Some("G")),
        ("G", 0, 0, 0, 6, Some("F"), None, None),
        ("H", 7, 7, 3, 7, Some("P"), Some("D"), Some("L")),
        ("I", 0, 0, 0, 8, Some("J"), None, None),
        ("J", 1, 1, 1, 9, Some("L"), Some("I"), Some("K")),
        ("K", 0, 0, 0, 10, Some("J"), None, None),
        ("L", 3, 3, 2, 11, Some("H"), Some("J"), Some("N")),
        ("M", 0, 0, 0, 12, Some("N"), None, None),
        ("N", 1, 1, 1, 13, Some("L"), Some("M"), Some("O")),
        ("O", 0, 0, 0, 14, Some("N"), None, None),
        ("P", 15, 10, 4, 15, None, Some("H"), Some("T")),
        ("Q", 0, 0, 0, 16, Some("R"), None, None),
        ("R", 1, 1, 1, 17, Some("T"), Some("Q"), Some("S")),
        ("S", 0, 0, 0, 18, Some("R"), None, None),
        ("T", 3, 6, 3, 19, Some("P"), Some("R"), Some("X")),
        ("U", 0, 0, 0, 20, Some("V"), None, None),
        ("V", 1, 1, 1, 21, Some("X"), Some("U"), Some("W")),
        ("W", 0, 0, 0, 22, Some("V"), None, None),
        ("X", 3, 2, 2, 23, Some("T"), Some("V"), Some("Y")),
        ("Y", 0, 1, 1, 24, Some("X"), None, Some("Z")),
        ("Z", 0, 0, 0, 25, Some("Y"), None, None),
    ];

    for (i, &(key, lcc, rcc, depth, rank, parent, lc, rc)) in expected.iter().enumerate() {
        let node = tree.bostree_select(i as u32).unwrap();
        {
            let nb = node.borrow();
            assert_eq!(nb.key, key, "select({}) key", i);
            assert_eq!(nb.left_child_count, lcc, "key {} lcc", key);
            assert_eq!(nb.right_child_count, rcc, "key {} rcc", key);
            assert_eq!(nb.depth, depth, "key {} depth", key);
        }
        assert_eq!(bostree_rank(&node), rank, "rank for {}", key);
        assert_eq!(parent_key(&node).as_deref(), parent, "parent for {}", key);
        assert_eq!(left_key(&node).as_deref(), lc, "lc for {}", key);
        assert_eq!(right_key(&node).as_deref(), rc, "rc for {}", key);
    }

    // root is P
    let root = tree.root_node.as_ref().unwrap();
    assert_eq!(root.borrow().key, "P");
}

#[test]
fn test_alphabet_inorder_via_next() {
    let tree = build_alphabet();
    let mut current = tree.bostree_select(0);
    let mut keys: Vec<String> = Vec::new();
    while let Some(n) = current {
        keys.push(n.borrow().key.clone());
        current = bostree_next_node(&n);
    }
    let expected: Vec<String> = (b'A'..=b'Z').map(|c| (c as char).to_string()).collect();
    assert_eq!(keys, expected);
}

#[test]
fn test_alphabet_reverse_via_previous() {
    let tree = build_alphabet();
    let last = tree.bostree_select(25).unwrap();
    let mut current = Some(last);
    let mut keys: Vec<String> = Vec::new();
    while let Some(n) = current {
        keys.push(n.borrow().key.clone());
        current = bostree_previous_node(&n);
    }
    let mut expected: Vec<String> = (b'A'..=b'Z').map(|c| (c as char).to_string()).collect();
    expected.reverse();
    assert_eq!(keys, expected);
}

#[test]
fn test_alphabet_lookup_each() {
    let tree = build_alphabet();
    for c in b'A'..=b'Z' {
        let key = (c as char).to_string();
        let n = tree.bostree_lookup(&key).unwrap();
        assert_eq!(n.borrow().key, key);
    }
}

#[test]
fn test_alphabet_remove_m() {
    let mut tree = build_alphabet();
    let m = tree.bostree_lookup("M").unwrap();
    tree.bostree_remove(&m);
    assert_eq!(tree.bostree_node_count(), 25);

    // From C ground truth
    let expected: &[(&str, u32, u32, u32, u32, Option<&str>, Option<&str>, Option<&str>)] = &[
        ("A", 0, 0, 0, 0, Some("B"), None, None),
        ("B", 1, 1, 1, 1, Some("D"), Some("A"), Some("C")),
        ("C", 0, 0, 0, 2, Some("B"), None, None),
        ("D", 3, 3, 2, 3, Some("H"), Some("B"), Some("F")),
        ("E", 0, 0, 0, 4, Some("F"), None, None),
        ("F", 1, 1, 1, 5, Some("D"), Some("E"), Some("G")),
        ("G", 0, 0, 0, 6, Some("F"), None, None),
        ("H", 7, 6, 3, 7, Some("P"), Some("D"), Some("L")),
        ("I", 0, 0, 0, 8, Some("J"), None, None),
        ("J", 1, 1, 1, 9, Some("L"), Some("I"), Some("K")),
        ("K", 0, 0, 0, 10, Some("J"), None, None),
        ("L", 3, 2, 2, 11, Some("H"), Some("J"), Some("N")),
        ("N", 0, 1, 1, 12, Some("L"), None, Some("O")),
        ("O", 0, 0, 0, 13, Some("N"), None, None),
        ("P", 14, 10, 4, 14, None, Some("H"), Some("T")),
        ("Q", 0, 0, 0, 15, Some("R"), None, None),
        ("R", 1, 1, 1, 16, Some("T"), Some("Q"), Some("S")),
        ("S", 0, 0, 0, 17, Some("R"), None, None),
        ("T", 3, 6, 3, 18, Some("P"), Some("R"), Some("X")),
        ("U", 0, 0, 0, 19, Some("V"), None, None),
        ("V", 1, 1, 1, 20, Some("X"), Some("U"), Some("W")),
        ("W", 0, 0, 0, 21, Some("V"), None, None),
        ("X", 3, 2, 2, 22, Some("T"), Some("V"), Some("Y")),
        ("Y", 0, 1, 1, 23, Some("X"), None, Some("Z")),
        ("Z", 0, 0, 0, 24, Some("Y"), None, None),
    ];

    for (i, &(key, lcc, rcc, depth, rank, parent, lc, rc)) in expected.iter().enumerate() {
        let node = tree.bostree_select(i as u32).unwrap();
        {
            let nb = node.borrow();
            assert_eq!(nb.key, key, "select({}) key", i);
            assert_eq!(nb.left_child_count, lcc, "key {} lcc", key);
            assert_eq!(nb.right_child_count, rcc, "key {} rcc", key);
            assert_eq!(nb.depth, depth, "key {} depth", key);
        }
        assert_eq!(bostree_rank(&node), rank, "rank for {}", key);
        assert_eq!(parent_key(&node).as_deref(), parent, "parent for {}", key);
        assert_eq!(left_key(&node).as_deref(), lc, "lc for {}", key);
        assert_eq!(right_key(&node).as_deref(), rc, "rc for {}", key);
    }

    // M no longer present
    assert!(tree.bostree_lookup("M").is_none());
}

#[test]
fn test_alphabet_reverse_insertion_balanced() {
    // Insert Z->A; verify expected balanced tree
    let mut tree = BOSTree::bostree_new(cmp, None);
    for c in (b'A'..=b'Z').rev() {
        tree.bostree_insert((c as char).to_string(), None);
    }
    assert_eq!(tree.bostree_node_count(), 26);

    let expected: &[(&str, u32, u32, u32, u32, Option<&str>, Option<&str>, Option<&str>)] = &[
        ("A", 0, 0, 0, 0, Some("B"), None, None),
        ("B", 1, 0, 1, 1, Some("C"), Some("A"), None),
        ("C", 2, 3, 2, 2, Some("G"), Some("B"), Some("E")),
        ("D", 0, 0, 0, 3, Some("E"), None, None),
        ("E", 1, 1, 1, 4, Some("C"), Some("D"), Some("F")),
        ("F", 0, 0, 0, 5, Some("E"), None, None),
        ("G", 6, 3, 3, 6, Some("K"), Some("C"), Some("I")),
        ("H", 0, 0, 0, 7, Some("I"), None, None),
        ("I", 1, 1, 1, 8, Some("G"), Some("H"), Some("J")),
        ("J", 0, 0, 0, 9, Some("I"), None, None),
        ("K", 10, 15, 4, 10, None, Some("G"), Some("S")),
        ("L", 0, 0, 0, 11, Some("M"), None, None),
        ("M", 1, 1, 1, 12, Some("O"), Some("L"), Some("N")),
        ("N", 0, 0, 0, 13, Some("M"), None, None),
        ("O", 3, 3, 2, 14, Some("S"), Some("M"), Some("Q")),
        ("P", 0, 0, 0, 15, Some("Q"), None, None),
        ("Q", 1, 1, 1, 16, Some("O"), Some("P"), Some("R")),
        ("R", 0, 0, 0, 17, Some("Q"), None, None),
        ("S", 7, 7, 3, 18, Some("K"), Some("O"), Some("W")),
        ("T", 0, 0, 0, 19, Some("U"), None, None),
        ("U", 1, 1, 1, 20, Some("W"), Some("T"), Some("V")),
        ("V", 0, 0, 0, 21, Some("U"), None, None),
        ("W", 3, 3, 2, 22, Some("S"), Some("U"), Some("Y")),
        ("X", 0, 0, 0, 23, Some("Y"), None, None),
        ("Y", 1, 1, 1, 24, Some("W"), Some("X"), Some("Z")),
        ("Z", 0, 0, 0, 25, Some("Y"), None, None),
    ];

    for (i, &(key, lcc, rcc, depth, rank, parent, lc, rc)) in expected.iter().enumerate() {
        let node = tree.bostree_select(i as u32).unwrap();
        {
            let nb = node.borrow();
            assert_eq!(nb.key, key, "select({}) key", i);
            assert_eq!(nb.left_child_count, lcc, "key {} lcc", key);
            assert_eq!(nb.right_child_count, rcc, "key {} rcc", key);
            assert_eq!(nb.depth, depth, "key {} depth", key);
        }
        assert_eq!(bostree_rank(&node), rank, "rank for {}", key);
        assert_eq!(parent_key(&node).as_deref(), parent, "parent for {}", key);
        assert_eq!(left_key(&node).as_deref(), lc, "lc for {}", key);
        assert_eq!(right_key(&node).as_deref(), rc, "rc for {}", key);
    }
}

fn main() {}
