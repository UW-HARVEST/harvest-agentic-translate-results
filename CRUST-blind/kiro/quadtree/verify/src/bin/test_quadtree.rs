use quadtree::quadtree::quadtree::*;

#[test]
fn test_point_new() {
    let p = QuadtreePoint::quadtree_point_new(5.0, 6.0);
    assert_eq!(p.x, 5.0);
    assert_eq!(p.y, 6.0);
}

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
fn test_bounds_extend() {
    let b = QuadtreeBounds::quadtree_bounds_new();
    b.quadtree_bounds_extend(5.0, 5.0);
    assert_eq!(b.nw.as_ref().unwrap().x, 5.0);
    assert_eq!(b.nw.as_ref().unwrap().y, 5.0);
    assert_eq!(b.se.as_ref().unwrap().x, 5.0);
    assert_eq!(b.se.as_ref().unwrap().y, 5.0);
    assert_eq!(b.width, 0.0);
    assert_eq!(b.height, 0.0);

    b.quadtree_bounds_extend(10.0, 10.0);
    assert_eq!(b.nw.as_ref().unwrap().x, 5.0);
    assert_eq!(b.nw.as_ref().unwrap().y, 10.0);
    assert_eq!(b.se.as_ref().unwrap().x, 10.0);
    assert_eq!(b.se.as_ref().unwrap().y, 5.0);
    assert_eq!(b.width, 5.0);
    assert_eq!(b.height, 5.0);
}

#[test]
fn test_node_new() {
    let node: QuadtreeNode<i32> = QuadtreeNode::quadtree_node_new();
    assert!(!node.quadtree_node_isleaf());
    assert!(node.quadtree_node_isempty());
    assert!(!node.quadtree_node_ispointer());
}

#[test]
fn test_node_with_bounds() {
    let node: QuadtreeNode<i32> = QuadtreeNode::quadtree_node_with_bounds(0.0, 0.0, 10.0, 10.0);
    let bounds = node.bounds.as_ref().unwrap();
    assert_eq!(bounds.nw.as_ref().unwrap().x, 0.0);
    assert_eq!(bounds.nw.as_ref().unwrap().y, 10.0);
    assert_eq!(bounds.se.as_ref().unwrap().x, 10.0);
    assert_eq!(bounds.se.as_ref().unwrap().y, 0.0);
    assert_eq!(bounds.width, 10.0);
    assert_eq!(bounds.height, 10.0);
}

#[test]
fn test_tree_new_bounds() {
    let tree: Quadtree<i32> = Quadtree::quadtree_new(1.0, 1.0, 10.0, 10.0);
    let root = tree.root.as_ref().unwrap();
    let bounds = root.bounds.as_ref().unwrap();
    assert_eq!(bounds.nw.as_ref().unwrap().x, 1.0);
    assert_eq!(bounds.nw.as_ref().unwrap().y, 10.0);
    assert_eq!(bounds.se.as_ref().unwrap().x, 10.0);
    assert_eq!(bounds.se.as_ref().unwrap().y, 1.0);
    assert_eq!(tree.length, 0);
}

#[test]
fn test_insert_out_of_bounds() {
    let tree: Quadtree<i32> = Quadtree::quadtree_new(1.0, 1.0, 10.0, 10.0);
    assert!(!tree.quadtree_insert(0.0, 0.0, Some(10)));
    assert_eq!(tree.length, 0);
    assert!(!tree.quadtree_insert(110.0, 110.0, Some(10)));
    assert_eq!(tree.length, 0);
}

#[test]
fn test_insert_first_point() {
    let tree: Quadtree<i32> = Quadtree::quadtree_new(1.0, 1.0, 10.0, 10.0);
    assert!(tree.quadtree_insert(8.0, 2.0, Some(10)));
    assert_eq!(tree.length, 1);
    let root = tree.root.as_ref().unwrap();
    let pt = root.point.as_ref().unwrap();
    assert_eq!(pt.x, 8.0);
    assert_eq!(pt.y, 2.0);
}

#[test]
fn test_insert_boundary_fail() {
    let tree: Quadtree<i32> = Quadtree::quadtree_new(1.0, 1.0, 10.0, 10.0);
    tree.quadtree_insert(8.0, 2.0, Some(10));
    assert!(!tree.quadtree_insert(0.0, 1.0, Some(10)));
    assert_eq!(tree.length, 1);
}

#[test]
fn test_insert_split_and_duplicate() {
    let tree: Quadtree<i32> = Quadtree::quadtree_new(1.0, 1.0, 10.0, 10.0);
    tree.quadtree_insert(8.0, 2.0, Some(10));

    assert!(tree.quadtree_insert(2.0, 3.0, Some(10)));
    assert_eq!(tree.length, 2);
    assert!(tree.root.as_ref().unwrap().point.is_none());

    assert!(tree.quadtree_insert(2.0, 3.0, Some(10)));
    assert_eq!(tree.length, 2);
}

#[test]
fn test_insert_third_and_search() {
    let tree: Quadtree<i32> = Quadtree::quadtree_new(1.0, 1.0, 10.0, 10.0);
    tree.quadtree_insert(8.0, 2.0, Some(10));
    tree.quadtree_insert(2.0, 3.0, Some(10));
    assert!(tree.quadtree_insert(3.0, 1.1, Some(10)));
    assert_eq!(tree.length, 3);

    let found = tree.quadtree_search(3.0, 1.1);
    assert!(found.is_some());
    let pt = found.as_ref().unwrap();
    assert_eq!(pt.x, 3.0);
    assert_eq!(pt.y, 1.1);
}

#[test]
fn test_search_not_found() {
    let tree: Quadtree<i32> = Quadtree::quadtree_new(1.0, 1.0, 10.0, 10.0);
    tree.quadtree_insert(8.0, 2.0, Some(10));
    let result = tree.quadtree_search(999.0, 999.0);
    assert!(result.is_none());
}

#[test]
fn test_walk() {
    let tree: Quadtree<i32> = Quadtree::quadtree_new(1.0, 1.0, 10.0, 10.0);
    tree.quadtree_insert(8.0, 2.0, Some(10));
    tree.quadtree_insert(2.0, 3.0, Some(10));
    fn descent(_node: &mut Option<Box<QuadtreeNode<i32>>>) {}
    fn ascent(_node: &mut Option<Box<QuadtreeNode<i32>>>) {}
    tree.quadtree_walk(descent, ascent);
}

#[test]
fn test_free() {
    let mut tree: Quadtree<i32> = Quadtree::quadtree_new(1.0, 1.0, 10.0, 10.0);
    tree.quadtree_insert(8.0, 2.0, Some(10));
    tree.quadtree_free();
    assert!(tree.root.is_none());
}

fn main() {}
