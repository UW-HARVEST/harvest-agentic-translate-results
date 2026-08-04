use quadtree::quadtree::quadtree::{Quadtree, QuadtreeBounds, QuadtreeNode, QuadtreePoint};
use quadtree::quadtree::elision_;

// --------------------- QuadtreePoint ---------------------

#[test]
fn test_point_new() {
    let p = QuadtreePoint::quadtree_point_new(5.0, 6.0);
    assert_eq!(p.x, 5.0);
    assert_eq!(p.y, 6.0);
}

#[test]
fn test_point_new_zero() {
    let p = QuadtreePoint::quadtree_point_new(0.0, 0.0);
    assert_eq!(p.x, 0.0);
    assert_eq!(p.y, 0.0);
}

#[test]
fn test_point_new_negative() {
    let p = QuadtreePoint::quadtree_point_new(-3.5, -7.25);
    assert_eq!(p.x, -3.5);
    assert_eq!(p.y, -7.25);
}

#[test]
fn test_point_free_noop() {
    let p = QuadtreePoint::quadtree_point_new(1.0, 2.0);
    p.quadtree_point_free();
    // After free, the original should still be valid in Rust semantics.
    assert_eq!(p.x, 1.0);
    assert_eq!(p.y, 2.0);
}

// --------------------- QuadtreeBounds ---------------------

#[test]
fn test_bounds_new() {
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
fn test_bounds_extend_first() {
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
fn test_bounds_extend_two() {
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

#[test]
fn test_bounds_extend_three() {
    let b = QuadtreeBounds::quadtree_bounds_new();
    b.quadtree_bounds_extend(5.0, 5.0);
    b.quadtree_bounds_extend(10.0, 10.0);
    b.quadtree_bounds_extend(1.0, 0.0);
    let nw = b.nw.as_ref().unwrap();
    let se = b.se.as_ref().unwrap();
    assert_eq!(nw.x, 1.0);
    assert_eq!(nw.y, 10.0);
    assert_eq!(se.x, 10.0);
    assert_eq!(se.y, 0.0);
    assert_eq!(b.width, 9.0);
    assert_eq!(b.height, 10.0);
}

#[test]
fn test_bounds_free_noop() {
    let b = QuadtreeBounds::quadtree_bounds_new();
    b.quadtree_bounds_free();
    // Should still be valid afterward in Rust.
    assert!(b.nw.is_some());
    assert!(b.se.is_some());
}

// --------------------- QuadtreeNode ---------------------

#[test]
fn test_node_new() {
    let n: QuadtreeNode<i32> = QuadtreeNode::quadtree_node_new();
    assert!(n.ne.is_none());
    assert!(n.nw.is_none());
    assert!(n.se.is_none());
    assert!(n.sw.is_none());
    assert!(n.bounds.is_none());
    assert!(n.point.is_none());
    assert!(n.key.is_none());
    assert_eq!(n.quadtree_node_isleaf(), false);
    assert_eq!(n.quadtree_node_isempty(), true);
    assert_eq!(n.quadtree_node_ispointer(), false);
}

#[test]
fn test_node_with_bounds() {
    let n: QuadtreeNode<i32> = QuadtreeNode::quadtree_node_with_bounds(0.0, 0.0, 10.0, 10.0);
    let b = n.bounds.as_ref().unwrap();
    let nw = b.nw.as_ref().unwrap();
    let se = b.se.as_ref().unwrap();
    assert_eq!(nw.x, 0.0);
    assert_eq!(nw.y, 10.0);
    assert_eq!(se.x, 10.0);
    assert_eq!(se.y, 0.0);
    assert_eq!(b.width, 10.0);
    assert_eq!(b.height, 10.0);
    assert_eq!(n.quadtree_node_isleaf(), false);
    assert_eq!(n.quadtree_node_isempty(), true);
    assert_eq!(n.quadtree_node_ispointer(), false);
    assert!(n.point.is_none());
    assert!(n.key.is_none());
    assert!(n.ne.is_none());
    assert!(n.nw.is_none());
    assert!(n.se.is_none());
    assert!(n.sw.is_none());
}

#[test]
fn test_node_with_bounds_negative() {
    let n: QuadtreeNode<i32> = QuadtreeNode::quadtree_node_with_bounds(-5.0, -5.0, 5.0, 5.0);
    let b = n.bounds.as_ref().unwrap();
    let nw = b.nw.as_ref().unwrap();
    let se = b.se.as_ref().unwrap();
    assert_eq!(nw.x, -5.0);
    assert_eq!(nw.y, 5.0);
    assert_eq!(se.x, 5.0);
    assert_eq!(se.y, -5.0);
    assert_eq!(b.width, 10.0);
    assert_eq!(b.height, 10.0);
}

#[test]
fn test_node_isleaf_with_point() {
    let mut n: QuadtreeNode<i32> = QuadtreeNode::quadtree_node_new();
    n.point = Some(Box::new(QuadtreePoint::quadtree_point_new(1.0, 2.0)));
    assert_eq!(n.quadtree_node_isleaf(), true);
    assert_eq!(n.quadtree_node_isempty(), false);
    assert_eq!(n.quadtree_node_ispointer(), false);
}

#[test]
fn test_node_ispointer_with_children() {
    let mut n: QuadtreeNode<i32> = QuadtreeNode::quadtree_node_new();
    n.nw = Some(Box::new(QuadtreeNode::quadtree_node_new()));
    n.ne = Some(Box::new(QuadtreeNode::quadtree_node_new()));
    n.sw = Some(Box::new(QuadtreeNode::quadtree_node_new()));
    n.se = Some(Box::new(QuadtreeNode::quadtree_node_new()));
    assert_eq!(n.quadtree_node_isleaf(), false);
    assert_eq!(n.quadtree_node_isempty(), false);
    assert_eq!(n.quadtree_node_ispointer(), true);
}

#[test]
fn test_node_partial_children_not_pointer() {
    // Only some quadrants assigned -> not a pointer.
    let mut n: QuadtreeNode<i32> = QuadtreeNode::quadtree_node_new();
    n.nw = Some(Box::new(QuadtreeNode::quadtree_node_new()));
    n.ne = Some(Box::new(QuadtreeNode::quadtree_node_new()));
    assert_eq!(n.quadtree_node_ispointer(), false);
    assert_eq!(n.quadtree_node_isempty(), false);
    assert_eq!(n.quadtree_node_isleaf(), false);
}

fn local_elision(_key: Option<i32>) {}

#[test]
fn test_node_reset_clears_point_and_key() {
    let mut n: QuadtreeNode<i32> = QuadtreeNode::quadtree_node_new();
    n.point = Some(Box::new(QuadtreePoint::quadtree_point_new(3.0, 4.0)));
    n.key = Some(7);
    n.quadtree_node_reset(Some(local_elision));
    assert!(n.point.is_none());
    assert!(n.key.is_none());
}

#[test]
fn test_node_reset_with_no_callback() {
    let mut n: QuadtreeNode<i32> = QuadtreeNode::quadtree_node_new();
    n.point = Some(Box::new(QuadtreePoint::quadtree_point_new(3.0, 4.0)));
    n.key = Some(7);
    n.quadtree_node_reset(None);
    assert!(n.point.is_none());
    assert!(n.key.is_none());
}

#[test]
fn test_node_free_noop() {
    let n: QuadtreeNode<i32> = QuadtreeNode::quadtree_node_new();
    n.quadtree_node_free(None);
}

// --------------------- Quadtree ---------------------

#[test]
fn test_tree_new_root_bounds() {
    let t: Quadtree<i32> = Quadtree::quadtree_new(1.0, 1.0, 10.0, 10.0);
    let root = t.root.as_ref().unwrap();
    let b = root.bounds.as_ref().unwrap();
    let nw = b.nw.as_ref().unwrap();
    let se = b.se.as_ref().unwrap();
    assert_eq!(nw.x, 1.0);
    assert_eq!(nw.y, 10.0);
    assert_eq!(se.x, 10.0);
    assert_eq!(se.y, 1.0);
    assert_eq!(b.width, 9.0);
    assert_eq!(b.height, 9.0);
    assert_eq!(t.length, 0);
    assert!(t.key_free.is_none());
}

#[test]
fn test_insert_out_of_bounds_below() {
    let t: Quadtree<i32> = Quadtree::quadtree_new(1.0, 1.0, 10.0, 10.0);
    let r = t.quadtree_insert(0.0, 0.0, Some(10));
    assert_eq!(r, false);
    assert_eq!(t.length, 0);
}

#[test]
fn test_insert_out_of_bounds_above() {
    let t: Quadtree<i32> = Quadtree::quadtree_new(1.0, 1.0, 10.0, 10.0);
    let r = t.quadtree_insert(110.0, 110.0, Some(10));
    assert_eq!(r, false);
    assert_eq!(t.length, 0);
}

#[test]
fn test_insert_first_point() {
    let t: Quadtree<i32> = Quadtree::quadtree_new(1.0, 1.0, 10.0, 10.0);
    let r = t.quadtree_insert(8.0, 2.0, Some(10));
    assert_eq!(r, true);
    assert_eq!(t.length, 1);
    let root = t.root.as_ref().unwrap();
    let p = root.point.as_ref().unwrap();
    assert_eq!(p.x, 8.0);
    assert_eq!(p.y, 2.0);
}

#[test]
fn test_insert_partially_out_of_bounds() {
    // C: insert(0.0, 1.0) returns 0 because x=0 is below minx=1.
    let t: Quadtree<i32> = Quadtree::quadtree_new(1.0, 1.0, 10.0, 10.0);
    assert!(t.quadtree_insert(8.0, 2.0, Some(10)));
    let r = t.quadtree_insert(0.0, 1.0, Some(11));
    assert_eq!(r, false);
    assert_eq!(t.length, 1);
}

#[test]
fn test_insert_split_and_replacement() {
    let t: Quadtree<i32> = Quadtree::quadtree_new(1.0, 1.0, 10.0, 10.0);
    assert!(t.quadtree_insert(8.0, 2.0, Some(10)));
    // Normal insertion - causes split since (2,3) is in different quadrant
    let r1 = t.quadtree_insert(2.0, 3.0, Some(11));
    assert_eq!(r1, true);
    assert_eq!(t.length, 2);
    // Replacement of (2,3)
    let r2 = t.quadtree_insert(2.0, 3.0, Some(12));
    assert_eq!(r2, true);
    assert_eq!(t.length, 2);
    // After splitting, the root point is None.
    let root = t.root.as_ref().unwrap();
    assert!(root.point.is_none());
    // After split, root should be a pointer.
    assert!(root.quadtree_node_ispointer());
}

#[test]
fn test_insert_third_point() {
    let t: Quadtree<i32> = Quadtree::quadtree_new(1.0, 1.0, 10.0, 10.0);
    assert!(t.quadtree_insert(8.0, 2.0, Some(10)));
    assert!(t.quadtree_insert(2.0, 3.0, Some(11)));
    assert!(t.quadtree_insert(2.0, 3.0, Some(12))); // replacement
    let r = t.quadtree_insert(3.0, 1.1, Some(13));
    assert_eq!(r, true);
    assert_eq!(t.length, 3);
}

#[test]
fn test_search_existing() {
    let t: Quadtree<i32> = Quadtree::quadtree_new(1.0, 1.0, 10.0, 10.0);
    assert!(t.quadtree_insert(8.0, 2.0, Some(10)));
    assert!(t.quadtree_insert(2.0, 3.0, Some(11)));
    assert!(t.quadtree_insert(3.0, 1.1, Some(13)));
    let result = t.quadtree_search(3.0, 1.1);
    let p = result.as_ref().expect("Expected to find a point");
    assert_eq!(p.x, 3.0);
    assert_eq!(p.y, 1.1);
}

#[test]
fn test_search_existing_root_leaf() {
    let t: Quadtree<i32> = Quadtree::quadtree_new(1.0, 1.0, 10.0, 10.0);
    assert!(t.quadtree_insert(8.0, 2.0, Some(10)));
    let result = t.quadtree_search(8.0, 2.0);
    let p = result.as_ref().expect("Expected to find a point");
    assert_eq!(p.x, 8.0);
    assert_eq!(p.y, 2.0);
}

#[test]
fn test_search_nonexistent_returns_none() {
    let t: Quadtree<i32> = Quadtree::quadtree_new(1.0, 1.0, 10.0, 10.0);
    assert!(t.quadtree_insert(8.0, 2.0, Some(10)));
    assert!(t.quadtree_insert(2.0, 3.0, Some(11)));
    let result = t.quadtree_search(100.0, 100.0);
    assert!(result.is_none());
}

#[test]
fn test_search_wrong_point_in_leaf() {
    let t: Quadtree<i32> = Quadtree::quadtree_new(1.0, 1.0, 10.0, 10.0);
    assert!(t.quadtree_insert(8.0, 2.0, Some(10)));
    let result = t.quadtree_search(5.0, 5.0);
    assert!(result.is_none());
}

#[test]
fn test_walk_no_panic() {
    fn descent<T>(_n: &mut Option<Box<QuadtreeNode<T>>>) {}
    fn ascent<T>(_n: &mut Option<Box<QuadtreeNode<T>>>) {}

    let t: Quadtree<i32> = Quadtree::quadtree_new(1.0, 1.0, 10.0, 10.0);
    assert!(t.quadtree_insert(8.0, 2.0, Some(10)));
    assert!(t.quadtree_insert(2.0, 3.0, Some(11)));
    assert!(t.quadtree_insert(3.0, 1.1, Some(13)));
    t.quadtree_walk(descent::<i32>, ascent::<i32>);
}

#[test]
fn test_free_clears_root() {
    let mut t: Quadtree<i32> = Quadtree::quadtree_new(1.0, 1.0, 10.0, 10.0);
    assert!(t.quadtree_insert(8.0, 2.0, Some(10)));
    t.quadtree_free();
    assert!(t.root.is_none());
    assert_eq!(t.length, 0);
}

#[test]
fn test_boundary_insertion() {
    // C allows insertion at exact boundary (e.g., 0,0 with bounds (0,0)-(10,10)).
    let t: Quadtree<i32> = Quadtree::quadtree_new(0.0, 0.0, 10.0, 10.0);
    assert_eq!(t.quadtree_insert(0.0, 0.0, Some(1)), true);
    assert_eq!(t.length, 1);
    assert_eq!(t.quadtree_insert(10.0, 10.0, Some(2)), true);
    assert_eq!(t.length, 2);
    assert_eq!(t.quadtree_insert(0.0, 10.0, Some(3)), true);
    assert_eq!(t.length, 3);
    assert_eq!(t.quadtree_insert(10.0, 0.0, Some(4)), true);
    assert_eq!(t.length, 4);
    assert_eq!(t.quadtree_insert(5.0, 5.0, Some(5)), true);
    assert_eq!(t.length, 5);
}

#[test]
fn test_split_creates_correct_subbounds() {
    // After inserting 2 points causing a split, verify the structure.
    let t: Quadtree<i32> = Quadtree::quadtree_new(0.0, 0.0, 10.0, 10.0);
    // First point in NE quadrant of root: (8, 8)
    assert!(t.quadtree_insert(8.0, 8.0, Some(1)));
    // Second point in SW quadrant of root: (2, 2). This will split root.
    assert!(t.quadtree_insert(2.0, 2.0, Some(2)));
    assert_eq!(t.length, 2);
    let root = t.root.as_ref().unwrap();
    assert!(root.quadtree_node_ispointer());
    // Root NW: x in [0, 5], y in [5, 10]
    let nw_child = root.nw.as_ref().unwrap();
    let nw_b = nw_child.bounds.as_ref().unwrap();
    assert_eq!(nw_b.nw.as_ref().unwrap().x, 0.0);
    assert_eq!(nw_b.nw.as_ref().unwrap().y, 10.0);
    assert_eq!(nw_b.se.as_ref().unwrap().x, 5.0);
    assert_eq!(nw_b.se.as_ref().unwrap().y, 5.0);
    assert_eq!(nw_b.width, 5.0);
    assert_eq!(nw_b.height, 5.0);

    // Root NE: x in [5, 10], y in [5, 10] - has the (8,8) leaf
    let ne_child = root.ne.as_ref().unwrap();
    let ne_b = ne_child.bounds.as_ref().unwrap();
    assert_eq!(ne_b.nw.as_ref().unwrap().x, 5.0);
    assert_eq!(ne_b.nw.as_ref().unwrap().y, 10.0);
    assert_eq!(ne_b.se.as_ref().unwrap().x, 10.0);
    assert_eq!(ne_b.se.as_ref().unwrap().y, 5.0);
    assert!(ne_child.quadtree_node_isleaf());
    let p = ne_child.point.as_ref().unwrap();
    assert_eq!(p.x, 8.0);
    assert_eq!(p.y, 8.0);

    // Root SW: x in [0, 5], y in [0, 5] - has the (2,2) leaf
    let sw_child = root.sw.as_ref().unwrap();
    let sw_b = sw_child.bounds.as_ref().unwrap();
    assert_eq!(sw_b.nw.as_ref().unwrap().x, 0.0);
    assert_eq!(sw_b.nw.as_ref().unwrap().y, 5.0);
    assert_eq!(sw_b.se.as_ref().unwrap().x, 5.0);
    assert_eq!(sw_b.se.as_ref().unwrap().y, 0.0);
    assert!(sw_child.quadtree_node_isleaf());
    let p = sw_child.point.as_ref().unwrap();
    assert_eq!(p.x, 2.0);
    assert_eq!(p.y, 2.0);

    // Root SE: x in [5, 10], y in [0, 5] - empty
    let se_child = root.se.as_ref().unwrap();
    assert!(se_child.quadtree_node_isempty());
}

#[test]
fn test_search_after_split() {
    let t: Quadtree<i32> = Quadtree::quadtree_new(0.0, 0.0, 10.0, 10.0);
    assert!(t.quadtree_insert(8.0, 8.0, Some(1)));
    assert!(t.quadtree_insert(2.0, 2.0, Some(2)));
    let r1 = t.quadtree_search(8.0, 8.0);
    let p1 = r1.as_ref().unwrap();
    assert_eq!(p1.x, 8.0);
    assert_eq!(p1.y, 8.0);
    let r2 = t.quadtree_search(2.0, 2.0);
    let p2 = r2.as_ref().unwrap();
    assert_eq!(p2.x, 2.0);
    assert_eq!(p2.y, 2.0);
}

// --------------------- elision_ ---------------------

#[test]
fn test_elision() {
    // Just verify the function runs and accepts a key.
    elision_::<i32>(Some(Box::new(42)));
    elision_::<i32>(None);
}

fn main() {}
