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
    let mut t = test_tree();
    let g = t.bostree_lookup("G").expect("G not found");
    t.bostree_remove(&g);
    let h = t.bostree_lookup("H").expect("H not found");
    t.bostree_remove(&h);

    test_tree_sanity(&t);
    assert!(
        t.bostree_lookup("E").is_some(),
        "Nodes missing after removing another one"
    );
}

#[test]
fn test_remove_bug_2() {
    main();
}
