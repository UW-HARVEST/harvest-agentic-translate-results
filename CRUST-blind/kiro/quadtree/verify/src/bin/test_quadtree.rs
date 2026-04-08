use quadtree::quadtree::quadtree::*;

// --- Point tests ---

#[test]
fn test_point_new() {
    let p = QuadtreePoint::quadtree_point_new(5.0, 6.0);
    assert_eq!(p.x, 5.0);
    assert_eq!(p.y, 6.0);
}

// --- Bounds tests ---

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
fn test_bounds_extend_single() {
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

// --- Node tests ---

#[test]
fn test_node_new_state() {
    let node = QuadtreeNode::<i32>::quadtree_node_new();
    assert!(!node.quadtree_node_isleaf());
    assert!(node.quadtree_node_isempty());
    assert!(!node.quadtree_node_ispointer());
}

#[test]
fn test_node_with_bounds() {
    let node = QuadtreeNode::<i32>::quadtree_node_with_bounds(1.0, 1.0, 10.0, 10.0);
    let bounds = node.bounds.as_ref().unwrap();
    let nw = bounds.nw.as_ref().unwrap();
    let se = bounds.se.as_ref().unwrap();
    assert_eq!(nw.x, 1.0);
    assert_eq!(nw.y, 10.0);
    assert_eq!(se.x, 10.0);
    assert_eq!(se.y, 1.0);
    assert_eq!(bounds.width, 9.0);
    assert_eq!(bounds.height, 9.0);
    assert!(!node.quadtree_node_isleaf());
    assert!(node.quadtree_node_isempty());
    assert!(!node.quadtree_node_ispointer());
}

// --- Quadtree insert/search tests ---

#[test]
fn test_tree_new_bounds() {
    let tree = Quadtree::<i32>::quadtree_new(1.0, 1.0, 10.0, 10.0);
    let root = tree.root.as_ref().unwrap();
    let bounds = root.bounds.as_ref().unwrap();
    let nw = bounds.nw.as_ref().unwrap();
    let se = bounds.se.as_ref().unwrap();
    assert_eq!(nw.x, 1.0);
    assert_eq!(nw.y, 10.0);
    assert_eq!(se.x, 10.0);
    assert_eq!(se.y, 1.0);
    assert_eq!(tree.length, 0);
}

#[test]
fn test_insert_outside_bounds() {
    let tree = Quadtree::<i32>::quadtree_new(1.0, 1.0, 10.0, 10.0);
    // C: insert(0,0)=0, insert(110,110)=0
    assert_eq!(tree.quadtree_insert(0.0, 0.0, Some(10)), false);
    assert_eq!(tree.length, 0);
    assert_eq!(tree.quadtree_insert(110.0, 110.0, Some(10)), false);
    assert_eq!(tree.length, 0);
}

#[test]
fn test_insert_first_point() {
    let tree = Quadtree::<i32>::quadtree_new(1.0, 1.0, 10.0, 10.0);
    // C: insert(8,2)=1 len=1
    assert_eq!(tree.quadtree_insert(8.0, 2.0, Some(10)), true);
    assert_eq!(tree.length, 1);
    let root = tree.root.as_ref().unwrap();
    let pt = root.point.as_ref().unwrap();
    assert_eq!(pt.x, 8.0);
    assert_eq!(pt.y, 2.0);
}

#[test]
fn test_insert_outside_lower_boundary() {
    let tree = Quadtree::<i32>::quadtree_new(1.0, 1.0, 10.0, 10.0);
    tree.quadtree_insert(8.0, 2.0, Some(10));
    // C: insert(0,1)=0 (x=0 < nw.x=1)
    assert_eq!(tree.quadtree_insert(0.0, 1.0, Some(10)), false);
    assert_eq!(tree.length, 1);
}

#[test]
fn test_insert_causes_split() {
    let tree = Quadtree::<i32>::quadtree_new(1.0, 1.0, 10.0, 10.0);
    tree.quadtree_insert(8.0, 2.0, Some(10));
    // C: insert(2,3)=1 len=2, root becomes pointer
    assert_eq!(tree.quadtree_insert(2.0, 3.0, Some(10)), true);
    assert_eq!(tree.length, 2);
    let root = tree.root.as_ref().unwrap();
    assert!(root.quadtree_node_ispointer());
    assert!(!root.quadtree_node_isleaf());
    assert!(root.point.is_none());
}

#[test]
fn test_insert_replacement() {
    let tree = Quadtree::<i32>::quadtree_new(1.0, 1.0, 10.0, 10.0);
    tree.quadtree_insert(8.0, 2.0, Some(10));
    tree.quadtree_insert(2.0, 3.0, Some(10));
    // C: insert(2,3 again)=2 len=2 (replacement, length unchanged)
    assert_eq!(tree.quadtree_insert(2.0, 3.0, Some(20)), true);
    assert_eq!(tree.length, 2);
}

#[test]
fn test_insert_third_point() {
    let tree = Quadtree::<i32>::quadtree_new(1.0, 1.0, 10.0, 10.0);
    tree.quadtree_insert(8.0, 2.0, Some(10));
    tree.quadtree_insert(2.0, 3.0, Some(10));
    // C: insert(3,1.1)=1 len=3
    assert_eq!(tree.quadtree_insert(3.0, 1.1, Some(10)), true);
    assert_eq!(tree.length, 3);
}

#[test]
fn test_search_found() {
    let tree = Quadtree::<i32>::quadtree_new(1.0, 1.0, 10.0, 10.0);
    tree.quadtree_insert(8.0, 2.0, Some(10));
    tree.quadtree_insert(2.0, 3.0, Some(10));
    tree.quadtree_insert(3.0, 1.1, Some(10));

    // C: search(3,1.1) x=3.0 y=1.1
    let r = tree.quadtree_search(3.0, 1.1);
    assert!(r.is_some());
    let pt = r.as_ref().unwrap();
    assert_eq!(pt.x, 3.0);
    assert_eq!(pt.y, 1.1);

    // C: search(8,2) x=8.0 y=2.0
    let r2 = tree.quadtree_search(8.0, 2.0);
    assert!(r2.is_some());
    let pt2 = r2.as_ref().unwrap();
    assert_eq!(pt2.x, 8.0);
    assert_eq!(pt2.y, 2.0);
}

#[test]
fn test_search_not_found() {
    let tree = Quadtree::<i32>::quadtree_new(1.0, 1.0, 10.0, 10.0);
    tree.quadtree_insert(8.0, 2.0, Some(10));
    // C: search(999,999) null=1
    let r = tree.quadtree_search(999.0, 999.0);
    assert!(r.is_none());
}

#[test]
fn test_insert_at_boundaries() {
    let tree = Quadtree::<i32>::quadtree_new(1.0, 1.0, 10.0, 10.0);
    tree.quadtree_insert(8.0, 2.0, Some(10));
    tree.quadtree_insert(2.0, 3.0, Some(10));
    tree.quadtree_insert(3.0, 1.1, Some(10));
    // C: insert(1,1)=1 len=4
    assert_eq!(tree.quadtree_insert(1.0, 1.0, Some(10)), true);
    assert_eq!(tree.length, 4);
    // C: insert(10,10)=1 len=5
    assert_eq!(tree.quadtree_insert(10.0, 10.0, Some(10)), true);
    assert_eq!(tree.length, 5);
}

#[test]
fn test_replacement_no_length_change() {
    // Fresh tree to test replacement in isolation
    // C: t2 insert(50,50)=1 len=1; insert(50,50 again)=2 len=1
    let tree = Quadtree::<i32>::quadtree_new(0.0, 0.0, 100.0, 100.0);
    assert_eq!(tree.quadtree_insert(50.0, 50.0, Some(1)), true);
    assert_eq!(tree.length, 1);
    assert_eq!(tree.quadtree_insert(50.0, 50.0, Some(2)), true);
    assert_eq!(tree.length, 1);
}

#[test]
fn test_walk() {
    use std::sync::atomic::{AtomicU32, Ordering};
    static DESCENT_COUNT: AtomicU32 = AtomicU32::new(0);
    static ASCENT_COUNT: AtomicU32 = AtomicU32::new(0);

    fn descent(_node: &mut Option<Box<QuadtreeNode<i32>>>) {
        DESCENT_COUNT.fetch_add(1, Ordering::SeqCst);
    }
    fn ascent(_node: &mut Option<Box<QuadtreeNode<i32>>>) {
        ASCENT_COUNT.fetch_add(1, Ordering::SeqCst);
    }

    DESCENT_COUNT.store(0, Ordering::SeqCst);
    ASCENT_COUNT.store(0, Ordering::SeqCst);

    let tree = Quadtree::<i32>::quadtree_new(1.0, 1.0, 10.0, 10.0);
    // Single root node, no children
    tree.quadtree_walk(descent, ascent);
    assert_eq!(DESCENT_COUNT.load(Ordering::SeqCst), 1);
    assert_eq!(ASCENT_COUNT.load(Ordering::SeqCst), 1);
}

#[test]
fn test_quadtree_free() {
    let mut tree = Quadtree::<i32>::quadtree_new(1.0, 1.0, 10.0, 10.0);
    tree.quadtree_insert(5.0, 5.0, Some(10));
    tree.quadtree_free();
    assert!(tree.root.is_none());
}

#[test]
fn test_node_isleaf_after_insert() {
    let tree = Quadtree::<i32>::quadtree_new(1.0, 1.0, 10.0, 10.0);
    tree.quadtree_insert(5.0, 5.0, Some(10));
    let root = tree.root.as_ref().unwrap();
    assert!(root.quadtree_node_isleaf());
    assert!(!root.quadtree_node_isempty());
    assert!(!root.quadtree_node_ispointer());
}

fn main() {}
