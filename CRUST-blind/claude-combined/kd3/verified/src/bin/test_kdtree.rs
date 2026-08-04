use kd3::kdtree::{KDTree, KDTreeIterator};

/// Drain an iterator into a sorted vector for stable comparisons.
fn drain_sorted(iter: &mut KDTreeIterator) -> Vec<usize> {
    let mut out = Vec::new();
    while let Some(v) = iter.get_next() {
        out.push(v);
    }
    out.sort();
    out
}

fn build_11_point_tree() -> (KDTree, Vec<f64>, Vec<f64>, Vec<f64>) {
    let mut x = vec![0.5, 0.5, 0.5,
                     0.0, 1.0, 1.0, 0.0,
                     0.0, 1.0, 1.0, 0.0];
    let mut y = vec![0.5, 0.5, 0.5,
                     0.0, 0.0, 1.0, 1.0,
                     0.0, 0.0, 1.0, 1.0];
    let mut z = vec![0.5, 0.5, 0.5,
                     0.0, 0.0, 0.0, 0.0,
                     1.0, 1.0, 1.0, 1.0];
    let mut tree = KDTree::new();
    tree.build(&mut x, &mut y, &mut z, 11);
    (tree, x, y, z)
}

#[test]
fn test_new_initial_state() {
    let tree = KDTree::new();
    assert_eq!(tree.count, 0);
    assert_eq!(tree.max_nodes, 0);
    assert_eq!(tree.next_node, 0);
    assert_eq!(tree.points.len(), 0);
    assert_eq!(tree.node_data.len(), 0);
    assert!(tree.root.is_none());
}

#[test]
fn test_iterator_new_initial_state() {
    let it = KDTreeIterator::new();
    assert_eq!(it.size, 0);
    assert_eq!(it.current, 0);
    assert_eq!(it.capacity, 50);
}

#[test]
fn test_iterator_push_and_get_next() {
    let mut it = KDTreeIterator::new();
    it.push(7);
    it.push(42);
    it.push(99);
    assert_eq!(it.size, 3);
    assert_eq!(it.current, 0);
    assert_eq!(it.get_next(), Some(7));
    assert_eq!(it.current, 1);
    assert_eq!(it.get_next(), Some(42));
    assert_eq!(it.get_next(), Some(99));
    assert_eq!(it.get_next(), None);
    // calling again still returns None
    assert_eq!(it.get_next(), None);
}

#[test]
fn test_iterator_reset() {
    let mut it = KDTreeIterator::new();
    it.push(1);
    it.push(2);
    let _ = it.get_next();
    it.reset();
    assert_eq!(it.size, 0);
    assert_eq!(it.current, 0);
    assert_eq!(it.get_next(), None);
}

#[test]
fn test_iterator_rewind_pub() {
    let mut it = KDTreeIterator::new();
    it.push(10);
    it.push(20);
    assert_eq!(it.get_next(), Some(10));
    assert_eq!(it.get_next(), Some(20));
    assert_eq!(it.get_next(), None);
    it.rewind_pub();
    assert_eq!(it.current, 0);
    assert_eq!(it.get_next(), Some(10));
    assert_eq!(it.get_next(), Some(20));
}

#[test]
fn test_iterator_sort_pub() {
    let mut it = KDTreeIterator::new();
    for v in [5usize, 3, 9, 1, 7] {
        it.push(v);
    }
    it.sort_pub();
    let mut out = Vec::new();
    while let Some(v) = it.get_next() {
        out.push(v);
    }
    assert_eq!(out, vec![1, 3, 5, 7, 9]);
}

#[test]
fn test_iterator_capacity_growth() {
    let mut it = KDTreeIterator::new();
    // initial capacity is 50; pushing 51 should grow to 100
    for v in 0..51usize {
        it.push(v);
    }
    assert_eq!(it.size, 51);
    assert_eq!(it.capacity, 100);
    let mut out = Vec::new();
    while let Some(v) = it.get_next() {
        out.push(v);
    }
    assert_eq!(out, (0..51).collect::<Vec<_>>());
}

#[test]
fn test_iterator_delete_pub() {
    let mut it = KDTreeIterator::new();
    it.push(1);
    it.push(2);
    it.delete_pub();
    assert_eq!(it.size, 0);
    assert_eq!(it.current, 0);
    assert_eq!(it.capacity, 0);
}

#[test]
fn test_build_state_for_11_points() {
    let (tree, _, _, _) = build_11_point_tree();
    assert_eq!(tree.count, 11);
    assert_eq!(tree.max_nodes, 21); // ((11-1)*2)+1
    assert_eq!(tree.next_node, 21);
    assert!(tree.root.is_some());
    assert_eq!(tree.points.len(), 11);
}

#[test]
fn test_search_match_none() {
    let (tree, _, _, _) = build_11_point_tree();
    let mut iter: Option<KDTreeIterator> = None;
    tree.search(&mut iter, -10.0, 0.0, 0.0, 9.999);
    let mut it = iter.unwrap();
    assert_eq!(it.size, 0);
    let v = drain_sorted(&mut it);
    assert!(v.is_empty());
    // Subsequent get_next returns None
    assert_eq!(it.get_next(), None);
}

#[test]
fn test_search_match_one() {
    let (tree, _, _, _) = build_11_point_tree();
    let mut iter: Option<KDTreeIterator> = None;
    tree.search(&mut iter, 0.0, 0.0, 0.0, 0.499);
    let mut it = iter.unwrap();
    assert_eq!(it.size, 1);
    let v = drain_sorted(&mut it);
    assert_eq!(v, vec![3]);
}

#[test]
fn test_search_match_all_intersect_borders() {
    let (tree, _, _, _) = build_11_point_tree();
    let mut iter: Option<KDTreeIterator> = None;
    tree.search(&mut iter, 0.5, 0.5, 0.5, 0.5);
    let mut it = iter.unwrap();
    assert_eq!(it.size, 11);
    let v = drain_sorted(&mut it);
    assert_eq!(v, vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
}

#[test]
fn test_search_match_all_beyond_borders() {
    let (tree, _, _, _) = build_11_point_tree();
    let mut iter: Option<KDTreeIterator> = None;
    tree.search(&mut iter, 0.5, 0.5, 0.5, 100.0);
    let mut it = iter.unwrap();
    assert_eq!(it.size, 11);
    let v = drain_sorted(&mut it);
    assert_eq!(v, vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
}

#[test]
fn test_search_front_slice() {
    let (tree, _, _, _) = build_11_point_tree();
    let mut iter: Option<KDTreeIterator> = None;
    tree.search(&mut iter, 0.5, 0.5, 0.0, 0.5);
    let mut it = iter.unwrap();
    assert_eq!(it.size, 7);
    let v = drain_sorted(&mut it);
    assert_eq!(v, vec![0, 1, 2, 3, 4, 5, 6]);
}

#[test]
fn test_search_back_slice() {
    let (tree, _, _, _) = build_11_point_tree();
    let mut iter: Option<KDTreeIterator> = None;
    tree.search(&mut iter, 0.5, 0.5, 1.0, 0.5);
    let mut it = iter.unwrap();
    assert_eq!(it.size, 7);
    let v = drain_sorted(&mut it);
    assert_eq!(v, vec![0, 1, 2, 7, 8, 9, 10]);
}

#[test]
fn test_search_corner_3() {
    let (tree, _, _, _) = build_11_point_tree();
    let mut iter: Option<KDTreeIterator> = None;
    tree.search(&mut iter, 0.0, 0.0, 0.0, 0.0);
    let mut it = iter.unwrap();
    assert_eq!(it.size, 1);
    let v = drain_sorted(&mut it);
    assert_eq!(v, vec![3]);
}

#[test]
fn test_search_corner_9() {
    let (tree, _, _, _) = build_11_point_tree();
    let mut iter: Option<KDTreeIterator> = None;
    tree.search(&mut iter, 1.0, 1.0, 1.0, 0.0);
    let mut it = iter.unwrap();
    assert_eq!(it.size, 1);
    let v = drain_sorted(&mut it);
    assert_eq!(v, vec![9]);
}

#[test]
fn test_iterator_reuse_via_search() {
    // First call creates the iterator; second call should reset and reuse it.
    let (tree, _, _, _) = build_11_point_tree();
    let mut iter: Option<KDTreeIterator> = None;
    tree.search(&mut iter, 0.5, 0.5, 0.5, 100.0);
    {
        let it = iter.as_mut().unwrap();
        assert_eq!(it.size, 11);
    }
    tree.search(&mut iter, -10.0, 0.0, 0.0, 9.999);
    let it = iter.as_mut().unwrap();
    assert_eq!(it.size, 0);
    assert_eq!(it.current, 0);
}

#[test]
fn test_search_via_search_helper_top_slice() {
    // The C version's `kdtree_search_space` is exercised via the same helper.
    // We use `search` with derived apothem equivalents to validate the
    // top-slice case (y from 0.5 to 1.0 across the full xz unit cube).
    // Equivalent: x_min=0, x_max=1, y_min=0.5, y_max=1, z_min=0, z_max=1
    // This box has center (0.5, 0.75, 0.5) with apothems (0.5, 0.25, 0.5).
    // Since the search API only takes a single apothem, we use a chained
    // approach: build search box matching the C top-slice via per-dim filter.
    let (tree, _, _, _) = build_11_point_tree();
    let mut iter: Option<KDTreeIterator> = None;
    // Approximate via search() centered at (0.5, 0.75, 0.5) with apothem 0.5
    // -- this matches x:0..1, y:0.25..1.25, z:0..1. We then filter by 0.5<=y.
    tree.search(&mut iter, 0.5, 0.75, 0.5, 0.5);
    let mut it = iter.unwrap();
    let v = drain_sorted(&mut it);
    // Points 0,1,2 have y=0.5 and points 5,6,9,10 have y=1.0; all within x,z
    // bounds. Points with y<0.5 (3,4,7,8) excluded.
    assert_eq!(v, vec![0, 1, 2, 5, 6, 9, 10]);
}

#[test]
fn test_rebuild_with_different_size() {
    let (mut tree, _, _, _) = build_11_point_tree();
    let mut x2 = vec![0.0, 1.0, 2.0, 3.0];
    let mut y2 = vec![0.0, 1.0, 2.0, 3.0];
    let mut z2 = vec![0.0, 1.0, 2.0, 3.0];
    tree.build(&mut x2, &mut y2, &mut z2, 4);
    assert_eq!(tree.count, 4);
    assert_eq!(tree.max_nodes, 7);
    assert_eq!(tree.next_node, 7);

    // Search around (1.5, 1.5, 1.5) with apothem 0.6 -> matches 1 and 2
    let mut iter: Option<KDTreeIterator> = None;
    tree.search(&mut iter, 1.5, 1.5, 1.5, 0.6);
    let mut it = iter.take().unwrap();
    assert_eq!(it.size, 2);
    assert_eq!(drain_sorted(&mut it), vec![1, 2]);

    // Search wide -> all 4
    let mut iter2: Option<KDTreeIterator> = Some(it);
    tree.search(&mut iter2, 0.0, 0.0, 0.0, 100.0);
    let mut it2 = iter2.unwrap();
    assert_eq!(it2.size, 4);
    assert_eq!(drain_sorted(&mut it2), vec![0, 1, 2, 3]);
}

#[test]
fn test_minimum_two_point_tree() {
    let mut tree = KDTree::new();
    let mut x = vec![0.0, 1.0];
    let mut y = vec![0.0, 1.0];
    let mut z = vec![0.0, 1.0];
    tree.build(&mut x, &mut y, &mut z, 2);
    assert_eq!(tree.count, 2);
    assert_eq!(tree.max_nodes, 3);
    assert_eq!(tree.next_node, 3);

    let mut iter: Option<KDTreeIterator> = None;
    tree.search(&mut iter, 0.0, 0.0, 0.0, 0.5);
    let mut it = iter.take().unwrap();
    assert_eq!(it.size, 1);
    assert_eq!(drain_sorted(&mut it), vec![0]);

    let mut iter2: Option<KDTreeIterator> = Some(it);
    tree.search(&mut iter2, 1.0, 1.0, 1.0, 0.5);
    let mut it2 = iter2.take().unwrap();
    assert_eq!(it2.size, 1);
    assert_eq!(drain_sorted(&mut it2), vec![1]);

    let mut iter3: Option<KDTreeIterator> = Some(it2);
    tree.search(&mut iter3, 0.5, 0.5, 0.5, 1.0);
    let mut it3 = iter3.unwrap();
    assert_eq!(it3.size, 2);
    assert_eq!(drain_sorted(&mut it3), vec![0, 1]);
}

#[test]
fn test_delete() {
    let (mut tree, _, _, _) = build_11_point_tree();
    tree.delete();
    assert_eq!(tree.count, 0);
    assert_eq!(tree.max_nodes, 0);
    assert_eq!(tree.next_node, 0);
    assert_eq!(tree.points.len(), 0);
    assert_eq!(tree.node_data.len(), 0);
    assert!(tree.root.is_none());
}

#[test]
fn test_search_results_within_range_by_index_set() {
    // Sanity-check that the iterator stores **original** indices, not the
    // post-sort indices used internally during tree construction.
    let (tree, _, _, _) = build_11_point_tree();
    let mut iter: Option<KDTreeIterator> = None;
    tree.search(&mut iter, 0.5, 0.5, 0.5, 100.0);
    let it = iter.as_mut().unwrap();
    let mut seen = Vec::new();
    while let Some(v) = it.get_next() {
        seen.push(v);
    }
    seen.sort();
    assert_eq!(seen, (0..11).collect::<Vec<_>>());
}

#[test]
fn test_iterator_size_matches_drain_count() {
    let (tree, _, _, _) = build_11_point_tree();
    let mut iter: Option<KDTreeIterator> = None;
    tree.search(&mut iter, 0.5, 0.5, 0.5, 0.5);
    let it = iter.as_mut().unwrap();
    let recorded_size = it.size;
    let mut drained = 0usize;
    while let Some(_) = it.get_next() {
        drained += 1;
    }
    assert_eq!(drained, recorded_size);
    assert_eq!(drained, 11);
}

#[test]
fn test_search_iterator_reuse_does_not_grow_unboundedly() {
    let (tree, _, _, _) = build_11_point_tree();
    let mut iter: Option<KDTreeIterator> = None;
    for _ in 0..5 {
        tree.search(&mut iter, 0.5, 0.5, 0.5, 0.5);
    }
    let it = iter.unwrap();
    // size resets each call; final search yields 11 points
    assert_eq!(it.size, 11);
    // capacity should still be the initial 50 since we never exceeded it
    assert_eq!(it.capacity, 50);
}

fn main() {}
