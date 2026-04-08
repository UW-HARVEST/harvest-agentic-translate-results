use Bostree::bostree::BOSTree;
use Bostree::test_tree_sanity;

fn strcmp_cmp(a: &str, b: &str) -> i32 {
    a.cmp(b) as i32
}

fn test_tree() -> BOSTree {
    let mut t = BOSTree::bostree_new(strcmp_cmp, None);
    for c in b'A'..b'Z' {
        let key = String::from(c as char);
        t.bostree_insert(key, None);
    }
    t
}

fn main() {
    for c in b'A'..b'Z' {
        let remove_key = String::from(c as char);
        let mut t = test_tree();
        let node = t.bostree_lookup(&remove_key).expect("node not found");
        t.bostree_remove(&node);
        test_tree_sanity(&t);
        assert_eq!(
            t.bostree_node_count(),
            (b'Z' - b'A' - 1) as u32,
            "Removed one node from a tree, but the node count did not decrease properly."
        );
    }
}

#[test]
fn test_remove_bug() {
    main();
}
