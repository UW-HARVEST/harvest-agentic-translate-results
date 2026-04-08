use quadtree::quadtree::quadtree::*;

// === Point tests ===

#[test]
fn test_point_new() {
    let p = QuadtreePoint::quadtree_point_new(5.0, 6.0);
    assert_eq!(p.x, 5.0);
    assert_eq!(p.y, 6.0);
}

#[test]
fn test_point_zero() {
    let p = QuadtreePoint::quadtree_point_new(0.0, 0.0);
    assert_eq!(p.x, 0.0);
    assert_eq!(p.y, 0.0);
}

#[test]
fn test_point_negative() {
    let p = QuadtreePoint::quadtree_point_new(-3.5, -7.2);
    assert_eq!(p.x, -3.5);
    assert_eq!(p.y, -7.2);
}

// === Bounds tests ===

#[test]
fn test_bounds_new_initial_values() {
    let b = QuadtreeBounds::quadtree_bounds_new();
    let nw = b.nw.as_ref().unwrap();
    let se = b.se.as_ref().unwrap();
    assert_eq!(nw.x, f64::INFINITY);
    assert_eq!(nw.y, f64::NEG_INFINITY);
    assert_eq!(se.x, f64::NEG_INFINITY);
    assert_eq!(se.y, f64::INFINITY);
    assert_eq!(b.width, 0.0);
    assert_eq!(b.height, 0.0);
}

#[test]
fn test_bounds_extend_single_point() {
    let b = QuadtreeBounds::quadtree_bounds_new();
    b.quadtree_bounds_extend(5.0, 5.0);
    let nw = b.nw.as_ref().unwrap();
    let se = b.se.as_ref().unwrap();
    assert_eq!(nw.x, 5.0);
    assert_eq!(nw.y, 5.0);
    assert_eq!(se.x, 5.0);
    assert_eq!(se.y, 5.0);
    assert_eq!(b.width, 0.0);
    assert_eq!(b.height, 0.0);
}

#[test]
fn test_bounds_extend_two_points() {
    let b = QuadtreeBounds::quadtree_bounds_new();
    b.quadtree_bounds_extend(5.0, 5.0);
    b.quadtree_bounds_extend(10.0, 10.0);
    let nw = b.nw.as_ref().unwrap();
    let se = b.se.as_ref().unwrap();
    assert_eq!(nw.x, 5.0);
    assert_eq!(nw.y, 10.0);
    assert_eq!(se.x, 10.0);
    assert_eq!(se.y, 5.0);
    assert_eq!(b.width, 5.0);
    assert_eq!(b.height, 5.0);
}

// === Node tests ===

#[test]
fn test_node_new_is_empty() {
    let node: QuadtreeNode<i32> = QuadtreeNode::quadtree_node_new();
    assert!(!node.quadtree_node_isleaf());
    assert!(node.quadtree_node_isempty());
    assert!(!node.quadtree_node_ispointer());
}

#[test]
fn test_node_with_bounds() {
    let node: QuadtreeNode<i32> = QuadtreeNode::quadtree_node_with_bounds(1.0, 1.0, 10.0, 10.0);
    let bounds = node.bounds.as_ref().unwrap();
    let nw = bounds.nw.as_ref().unwrap();
    let se = bounds.se.as_ref().unwrap();
    assert_eq!(nw.x, 1.0);
    assert_eq!(nw.y, 10.0);
    assert_eq!(se.x, 10.0);
    assert_eq!(se.y, 1.0);
    assert_eq!(bounds.width, 9.0);
    assert_eq!(bounds.height, 9.0);
}

#[test]
fn test_node_isleaf_when_point_set() {
    let mut node: QuadtreeNode<i32> = QuadtreeNode::quadtree_node_new();
    node.point = Some(Box::new(QuadtreePoint::quadtree_point_new(1.0, 2.0)));
    assert!(node.quadtree_node_isleaf());
    assert!(!node.quadtree_node_isempty());
    assert!(!node.quadtree_node_ispointer());
}

#[test]
fn test_node_ispointer_when_all_children() {
    let mut node: QuadtreeNode<i32> = QuadtreeNode::quadtree_node_new();
    node.nw = Some(Box::new(QuadtreeNode::quadtree_node_new()));
    node.ne = Some(Box::new(QuadtreeNode::quadtree_node_new()));
    node.sw = Some(Box::new(QuadtreeNode::quadtree_node_new()));
    node.se = Some(Box::new(QuadtreeNode::quadtree_node_new()));
    assert!(node.quadtree_node_ispointer());
    assert!(!node.quadtree_node_isempty());
    assert!(!node.quadtree_node_isleaf());
}

#[test]
fn test_node_not_pointer_if_leaf() {
    // If point is set AND all children set, isleaf=true, ispointer=false
    let mut node: QuadtreeNode<i32> = QuadtreeNode::quadtree_node_new();
    node.nw = Some(Box::new(QuadtreeNode::quadtree_node_new()));
    node.ne = Some(Box::new(QuadtreeNode::quadtree_node_new()));
    node.sw = Some(Box::new(QuadtreeNode::quadtree_node_new()));
    node.se = Some(Box::new(QuadtreeNode::quadtree_node_new()));
    node.point = Some(Box::new(QuadtreePoint::quadtree_point_new(1.0, 1.0)));
    assert!(node.quadtree_node_isleaf());
    assert!(!node.quadtree_node_ispointer());
    assert!(!node.quadtree_node_isempty());
}

#[test]
fn test_node_reset_clears_point() {
    let mut node: QuadtreeNode<i32> = QuadtreeNode::quadtree_node_new();
    node.point = Some(Box::new(QuadtreePoint::quadtree_point_new(1.0, 2.0)));
    node.key = Some(42);
    assert!(node.quadtree_node_isleaf());
    node.quadtree_node_reset(Some(|_| {}));
    assert!(!node.quadtree_node_isleaf());
    assert!(node.point.is_none());
    assert!(node.key.is_none());
}

// === Tree tests ===

#[test]
fn test_tree_new_bounds() {
    let tree: Quadtree<i32> = Quadtree::quadtree_new(0.0, 0.0, 10.0, 10.0);
    let root = tree.root.as_ref().unwrap();
    let bounds = root.bounds.as_ref().unwrap();
    let nw = bounds.nw.as_ref().unwrap();
    let se = bounds.se.as_ref().unwrap();
    assert_eq!(nw.x, 0.0);
    assert_eq!(nw.y, 10.0);
    assert_eq!(se.x, 10.0);
    assert_eq!(se.y, 0.0);
}

#[test]
fn test_tree_new_bounds_1_1_10_10() {
    let tree: Quadtree<i32> = Quadtree::quadtree_new(1.0, 1.0, 10.0, 10.0);
    let root = tree.root.as_ref().unwrap();
    let bounds = root.bounds.as_ref().unwrap();
    assert_eq!(bounds.nw.as_ref().unwrap().x, 1.0);
    assert_eq!(bounds.nw.as_ref().unwrap().y, 10.0);
    assert_eq!(bounds.se.as_ref().unwrap().x, 10.0);
    assert_eq!(bounds.se.as_ref().unwrap().y, 1.0);
}

#[test]
fn test_tree_initial_length_zero() {
    let tree: Quadtree<i32> = Quadtree::quadtree_new(0.0, 0.0, 10.0, 10.0);
    assert_eq!(tree.length, 0);
}

#[test]
fn test_tree_insert_first_point() {
    let tree: Quadtree<i32> = Quadtree::quadtree_new(0.0, 0.0, 10.0, 10.0);
    assert!(tree.quadtree_insert(5.0, 5.0, Some(1)));
    assert_eq!(tree.length, 1);
    let root = tree.root.as_ref().unwrap();
    assert_eq!(root.point.as_ref().unwrap().x, 5.0);
    assert_eq!(root.point.as_ref().unwrap().y, 5.0);
}

#[test]
fn test_tree_insert_at_boundary_origin() {
    let tree: Quadtree<i32> = Quadtree::quadtree_new(0.0, 0.0, 10.0, 10.0);
    assert!(tree.quadtree_insert(0.0, 0.0, Some(1)));
    assert_eq!(tree.length, 1);
}

#[test]
fn test_tree_insert_at_boundary_max() {
    let tree: Quadtree<i32> = Quadtree::quadtree_new(0.0, 0.0, 10.0, 10.0);
    assert!(tree.quadtree_insert(10.0, 10.0, Some(1)));
    assert_eq!(tree.length, 1);
}

#[test]
fn test_tree_insert_out_of_bounds() {
    let tree: Quadtree<i32> = Quadtree::quadtree_new(0.0, 0.0, 10.0, 10.0);
    assert!(!tree.quadtree_insert(-1.0, -1.0, Some(1)));
    assert!(!tree.quadtree_insert(11.0, 11.0, Some(1)));
    assert_eq!(tree.length, 0);
}

#[test]
fn test_tree_insert_out_of_bounds_1_1_10_10() {
    let tree: Quadtree<i32> = Quadtree::quadtree_new(1.0, 1.0, 10.0, 10.0);
    assert!(!tree.quadtree_insert(0.0, 0.0, Some(1)));
    assert!(!tree.quadtree_insert(0.0, 1.0, Some(1)));
    assert!(tree.quadtree_insert(1.0, 1.0, Some(1)));
    assert_eq!(tree.length, 1);
}

#[test]
fn test_tree_insert_replace_same_point() {
    let tree: Quadtree<i32> = Quadtree::quadtree_new(0.0, 0.0, 10.0, 10.0);
    assert!(tree.quadtree_insert(5.0, 5.0, Some(1)));
    assert_eq!(tree.length, 1);
    // Replace: returns true, length stays the same
    assert!(tree.quadtree_insert(5.0, 5.0, Some(2)));
    assert_eq!(tree.length, 1);
}

#[test]
fn test_tree_insert_two_points_causes_split() {
    let tree: Quadtree<i32> = Quadtree::quadtree_new(1.0, 1.0, 10.0, 10.0);
    assert!(tree.quadtree_insert(8.0, 2.0, Some(10)));
    assert_eq!(tree.length, 1);
    assert!(tree.root.as_ref().unwrap().quadtree_node_isleaf());

    assert!(tree.quadtree_insert(2.0, 3.0, Some(20)));
    assert_eq!(tree.length, 2);
    // After two different points, root should no longer be a leaf
    assert!(tree.root.as_ref().unwrap().point.is_none());
}

#[test]
fn test_tree_insert_replace_after_split() {
    let tree: Quadtree<i32> = Quadtree::quadtree_new(1.0, 1.0, 10.0, 10.0);
    assert!(tree.quadtree_insert(8.0, 2.0, Some(10)));
    assert!(tree.quadtree_insert(2.0, 3.0, Some(20)));
    assert_eq!(tree.length, 2);
    // Replace existing point after split
    assert!(tree.quadtree_insert(2.0, 3.0, Some(30)));
    assert_eq!(tree.length, 2);
}

#[test]
fn test_tree_search_found() {
    let tree: Quadtree<i32> = Quadtree::quadtree_new(0.0, 0.0, 10.0, 10.0);
    tree.quadtree_insert(5.0, 5.0, Some(1));
    let result = tree.quadtree_search(5.0, 5.0);
    assert!(result.is_some());
    assert_eq!(result.as_ref().unwrap().x, 5.0);
    assert_eq!(result.as_ref().unwrap().y, 5.0);
}

#[test]
fn test_tree_search_not_found() {
    let tree: Quadtree<i32> = Quadtree::quadtree_new(0.0, 0.0, 10.0, 10.0);
    tree.quadtree_insert(5.0, 5.0, Some(1));
    let result = tree.quadtree_search(99.0, 99.0);
    assert!(result.is_none());
}

#[test]
fn test_tree_search_empty_tree() {
    let tree: Quadtree<i32> = Quadtree::quadtree_new(0.0, 0.0, 10.0, 10.0);
    let result = tree.quadtree_search(5.0, 5.0);
    assert!(result.is_none());
}

#[test]
fn test_tree_search_after_multiple_inserts() {
    let tree: Quadtree<i32> = Quadtree::quadtree_new(0.0, 0.0, 10.0, 10.0);
    tree.quadtree_insert(0.0, 0.0, Some(1));
    tree.quadtree_insert(10.0, 10.0, Some(2));
    tree.quadtree_insert(5.0, 5.0, Some(3));
    tree.quadtree_insert(1.0, 1.0, Some(4));
    tree.quadtree_insert(9.0, 9.0, Some(5));
    tree.quadtree_insert(2.0, 8.0, Some(6));
    tree.quadtree_insert(8.0, 2.0, Some(7));

    assert!(tree.quadtree_search(0.0, 0.0).is_some());
    assert!(tree.quadtree_search(1.0, 1.0).is_some());
    assert!(tree.quadtree_search(9.0, 9.0).is_some());
    assert_eq!(tree.quadtree_search(3.0, 1.1).is_none(), true);
    assert_eq!(tree.quadtree_search(7.0, 7.0).is_none(), true);
    assert_eq!(tree.length, 7);
}

#[test]
fn test_tree_search_returns_correct_coords() {
    let tree: Quadtree<i32> = Quadtree::quadtree_new(0.0, 0.0, 10.0, 10.0);
    tree.quadtree_insert(3.0, 1.1, Some(42));
    let found = tree.quadtree_search(3.0, 1.1);
    assert!(found.is_some());
    assert_eq!(found.as_ref().unwrap().x, 3.0);
    assert_eq!(found.as_ref().unwrap().y, 1.1);
}

#[test]
fn test_tree_full_scenario_from_c_test() {
    // Mirrors the C test_tree function exactly
    let tree: Quadtree<i32> = Quadtree::quadtree_new(1.0, 1.0, 10.0, 10.0);
    let root = tree.root.as_ref().unwrap();
    let bounds = root.bounds.as_ref().unwrap();
    assert_eq!(bounds.nw.as_ref().unwrap().x, 1.0);
    assert_eq!(bounds.nw.as_ref().unwrap().y, 10.0);
    assert_eq!(bounds.se.as_ref().unwrap().x, 10.0);
    assert_eq!(bounds.se.as_ref().unwrap().y, 1.0);

    assert!(!tree.quadtree_insert(0.0, 0.0, Some(10)));
    assert!(!tree.quadtree_insert(110.0, 110.0, Some(10)));

    assert!(tree.quadtree_insert(8.0, 2.0, Some(10)));
    assert_eq!(tree.length, 1);
    assert_eq!(tree.root.as_ref().unwrap().point.as_ref().unwrap().x, 8.0);
    assert_eq!(tree.root.as_ref().unwrap().point.as_ref().unwrap().y, 2.0);

    assert!(!tree.quadtree_insert(0.0, 1.0, Some(10))); // failed
    assert!(tree.quadtree_insert(2.0, 3.0, Some(10)));   // normal
    assert!(tree.quadtree_insert(2.0, 3.0, Some(10)));   // replace
    assert_eq!(tree.length, 2);
    assert!(tree.root.as_ref().unwrap().point.is_none());

    assert!(tree.quadtree_insert(3.0, 1.1, Some(10)));
    assert_eq!(tree.length, 3);
    let found = tree.quadtree_search(3.0, 1.1);
    assert!(found.is_some());
    assert_eq!(found.as_ref().unwrap().x, 3.0);
}

#[test]
fn test_tree_walk() {
    use std::sync::atomic::{AtomicU32, Ordering};
    static DESCENT_COUNT: AtomicU32 = AtomicU32::new(0);
    static ASCENT_COUNT: AtomicU32 = AtomicU32::new(0);

    DESCENT_COUNT.store(0, Ordering::SeqCst);
    ASCENT_COUNT.store(0, Ordering::SeqCst);

    fn descent(_node: &mut Option<Box<QuadtreeNode<i32>>>) {
        DESCENT_COUNT.fetch_add(1, Ordering::SeqCst);
    }
    fn ascent(_node: &mut Option<Box<QuadtreeNode<i32>>>) {
        ASCENT_COUNT.fetch_add(1, Ordering::SeqCst);
    }

    let tree: Quadtree<i32> = Quadtree::quadtree_new(0.0, 0.0, 10.0, 10.0);
    tree.quadtree_insert(1.0, 1.0, Some(1));
    tree.quadtree_insert(9.0, 9.0, Some(2));
    // After 2 inserts, root splits: root + 4 children = 5 nodes
    tree.quadtree_walk(descent, ascent);
    let d = DESCENT_COUNT.load(Ordering::SeqCst);
    let a = ASCENT_COUNT.load(Ordering::SeqCst);
    assert_eq!(d, a);
    assert!(d >= 5); // at least root + 4 quadrants
}

#[test]
fn test_tree_free() {
    let mut tree: Quadtree<i32> = Quadtree::quadtree_new(0.0, 0.0, 10.0, 10.0);
    tree.quadtree_insert(5.0, 5.0, Some(1));
    tree.quadtree_free();
    assert!(tree.root.is_none());
}

fn main() {}
