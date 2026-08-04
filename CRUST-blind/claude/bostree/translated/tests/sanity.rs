use Bostree::bostree::{
    bostree_next_node, bostree_previous_node, bostree_rank, BOSTree,
};
use Bostree::test_tree_sanity;

fn cmp(a: &str, b: &str) -> i32 {
    use std::cmp::Ordering;
    match a.cmp(b) {
        Ordering::Less => -1,
        Ordering::Equal => 0,
        Ordering::Greater => 1,
    }
}

#[test]
fn sanity_insert_then_remove_alphabet() {
    let mut tree = BOSTree::bostree_new(cmp, None);
    for c in b'A'..=b'Z' {
        let key = (c as char).to_string();
        tree.bostree_insert(key.clone(), Some(format!("Value")));
        test_tree_sanity(&tree);
    }
    assert_eq!(tree.bostree_node_count(), 26);

    for c in b'A'..=b'Z' {
        let key = (c as char).to_string();
        let node = tree.bostree_lookup(&key).expect("must find");
        tree.bostree_remove(&node);
        test_tree_sanity(&tree);
    }
    assert_eq!(tree.bostree_node_count(), 0);
}

#[test]
fn remove_each_node_from_full_tree() {
    fn build() -> BOSTree {
        let mut t = BOSTree::bostree_new(cmp, None);
        for c in b'A'..b'Z' {
            let k = (c as char).to_string();
            t.bostree_insert(k, None);
        }
        t
    }

    for c in b'A'..b'Z' {
        let mut t = build();
        let k = (c as char).to_string();
        let n = t.bostree_lookup(&k).expect("must find");
        t.bostree_remove(&n);
        test_tree_sanity(&t);
        assert_eq!(t.bostree_node_count(), (b'Z' - b'A' - 1) as u32);
    }
}

#[test]
fn rank_select_round_trip() {
    let mut tree = BOSTree::bostree_new(cmp, None);
    let keys = ["mango", "apple", "banana", "kiwi", "orange", "fig", "grape"];
    for k in keys.iter() {
        tree.bostree_insert(k.to_string(), None);
    }
    let n = tree.bostree_node_count();
    for i in 0..n {
        let node = tree.bostree_select(i).unwrap();
        assert_eq!(bostree_rank(&node), i);
    }
    let mut sorted: Vec<&str> = keys.iter().copied().collect();
    sorted.sort();
    let mut current = tree.bostree_select(0);
    let mut idx = 0;
    while let Some(n) = current {
        assert_eq!(n.borrow().key, sorted[idx]);
        idx += 1;
        current = bostree_next_node(&n);
    }
    assert_eq!(idx, sorted.len());

    let last = tree.bostree_select(tree.bostree_node_count() - 1).unwrap();
    let mut cur = Some(last);
    let mut idx = sorted.len();
    while let Some(n) = cur {
        idx -= 1;
        assert_eq!(n.borrow().key, sorted[idx]);
        cur = bostree_previous_node(&n);
    }
    assert_eq!(idx, 0);
}

#[test]
fn random_insert_remove_stress() {
    use rand::rngs::StdRng;
    use rand::{Rng, SeedableRng};
    let mut rng = StdRng::seed_from_u64(0xDEADBEEF);
    let mut tree = BOSTree::bostree_new(cmp, None);

    let mut keys: Vec<String> = Vec::new();
    for i in 0..500u32 {
        let key = format!("k{:08x}", rng.random::<u32>());
        if tree.bostree_lookup(&key).is_none() {
            tree.bostree_insert(key.clone(), Some(format!("v{}", i)));
            keys.push(key);
            test_tree_sanity(&tree);
        }
    }

    while !keys.is_empty() {
        let idx = (rng.random::<u32>() as usize) % keys.len();
        let key = keys.swap_remove(idx);
        let n = tree.bostree_lookup(&key).expect("should be present");
        tree.bostree_remove(&n);
        test_tree_sanity(&tree);
    }
    assert_eq!(tree.bostree_node_count(), 0);
}
