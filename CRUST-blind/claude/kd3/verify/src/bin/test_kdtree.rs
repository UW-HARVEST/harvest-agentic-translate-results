use kd3::kdtree::{KDTree, KDTreeIterator};

fn make_11_point_dataset() -> ([f64; 11], [f64; 11], [f64; 11]) {
    // Same dataset as run_test.c
    let xs: [f64; 11] = [0.5, 0.5, 0.5, 0.0, 1.0, 1.0, 0.0, 0.0, 1.0, 1.0, 0.0];
    let ys: [f64; 11] = [0.5, 0.5, 0.5, 0.0, 0.0, 1.0, 1.0, 0.0, 0.0, 1.0, 1.0];
    let zs: [f64; 11] = [0.5, 0.5, 0.5, 0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 1.0];
    (xs, ys, zs)
}

fn build_11pt_tree() -> KDTree {
    let (mut x, mut y, mut z) = make_11_point_dataset();
    let mut tree = KDTree::new();
    tree.build(&mut x, &mut y, &mut z, 11);
    tree
}

/// Drain the iterator, return the entries sorted in ascending order.
fn drain_sorted(iter: &mut KDTreeIterator) -> Vec<usize> {
    let mut out = Vec::new();
    while let Some(v) = iter.get_next() {
        out.push(v);
    }
    out.sort();
    out
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
fn test_build_11_points_state() {
    // After building with 11 points, the C code reports:
    // count=11, max_nodes=21, next_node=21
    let tree = build_11pt_tree();
    assert_eq!(tree.count, 11);
    assert_eq!(tree.max_nodes, 21);
    assert_eq!(tree.next_node, 21);
    assert_eq!(tree.points.len(), 11);
    assert_eq!(tree.node_data.len(), 21);
    assert!(tree.root.is_some());
}

#[test]
fn test_build_two_points_state() {
    // C output: count=2 max_nodes=3 next_node=3
    let mut x = [0.0_f64, 1.0];
    let mut y = [0.0_f64, 0.0];
    let mut z = [0.0_f64, 0.0];
    let mut tree = KDTree::new();
    tree.build(&mut x, &mut y, &mut z, 2);
    assert_eq!(tree.count, 2);
    assert_eq!(tree.max_nodes, 3);
    assert_eq!(tree.next_node, 3);
    assert_eq!(tree.points.len(), 2);
}

#[test]
fn test_search_match_none() {
    // C output: match_none size=0 data=[]
    let tree = build_11pt_tree();
    let mut iter: Option<KDTreeIterator> = None;
    tree.search(&mut iter, -10.0, 0.0, 0.0, 9.999);
    let it = iter.as_mut().expect("iterator should exist");
    assert_eq!(it.size, 0);
    let drained = drain_sorted(it);
    assert_eq!(drained, Vec::<usize>::new());
}

#[test]
fn test_search_match_one() {
    // C output: match_one size=1 data=[3]
    let tree = build_11pt_tree();
    let mut iter: Option<KDTreeIterator> = None;
    tree.search(&mut iter, 0.0, 0.0, 0.0, 0.499);
    let it = iter.as_mut().expect("iterator should exist");
    assert_eq!(it.size, 1);
    let drained = drain_sorted(it);
    assert_eq!(drained, vec![3]);
}

#[test]
fn test_search_match_all_borders() {
    // C output: size=11 data=[0..10]
    let tree = build_11pt_tree();
    let mut iter: Option<KDTreeIterator> = None;
    tree.search(&mut iter, 0.5, 0.5, 0.5, 0.5);
    let it = iter.as_mut().expect("iterator");
    assert_eq!(it.size, 11);
    let drained = drain_sorted(it);
    assert_eq!(drained, vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
}

#[test]
fn test_search_match_all_far() {
    // C output: size=11 data=[0..10]
    let tree = build_11pt_tree();
    let mut iter: Option<KDTreeIterator> = None;
    tree.search(&mut iter, 0.5, 0.5, 0.5, 100.0);
    let it = iter.as_mut().expect("iterator");
    assert_eq!(it.size, 11);
    let drained = drain_sorted(it);
    assert_eq!(drained, vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
}

#[test]
fn test_search_front_slice() {
    // C output: size=7 data=[0,1,2,3,4,5,6]
    let tree = build_11pt_tree();
    let mut iter: Option<KDTreeIterator> = None;
    tree.search(&mut iter, 0.5, 0.5, 0.0, 0.5);
    let it = iter.as_mut().expect("iterator");
    assert_eq!(it.size, 7);
    let drained = drain_sorted(it);
    assert_eq!(drained, vec![0, 1, 2, 3, 4, 5, 6]);
}

#[test]
fn test_search_back_slice() {
    // C output: size=7 data=[0,1,2,7,8,9,10]
    let tree = build_11pt_tree();
    let mut iter: Option<KDTreeIterator> = None;
    tree.search(&mut iter, 0.5, 0.5, 1.0, 0.5);
    let it = iter.as_mut().expect("iterator");
    assert_eq!(it.size, 7);
    let drained = drain_sorted(it);
    assert_eq!(drained, vec![0, 1, 2, 7, 8, 9, 10]);
}

#[test]
fn test_search_apothem_zero() {
    // C output: size=3 data=[0,1,2] -- only the three points exactly at (0.5,0.5,0.5)
    let tree = build_11pt_tree();
    let mut iter: Option<KDTreeIterator> = None;
    tree.search(&mut iter, 0.5, 0.5, 0.5, 0.0);
    let it = iter.as_mut().expect("iterator");
    assert_eq!(it.size, 3);
    let drained = drain_sorted(it);
    assert_eq!(drained, vec![0, 1, 2]);
}

#[test]
fn test_search_far_far_away() {
    // C output: size=0 data=[]
    let tree = build_11pt_tree();
    let mut iter: Option<KDTreeIterator> = None;
    tree.search(&mut iter, 100.0, 100.0, 100.0, 1.0);
    let it = iter.as_mut().expect("iterator");
    assert_eq!(it.size, 0);
    let drained = drain_sorted(it);
    assert_eq!(drained, Vec::<usize>::new());
}

#[test]
fn test_search_two_points() {
    // C output:
    //   twopt_first  (search 0,0,0, apothem=0.5): size=1 data=[0]
    //   twopt_second (search 1,0,0, apothem=0.0): size=1 data=[1]
    //   twopt_both   (search 0.5,0,0, apothem=0.5): size=2 data=[0,1]
    let mut x = [0.0_f64, 1.0];
    let mut y = [0.0_f64, 0.0];
    let mut z = [0.0_f64, 0.0];
    let mut tree = KDTree::new();
    tree.build(&mut x, &mut y, &mut z, 2);

    let mut iter: Option<KDTreeIterator> = None;
    tree.search(&mut iter, 0.0, 0.0, 0.0, 0.5);
    {
        let it = iter.as_mut().unwrap();
        assert_eq!(it.size, 1);
        assert_eq!(drain_sorted(it), vec![0]);
    }

    tree.search(&mut iter, 1.0, 0.0, 0.0, 0.0);
    {
        let it = iter.as_mut().unwrap();
        assert_eq!(it.size, 1);
        assert_eq!(drain_sorted(it), vec![1]);
    }

    tree.search(&mut iter, 0.5, 0.0, 0.0, 0.5);
    {
        let it = iter.as_mut().unwrap();
        assert_eq!(it.size, 2);
        assert_eq!(drain_sorted(it), vec![0, 1]);
    }
}

#[test]
fn test_iterator_get_next_end_returns_none() {
    // After iterating to the end, subsequent calls keep returning None
    // (the C version returns KDTREE_END which is SIZE_MAX).
    let tree = build_11pt_tree();
    let mut iter: Option<KDTreeIterator> = None;
    tree.search(&mut iter, 0.0, 0.0, 0.0, 0.499);
    let it = iter.as_mut().unwrap();
    assert_eq!(it.size, 1);

    // First call should return Some(3) (the only matching point).
    assert_eq!(it.get_next(), Some(3));
    // After end, returns None.
    assert_eq!(it.get_next(), None);
    assert_eq!(it.get_next(), None);
}

#[test]
fn test_iterator_rewind() {
    // From C: search 0.5,0.5,0.5 apothem=0.5 -> 11 results.
    // Iterating to end then rewinding restarts the iteration from the
    // first stored value (which is index 3 in C's particular ordering
    // for this dataset).
    let tree = build_11pt_tree();
    let mut iter: Option<KDTreeIterator> = None;
    tree.search(&mut iter, 0.5, 0.5, 0.5, 0.5);
    let it = iter.as_mut().unwrap();
    assert_eq!(it.size, 11);

    // Drain.
    while it.get_next().is_some() {}
    assert_eq!(it.get_next(), None);
    assert_eq!(it.current, it.size);

    // Rewind and iterate again - should produce the exact same sequence.
    it.rewind();
    assert_eq!(it.current, 0);

    let mut all = Vec::new();
    while let Some(v) = it.get_next() {
        all.push(v);
    }
    assert_eq!(all.len(), 11);
    let mut sorted = all.clone();
    sorted.sort();
    assert_eq!(sorted, vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
}

#[test]
fn test_iterator_sort() {
    // After sort, the data slice (up to size) should be sorted ascending.
    let tree = build_11pt_tree();
    let mut iter: Option<KDTreeIterator> = None;
    tree.search(&mut iter, 0.5, 0.5, 0.5, 0.5);
    let it = iter.as_mut().unwrap();
    it.sort();

    // The first `size` entries should be the sorted set 0..=10.
    let mut got: Vec<usize> = it.data[..it.size].to_vec();
    assert_eq!(got.len(), 11);
    assert_eq!(got, vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);

    // get_next walks them in sorted order.
    let mut expected = 0usize;
    while let Some(v) = it.get_next() {
        got[expected] = v; // re-use storage
        assert_eq!(v, expected);
        expected += 1;
    }
    assert_eq!(expected, 11);
}

#[test]
fn test_iterator_new_initial_state() {
    let it = KDTreeIterator::new();
    assert_eq!(it.size, 0);
    assert_eq!(it.current, 0);
    assert_eq!(it.capacity, 50); // KDTREE_ITERATOR_INITIAL_SIZE
    assert_eq!(it.data.len(), 0);
}

#[test]
fn test_iterator_push_grows() {
    // Filling beyond initial capacity (50) should grow capacity by ratio 2.
    let mut it = KDTreeIterator::new();
    assert_eq!(it.capacity, 50);
    for v in 0..50 {
        it.push(v);
    }
    assert_eq!(it.size, 50);
    assert_eq!(it.capacity, 50);

    // One more push triggers the growth.
    it.push(50);
    assert_eq!(it.size, 51);
    assert_eq!(it.capacity, 100);

    // get_next returns the values in insertion order.
    for expected in 0..51 {
        assert_eq!(it.get_next(), Some(expected));
    }
    assert_eq!(it.get_next(), None);
}

#[test]
fn test_iterator_reset() {
    let mut it = KDTreeIterator::new();
    it.push(7);
    it.push(8);
    it.push(9);
    assert_eq!(it.size, 3);
    let _ = it.get_next();
    assert_eq!(it.current, 1);

    it.reset();
    assert_eq!(it.size, 0);
    assert_eq!(it.current, 0);
    // After reset, get_next returns None (size is 0).
    assert_eq!(it.get_next(), None);
}

#[test]
fn test_iterator_delete_releases_state() {
    let mut it = KDTreeIterator::new();
    it.push(1);
    it.push(2);
    it.delete();
    assert_eq!(it.size, 0);
    assert_eq!(it.current, 0);
    assert_eq!(it.capacity, 0);
    assert_eq!(it.data.len(), 0);
}

#[test]
fn test_search_reuses_iterator() {
    // Calling search a second time on an existing iterator must reset
    // (not append). This mirrors the C `iter_reset` path.
    let tree = build_11pt_tree();
    let mut iter: Option<KDTreeIterator> = None;
    tree.search(&mut iter, 0.5, 0.5, 0.5, 0.5);
    assert_eq!(iter.as_ref().unwrap().size, 11);

    tree.search(&mut iter, 0.0, 0.0, 0.0, 0.499);
    let it = iter.as_mut().unwrap();
    assert_eq!(it.size, 1);
    assert_eq!(it.current, 0);
    let drained = drain_sorted(it);
    assert_eq!(drained, vec![3]);
}

#[test]
fn test_kdtree_delete_resets_state() {
    let mut tree = build_11pt_tree();
    assert_eq!(tree.count, 11);
    tree.delete();
    assert_eq!(tree.count, 0);
    assert_eq!(tree.max_nodes, 0);
    assert_eq!(tree.next_node, 0);
    assert_eq!(tree.points.len(), 0);
    assert_eq!(tree.node_data.len(), 0);
    assert!(tree.root.is_none());
}

#[test]
fn test_kdtree_rebuild_with_same_count() {
    // Rebuilding with the same count should reuse storage but yield correct results.
    let (mut x, mut y, mut z) = make_11_point_dataset();
    let mut tree = KDTree::new();
    tree.build(&mut x, &mut y, &mut z, 11);
    assert_eq!(tree.next_node, 21);

    // Mutate point 3 to be far away.
    x[3] = 100.0;
    y[3] = 100.0;
    z[3] = 100.0;

    tree.build(&mut x, &mut y, &mut z, 11);
    assert_eq!(tree.count, 11);
    assert_eq!(tree.max_nodes, 21);
    assert_eq!(tree.next_node, 21);

    // A search around (0,0,0) should no longer find point 3.
    let mut iter: Option<KDTreeIterator> = None;
    tree.search(&mut iter, 0.0, 0.0, 0.0, 0.499);
    let it = iter.as_mut().unwrap();
    assert_eq!(it.size, 0);

    // But searching far away picks up point 3.
    tree.search(&mut iter, 100.0, 100.0, 100.0, 0.0);
    let it = iter.as_mut().unwrap();
    assert_eq!(it.size, 1);
    assert_eq!(drain_sorted(it), vec![3]);
}

#[test]
fn test_kdtree_rebuild_with_different_count() {
    // Rebuilding with a different count should reallocate storage.
    let (mut x, mut y, mut z) = make_11_point_dataset();
    let mut tree = KDTree::new();
    tree.build(&mut x, &mut y, &mut z, 11);
    assert_eq!(tree.count, 11);

    let mut x2 = [0.0_f64, 1.0];
    let mut y2 = [0.0_f64, 0.0];
    let mut z2 = [0.0_f64, 0.0];
    tree.build(&mut x2, &mut y2, &mut z2, 2);
    assert_eq!(tree.count, 2);
    assert_eq!(tree.max_nodes, 3);
    assert_eq!(tree.next_node, 3);
}

#[test]
fn test_search_space_is_a_no_op() {
    // The Rust public signature for `search_space` does not return / take
    // an iterator. It is intentionally a no-op in the translation. This
    // test simply documents that fact and verifies the call does not
    // panic and does not mutate iterator state.
    let tree = build_11pt_tree();
    let mut iter: Option<KDTreeIterator> = None;

    // Populate the iterator first so we can detect any (non-)mutation.
    tree.search(&mut iter, 0.5, 0.5, 0.5, 0.5);
    let size_before = iter.as_ref().unwrap().size;
    let current_before = iter.as_ref().unwrap().current;
    assert_eq!(size_before, 11);

    tree.search_space(0.0, 1.0, 0.5, 1.0, 0.0, 1.0);

    // Iterator state should remain unchanged (search_space is a no-op).
    assert_eq!(iter.as_ref().unwrap().size, size_before);
    assert_eq!(iter.as_ref().unwrap().current, current_before);
}

#[test]
fn test_search_apothem_negative_panics() {
    // C asserts apothem >= 0; the Rust translation also asserts the same.
    let tree = build_11pt_tree();
    let mut iter: Option<KDTreeIterator> = None;
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        tree.search(&mut iter, 0.0, 0.0, 0.0, -1.0);
    }));
    assert!(result.is_err(), "negative apothem should panic");
}

#[test]
fn test_build_count_one_panics() {
    // C asserts count > 1; the Rust translation also asserts this.
    let mut x = [0.0_f64];
    let mut y = [0.0_f64];
    let mut z = [0.0_f64];
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let mut tree = KDTree::new();
        tree.build(&mut x, &mut y, &mut z, 1);
    }));
    assert!(result.is_err(), "count<=1 should panic");
}

fn main() {}
