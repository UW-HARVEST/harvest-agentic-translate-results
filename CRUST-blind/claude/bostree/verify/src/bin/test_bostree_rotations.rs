use Bostree::bostree::{BOSTree};

fn cmp(a: &str, b: &str) -> i32 {
    use std::cmp::Ordering;
    match a.cmp(b) {
        Ordering::Less => -1,
        Ordering::Equal => 0,
        Ordering::Greater => 1,
    }
}

fn check_inorder(tree: &BOSTree, expected: &[(&str, u32, u32, u32)]) {
    assert_eq!(tree.bostree_node_count() as usize, expected.len());
    for (i, &(k, lcc, rcc, depth)) in expected.iter().enumerate() {
        let n = tree.bostree_select(i as u32).unwrap();
        let nb = n.borrow();
        assert_eq!(nb.key, k);
        assert_eq!(nb.left_child_count, lcc);
        assert_eq!(nb.right_child_count, rcc);
        assert_eq!(nb.depth, depth);
    }
}

#[test]
fn test_left_rotation_left_left_case() {
    // Insert 3,2,1 (decreasing) -> left-left case, single right rotation
    // Tree should become 2 (root) with 1 left, 3 right
    let mut tree = BOSTree::bostree_new(cmp, None);
    tree.bostree_insert(String::from("3"), None);
    tree.bostree_insert(String::from("2"), None);
    tree.bostree_insert(String::from("1"), None);
    let root = tree.root_node.as_ref().unwrap();
    assert_eq!(root.borrow().key, "2");
    check_inorder(&tree, &[("1", 0, 0, 0), ("2", 1, 1, 1), ("3", 0, 0, 0)]);
}

#[test]
fn test_right_rotation_right_right_case() {
    // Insert 1,2,3 -> right-right, single left rotation
    let mut tree = BOSTree::bostree_new(cmp, None);
    tree.bostree_insert(String::from("1"), None);
    tree.bostree_insert(String::from("2"), None);
    tree.bostree_insert(String::from("3"), None);
    let root = tree.root_node.as_ref().unwrap();
    assert_eq!(root.borrow().key, "2");
    check_inorder(&tree, &[("1", 0, 0, 0), ("2", 1, 1, 1), ("3", 0, 0, 0)]);
}

#[test]
fn test_left_right_case() {
    // Insert 3,1,2 -> left-right
    let mut tree = BOSTree::bostree_new(cmp, None);
    tree.bostree_insert(String::from("3"), None);
    tree.bostree_insert(String::from("1"), None);
    tree.bostree_insert(String::from("2"), None);
    let root = tree.root_node.as_ref().unwrap();
    assert_eq!(root.borrow().key, "2");
    check_inorder(&tree, &[("1", 0, 0, 0), ("2", 1, 1, 1), ("3", 0, 0, 0)]);
}

#[test]
fn test_right_left_case() {
    // Insert 1,3,2 -> right-left
    let mut tree = BOSTree::bostree_new(cmp, None);
    tree.bostree_insert(String::from("1"), None);
    tree.bostree_insert(String::from("3"), None);
    tree.bostree_insert(String::from("2"), None);
    let root = tree.root_node.as_ref().unwrap();
    assert_eq!(root.borrow().key, "2");
    check_inorder(&tree, &[("1", 0, 0, 0), ("2", 1, 1, 1), ("3", 0, 0, 0)]);
}

fn main() {}
