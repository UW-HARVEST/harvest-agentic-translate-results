use kd3::kdtree::{KDTree, KDTreeIterator};

fn build_tree() -> KDTree {
    let mut x = vec![0.5, 0.5, 0.5, 0.0, 1.0, 1.0, 0.0, 0.0, 1.0, 1.0, 0.0];
    let mut y = vec![0.5, 0.5, 0.5, 0.0, 0.0, 1.0, 1.0, 0.0, 0.0, 1.0, 1.0];
    let mut z = vec![0.5, 0.5, 0.5, 0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 1.0];
    let mut tree = KDTree::new();
    tree.build(&mut x, &mut y, &mut z, 11);
    tree
}

fn collect_sorted(iter: &mut KDTreeIterator) -> Vec<usize> {
    iter.rewind();
    iter.sort();
    iter.rewind();
    let mut v = Vec::new();
    while let Some(val) = iter.get_next() {
        v.push(val);
    }
    v
}

// --- KDTree tests ---

#[test]
fn test_build_tree_fields() {
    let tree = build_tree();
    assert_eq!(tree.count, 11);
    assert_eq!(tree.max_nodes, 21);
    assert!(tree.root.is_some());
}

#[test]
fn test_search_match_none() {
    let tree = build_tree();
    let mut iter: Option<KDTreeIterator> = None;
    tree.search(&mut iter, -10.0, 0.0, 0.0, 9.999);
    let it = iter.as_mut().unwrap();
    assert_eq!(it.size, 0);
    assert_eq!(it.get_next(), None);
}

#[test]
fn test_search_match_one() {
    let tree = build_tree();
    let mut iter: Option<KDTreeIterator> = None;
    tree.search(&mut iter, 0.0, 0.0, 0.0, 0.499);
    let it = iter.as_mut().unwrap();
    assert_eq!(it.size, 1);
    let sorted = collect_sorted(it);
    assert_eq!(sorted, vec![3]);
}

#[test]
fn test_search_match_all_border() {
    let tree = build_tree();
    let mut iter: Option<KDTreeIterator> = None;
    tree.search(&mut iter, 0.5, 0.5, 0.5, 0.5);
    let it = iter.as_mut().unwrap();
    assert_eq!(it.size, 11);
    let sorted = collect_sorted(it);
    assert_eq!(sorted, vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
}

#[test]
fn test_search_match_all_beyond() {
    let tree = build_tree();
    let mut iter: Option<KDTreeIterator> = None;
    tree.search(&mut iter, 0.5, 0.5, 0.5, 100.0);
    let it = iter.as_mut().unwrap();
    assert_eq!(it.size, 11);
    let sorted = collect_sorted(it);
    assert_eq!(sorted, vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
}

#[test]
fn test_search_front_slice() {
    let tree = build_tree();
    let mut iter: Option<KDTreeIterator> = None;
    tree.search(&mut iter, 0.5, 0.5, 0.0, 0.5);
    let it = iter.as_mut().unwrap();
    assert_eq!(it.size, 7);
    let sorted = collect_sorted(it);
    assert_eq!(sorted, vec![0, 1, 2, 3, 4, 5, 6]);
}

#[test]
fn test_search_back_slice() {
    let tree = build_tree();
    let mut iter: Option<KDTreeIterator> = None;
    tree.search(&mut iter, 0.5, 0.5, 1.0, 0.5);
    let it = iter.as_mut().unwrap();
    assert_eq!(it.size, 7);
    let sorted = collect_sorted(it);
    assert_eq!(sorted, vec![0, 1, 2, 7, 8, 9, 10]);
}

#[test]
fn test_search_space_top_slice() {
    let tree = build_tree();
    let mut iter: Option<KDTreeIterator> = None;
    tree.search_space_iter(&mut iter, 0.0, 1.0, 0.5, 1.0, 0.0, 1.0);
    let it = iter.as_mut().unwrap();
    assert_eq!(it.size, 7);
    let sorted = collect_sorted(it);
    assert_eq!(sorted, vec![0, 1, 2, 5, 6, 9, 10]);
}

#[test]
fn test_search_exact_point() {
    let tree = build_tree();
    let mut iter: Option<KDTreeIterator> = None;
    tree.search(&mut iter, 1.0, 1.0, 1.0, 0.0);
    let it = iter.as_mut().unwrap();
    assert_eq!(it.size, 1);
    let sorted = collect_sorted(it);
    assert_eq!(sorted, vec![9]);
}

#[test]
fn test_search_space_tight_bounds() {
    let tree = build_tree();
    let mut iter: Option<KDTreeIterator> = None;
    tree.search_space_iter(&mut iter, 0.9, 1.1, -0.1, 0.1, -0.1, 0.1);
    let it = iter.as_mut().unwrap();
    assert_eq!(it.size, 1);
    let sorted = collect_sorted(it);
    assert_eq!(sorted, vec![4]);
}

#[test]
fn test_search_far_negative() {
    let tree = build_tree();
    let mut iter: Option<KDTreeIterator> = None;
    tree.search(&mut iter, -100.0, -100.0, -100.0, 0.1);
    let it = iter.as_mut().unwrap();
    assert_eq!(it.size, 0);
    assert_eq!(it.get_next(), None);
}

// --- Iterator reuse: search reuses existing iterator ---

#[test]
fn test_iterator_reuse_across_searches() {
    let tree = build_tree();
    let mut iter: Option<KDTreeIterator> = None;
    // First search
    tree.search(&mut iter, 0.0, 0.0, 0.0, 0.499);
    assert_eq!(iter.as_ref().unwrap().size, 1);
    // Second search reuses iterator
    tree.search(&mut iter, 0.5, 0.5, 0.5, 0.5);
    assert_eq!(iter.as_ref().unwrap().size, 11);
}

// --- KDTreeIterator tests ---

#[test]
fn test_iterator_new() {
    let it = KDTreeIterator::new();
    assert_eq!(it.size, 0);
    assert_eq!(it.current, 0);
    assert_eq!(it.capacity, 50);
}

#[test]
fn test_iterator_push_and_get_next() {
    let mut it = KDTreeIterator::new();
    it.push(42);
    it.push(7);
    it.push(99);
    assert_eq!(it.size, 3);
    assert_eq!(it.get_next(), Some(42));
    assert_eq!(it.get_next(), Some(7));
    assert_eq!(it.get_next(), Some(99));
    assert_eq!(it.get_next(), None);
}

#[test]
fn test_iterator_rewind() {
    let tree = build_tree();
    let mut iter: Option<KDTreeIterator> = None;
    tree.search(&mut iter, 0.0, 0.0, 0.0, 0.499);
    let it = iter.as_mut().unwrap();
    let first = it.get_next();
    assert_eq!(first, Some(3));
    it.rewind();
    let again = it.get_next();
    assert_eq!(again, Some(3));
}

#[test]
fn test_iterator_sort() {
    let mut it = KDTreeIterator::new();
    it.push(5);
    it.push(1);
    it.push(3);
    it.push(2);
    it.push(4);
    it.sort();
    it.rewind();
    let mut v = Vec::new();
    while let Some(val) = it.get_next() {
        v.push(val);
    }
    assert_eq!(v, vec![1, 2, 3, 4, 5]);
}

#[test]
fn test_iterator_reset() {
    let mut it = KDTreeIterator::new();
    it.push(10);
    it.push(20);
    assert_eq!(it.size, 2);
    it.reset();
    assert_eq!(it.size, 0);
    assert_eq!(it.current, 0);
    assert_eq!(it.get_next(), None);
}

#[test]
fn test_iterator_delete() {
    let mut it = KDTreeIterator::new();
    it.push(1);
    it.push(2);
    it.delete();
    assert_eq!(it.size, 0);
    assert_eq!(it.current, 0);
    assert_eq!(it.capacity, 0);
}

#[test]
fn test_iterator_get_next_empty() {
    let mut it = KDTreeIterator::new();
    assert_eq!(it.get_next(), None);
}

// --- KDTree::delete ---

#[test]
fn test_tree_delete() {
    let mut tree = build_tree();
    tree.delete();
    assert_eq!(tree.count, 0);
    assert_eq!(tree.max_nodes, 0);
    assert_eq!(tree.next_node, 0);
    assert!(tree.root.is_none());
    assert!(tree.points.is_empty());
    assert!(tree.node_data.is_empty());
}

// --- Two-point dataset ---

#[test]
fn test_two_point_tree() {
    let mut x = vec![0.0, 1.0];
    let mut y = vec![0.0, 1.0];
    let mut z = vec![0.0, 1.0];
    let mut tree = KDTree::new();
    tree.build(&mut x, &mut y, &mut z, 2);
    assert_eq!(tree.count, 2);
    assert_eq!(tree.max_nodes, 3);

    let mut iter: Option<KDTreeIterator> = None;
    tree.search(&mut iter, 0.5, 0.5, 0.5, 1.0);
    let it = iter.as_mut().unwrap();
    assert_eq!(it.size, 2);
    let sorted = collect_sorted(it);
    assert_eq!(sorted, vec![0, 1]);
}

#[test]
fn test_two_point_exact_origin() {
    let mut x = vec![0.0, 1.0];
    let mut y = vec![0.0, 1.0];
    let mut z = vec![0.0, 1.0];
    let mut tree = KDTree::new();
    tree.build(&mut x, &mut y, &mut z, 2);

    let mut iter: Option<KDTreeIterator> = None;
    tree.search(&mut iter, 0.0, 0.0, 0.0, 0.0);
    let it = iter.as_mut().unwrap();
    assert_eq!(it.size, 1);
    let sorted = collect_sorted(it);
    assert_eq!(sorted, vec![0]);
}

// --- Rebuild tree (reuse) ---

#[test]
fn test_rebuild_tree() {
    let mut x = vec![0.5, 0.5, 0.5, 0.0, 1.0, 1.0, 0.0, 0.0, 1.0, 1.0, 0.0];
    let mut y = vec![0.5, 0.5, 0.5, 0.0, 0.0, 1.0, 1.0, 0.0, 0.0, 1.0, 1.0];
    let mut z = vec![0.5, 0.5, 0.5, 0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 1.0];
    let mut tree = KDTree::new();
    tree.build(&mut x, &mut y, &mut z, 11);

    // Rebuild with same data
    tree.build(&mut x, &mut y, &mut z, 11);
    assert_eq!(tree.count, 11);

    let mut iter: Option<KDTreeIterator> = None;
    tree.search(&mut iter, 0.5, 0.5, 0.5, 0.5);
    let it = iter.as_mut().unwrap();
    assert_eq!(it.size, 11);
}

fn main() {}
