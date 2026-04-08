use kd3::kdtree::{KDTree, KDTreeIterator};

fn collect_sorted(iter: &mut Option<KDTreeIterator>) -> Vec<usize> {
    let it = iter.as_mut().unwrap();
    let mut results = Vec::new();
    while let Some(v) = it.get_next() {
        results.push(v);
    }
    results.sort();
    results
}

// === KDTree::new ===

#[test]
fn test_new_tree_defaults() {
    let tree = KDTree::new();
    assert_eq!(tree.count, 0);
    assert_eq!(tree.max_nodes, 0);
    assert_eq!(tree.next_node, 0);
    assert!(tree.root.is_none());
    assert!(tree.points.is_empty());
    assert!(tree.node_data.is_empty());
}

// === KDTree::build ===

#[test]
fn test_build_two_points() {
    let mut tree = KDTree::new();
    let mut x = vec![0.0, 1.0];
    let mut y = vec![0.0, 1.0];
    let mut z = vec![0.0, 1.0];
    tree.build(&mut x, &mut y, &mut z, 2);
    assert_eq!(tree.count, 2);
    assert!(tree.root.is_some());
}

#[test]
fn test_build_sets_max_nodes() {
    let mut tree = KDTree::new();
    let mut x = vec![0.0, 1.0, 2.0];
    let mut y = vec![0.0, 1.0, 2.0];
    let mut z = vec![0.0, 1.0, 2.0];
    tree.build(&mut x, &mut y, &mut z, 3);
    // max_nodes = (count-1)*2 + 1 = 5
    assert_eq!(tree.max_nodes, 5);
}

#[test]
#[should_panic]
fn test_build_count_one_panics() {
    let mut tree = KDTree::new();
    let mut x = vec![0.0];
    let mut y = vec![0.0];
    let mut z = vec![0.0];
    tree.build(&mut x, &mut y, &mut z, 1);
}

#[test]
fn test_rebuild_different_count() {
    let mut tree = KDTree::new();
    let mut x = vec![0.0, 1.0];
    let mut y = vec![0.0, 1.0];
    let mut z = vec![0.0, 1.0];
    tree.build(&mut x, &mut y, &mut z, 2);
    assert_eq!(tree.count, 2);

    let mut x2 = vec![10.0, 20.0, 30.0];
    let mut y2 = vec![10.0, 20.0, 30.0];
    let mut z2 = vec![10.0, 20.0, 30.0];
    tree.build(&mut x2, &mut y2, &mut z2, 3);
    assert_eq!(tree.count, 3);
    assert_eq!(tree.max_nodes, 5);
}

// === KDTree::search ===

#[test]
fn test_search_two_points_find_both() {
    let mut tree = KDTree::new();
    let mut x = vec![0.0, 1.0];
    let mut y = vec![0.0, 1.0];
    let mut z = vec![0.0, 1.0];
    tree.build(&mut x, &mut y, &mut z, 2);

    let mut iter: Option<KDTreeIterator> = None;
    tree.search(&mut iter, 0.5, 0.5, 0.5, 0.5);
    assert_eq!(collect_sorted(&mut iter), vec![0, 1]);
}

#[test]
fn test_search_exact_point_apothem_zero() {
    let mut tree = KDTree::new();
    let mut x = vec![0.0, 1.0];
    let mut y = vec![0.0, 1.0];
    let mut z = vec![0.0, 1.0];
    tree.build(&mut x, &mut y, &mut z, 2);

    let mut iter: Option<KDTreeIterator> = None;
    tree.search(&mut iter, 0.0, 0.0, 0.0, 0.0);
    assert_eq!(collect_sorted(&mut iter), vec![0]);
}

#[test]
fn test_search_large_apothem() {
    let mut tree = KDTree::new();
    let mut x = vec![0.0, 1.0];
    let mut y = vec![0.0, 1.0];
    let mut z = vec![0.0, 1.0];
    tree.build(&mut x, &mut y, &mut z, 2);

    let mut iter: Option<KDTreeIterator> = None;
    tree.search(&mut iter, 0.5, 0.5, 0.5, 100.0);
    assert_eq!(collect_sorted(&mut iter), vec![0, 1]);
}

#[test]
fn test_search_no_match() {
    let mut tree = KDTree::new();
    let mut x = vec![0.0, 1.0];
    let mut y = vec![0.0, 1.0];
    let mut z = vec![0.0, 1.0];
    tree.build(&mut x, &mut y, &mut z, 2);

    let mut iter: Option<KDTreeIterator> = None;
    tree.search(&mut iter, 5.0, 5.0, 5.0, 0.1);
    assert_eq!(collect_sorted(&mut iter), vec![]);
}

#[test]
fn test_search_duplicate_points() {
    let mut tree = KDTree::new();
    let mut x = vec![5.0, 5.0, 5.0];
    let mut y = vec![5.0, 5.0, 5.0];
    let mut z = vec![5.0, 5.0, 5.0];
    tree.build(&mut x, &mut y, &mut z, 3);

    let mut iter: Option<KDTreeIterator> = None;
    tree.search(&mut iter, 5.0, 5.0, 5.0, 0.0);
    assert_eq!(collect_sorted(&mut iter), vec![0, 1, 2]);
}

#[test]
fn test_search_negative_coords() {
    let mut tree = KDTree::new();
    let mut x = vec![-1.0, -2.0, 1.0, 2.0];
    let mut y = vec![-1.0, -2.0, 1.0, 2.0];
    let mut z = vec![-1.0, -2.0, 1.0, 2.0];
    tree.build(&mut x, &mut y, &mut z, 4);

    let mut iter: Option<KDTreeIterator> = None;
    tree.search(&mut iter, 0.0, 0.0, 0.0, 1.5);
    assert_eq!(collect_sorted(&mut iter), vec![0, 2]);

    tree.search(&mut iter, -1.5, -1.5, -1.5, 1.0);
    assert_eq!(collect_sorted(&mut iter), vec![0, 1]);
}

#[test]
fn test_search_large_dataset() {
    let mut tree = KDTree::new();
    let n = 100usize;
    let mut x: Vec<f64> = (0..n).map(|i| i as f64).collect();
    let mut y: Vec<f64> = (0..n).map(|i| (i % 10) as f64).collect();
    let mut z: Vec<f64> = (0..n).map(|i| (i / 10) as f64).collect();
    tree.build(&mut x, &mut y, &mut z, n);

    let mut iter: Option<KDTreeIterator> = None;
    // Point 50: x=50, y=0, z=5
    tree.search(&mut iter, 50.0, 0.0, 5.0, 0.5);
    assert_eq!(collect_sorted(&mut iter), vec![50]);

    tree.search(&mut iter, 0.0, 0.0, 0.0, 0.0);
    assert_eq!(collect_sorted(&mut iter), vec![0]);

    tree.search(&mut iter, 5.0, 5.0, 0.0, 0.5);
    assert_eq!(collect_sorted(&mut iter), vec![5]);
}

// === 11-point test from C run_test.c ===

#[test]
fn test_11_point_match_none() {
    let mut tree = KDTree::new();
    let (mut x, mut y, mut z) = eleven_points();
    tree.build(&mut x, &mut y, &mut z, 11);

    let mut iter: Option<KDTreeIterator> = None;
    tree.search(&mut iter, -10.0, 0.0, 0.0, 9.999);
    assert_eq!(collect_sorted(&mut iter), vec![]);
}

#[test]
fn test_11_point_match_one() {
    let mut tree = KDTree::new();
    let (mut x, mut y, mut z) = eleven_points();
    tree.build(&mut x, &mut y, &mut z, 11);

    let mut iter: Option<KDTreeIterator> = None;
    tree.search(&mut iter, 0.0, 0.0, 0.0, 0.499);
    assert_eq!(collect_sorted(&mut iter), vec![3]);
}

#[test]
fn test_11_point_match_all() {
    let mut tree = KDTree::new();
    let (mut x, mut y, mut z) = eleven_points();
    tree.build(&mut x, &mut y, &mut z, 11);

    let mut iter: Option<KDTreeIterator> = None;
    tree.search(&mut iter, 0.5, 0.5, 0.5, 0.5);
    assert_eq!(collect_sorted(&mut iter), vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);

    tree.search(&mut iter, 0.5, 0.5, 0.5, 100.0);
    assert_eq!(collect_sorted(&mut iter), vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
}

#[test]
fn test_11_point_front_slice() {
    let mut tree = KDTree::new();
    let (mut x, mut y, mut z) = eleven_points();
    tree.build(&mut x, &mut y, &mut z, 11);

    let mut iter: Option<KDTreeIterator> = None;
    tree.search(&mut iter, 0.5, 0.5, 0.0, 0.5);
    assert_eq!(collect_sorted(&mut iter), vec![0, 1, 2, 3, 4, 5, 6]);
}

#[test]
fn test_11_point_back_slice() {
    let mut tree = KDTree::new();
    let (mut x, mut y, mut z) = eleven_points();
    tree.build(&mut x, &mut y, &mut z, 11);

    let mut iter: Option<KDTreeIterator> = None;
    tree.search(&mut iter, 0.5, 0.5, 1.0, 0.5);
    assert_eq!(collect_sorted(&mut iter), vec![0, 1, 2, 7, 8, 9, 10]);
}

// === KDTree::delete ===

#[test]
fn test_delete_clears_tree() {
    let mut tree = KDTree::new();
    let mut x = vec![0.0, 1.0];
    let mut y = vec![0.0, 1.0];
    let mut z = vec![0.0, 1.0];
    tree.build(&mut x, &mut y, &mut z, 2);
    tree.delete();
    assert_eq!(tree.count, 0);
    assert!(tree.root.is_none());
    assert!(tree.points.is_empty());
    assert!(tree.node_data.is_empty());
}

// === KDTreeIterator ===

#[test]
fn test_iterator_new() {
    let iter = KDTreeIterator::new();
    assert_eq!(iter.size, 0);
    assert_eq!(iter.current, 0);
    assert_eq!(iter.capacity, 50); // KDTREE_ITERATOR_INITIAL_SIZE
}

#[test]
fn test_iterator_get_next_empty() {
    let mut iter = KDTreeIterator::new();
    assert_eq!(iter.get_next(), None);
}

#[test]
fn test_iterator_push_and_get() {
    let mut iter = KDTreeIterator::new();
    iter.push(42);
    iter.push(99);
    assert_eq!(iter.size, 2);
    assert_eq!(iter.get_next(), Some(42));
    assert_eq!(iter.get_next(), Some(99));
    assert_eq!(iter.get_next(), None);
}

#[test]
fn test_iterator_reset() {
    let mut iter = KDTreeIterator::new();
    iter.push(1);
    iter.push(2);
    assert_eq!(iter.size, 2);
    iter.reset();
    assert_eq!(iter.size, 0);
    assert_eq!(iter.current, 0);
}

// === Iterator rewind via search ===

#[test]
fn test_iterator_rewind_via_search() {
    let mut tree = KDTree::new();
    let mut x = vec![0.0, 1.0];
    let mut y = vec![0.0, 1.0];
    let mut z = vec![0.0, 1.0];
    tree.build(&mut x, &mut y, &mut z, 2);

    let mut iter: Option<KDTreeIterator> = None;
    tree.search(&mut iter, 0.5, 0.5, 0.5, 0.5);

    let it = iter.as_mut().unwrap();
    let v1 = it.get_next();
    let v2 = it.get_next();
    let v3 = it.get_next();
    assert!(v1.is_some());
    assert!(v2.is_some());
    assert_eq!(v3, None); // end of iteration

    // Rewind and iterate again — should get same results
    it.rewind();
    let r1 = it.get_next();
    let r2 = it.get_next();
    let r3 = it.get_next();
    assert_eq!(v1, r1);
    assert_eq!(v2, r2);
    assert_eq!(r3, None);
}

// === Iterator sort via search ===

#[test]
fn test_iterator_sort_via_search() {
    let mut tree = KDTree::new();
    let mut x = vec![3.0, 1.0, 2.0, 0.0];
    let mut y = vec![0.0, 0.0, 0.0, 0.0];
    let mut z = vec![0.0, 0.0, 0.0, 0.0];
    tree.build(&mut x, &mut y, &mut z, 4);

    let mut iter: Option<KDTreeIterator> = None;
    tree.search(&mut iter, 1.5, 0.0, 0.0, 2.0);

    let it = iter.as_mut().unwrap();
    assert_eq!(it.size, 4);

    // Collect unsorted
    let mut unsorted = Vec::new();
    while let Some(v) = it.get_next() {
        unsorted.push(v);
    }
    // C ground truth: unsorted order is [3, 1, 2, 0]
    assert_eq!(unsorted, vec![3, 1, 2, 0]);

    // Sort and collect
    it.sort();
    it.rewind();
    let mut sorted = Vec::new();
    while let Some(v) = it.get_next() {
        sorted.push(v);
    }
    assert_eq!(sorted, vec![0, 1, 2, 3]);
}

// === Iterator reuse across searches ===

#[test]
fn test_iterator_reuse() {
    let mut tree = KDTree::new();
    let mut x = vec![0.0, 1.0, 2.0];
    let mut y = vec![0.0, 0.0, 0.0];
    let mut z = vec![0.0, 0.0, 0.0];
    tree.build(&mut x, &mut y, &mut z, 3);

    let mut iter: Option<KDTreeIterator> = None;

    // First search
    tree.search(&mut iter, 0.0, 0.0, 0.0, 0.5);
    assert_eq!(collect_sorted(&mut iter), vec![0]);

    // Second search reuses iterator
    tree.search(&mut iter, 1.0, 0.0, 0.0, 0.5);
    assert_eq!(collect_sorted(&mut iter), vec![1]);
}

// === Rebuild and search ===

#[test]
fn test_rebuild_and_search() {
    let mut tree = KDTree::new();
    let mut x = vec![0.0, 1.0];
    let mut y = vec![0.0, 1.0];
    let mut z = vec![0.0, 1.0];
    tree.build(&mut x, &mut y, &mut z, 2);

    let mut iter: Option<KDTreeIterator> = None;
    tree.search(&mut iter, 0.0, 0.0, 0.0, 0.0);
    assert_eq!(collect_sorted(&mut iter), vec![0]);

    // Rebuild with different data
    let mut x2 = vec![10.0, 20.0, 30.0];
    let mut y2 = vec![10.0, 20.0, 30.0];
    let mut z2 = vec![10.0, 20.0, 30.0];
    tree.build(&mut x2, &mut y2, &mut z2, 3);

    tree.search(&mut iter, 20.0, 20.0, 20.0, 0.1);
    assert_eq!(collect_sorted(&mut iter), vec![1]);

    tree.search(&mut iter, 20.0, 20.0, 20.0, 15.0);
    assert_eq!(collect_sorted(&mut iter), vec![0, 1, 2]);
}

// === Helper ===

fn eleven_points() -> (Vec<f64>, Vec<f64>, Vec<f64>) {
    let x = vec![0.5, 0.5, 0.5, 0.0, 1.0, 1.0, 0.0, 0.0, 1.0, 1.0, 0.0];
    let y = vec![0.5, 0.5, 0.5, 0.0, 0.0, 1.0, 1.0, 0.0, 0.0, 1.0, 1.0];
    let z = vec![0.5, 0.5, 0.5, 0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 1.0];
    (x, y, z)
}

fn main() {}
