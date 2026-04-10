use Bostree::bostree::BOSTree;
use Bostree::test_tree_sanity;
use rand::Rng;

fn strcmp_cmp(a: &str, b: &str) -> i32 {
    a.cmp(b) as i32
}

fn main() {
    let mut tree = BOSTree::bostree_new(strcmp_cmp, None);
    let mut rng = rand::rng();
    let count = 10000;

    for i in 0..count {
        let name: String = (0..32)
            .map(|_| (b'A' + rng.random_range(0..=25u8)) as char)
            .collect();
        if tree.bostree_lookup(&name).is_some() {
            continue;
        }
        tree.bostree_insert(name, None);

        if i % 2500 == 0 {
            println!("{:07} elements", tree.bostree_node_count());
            test_tree_sanity(&tree);
            println!(" sanity check passed");
        }
    }

    let mut remaining = tree.bostree_node_count() as i32;
    while remaining > 0 {
        let w = rng.random_range(0..remaining as u32);
        let node = tree.bostree_select(w);
        assert!(node.is_some(), "Node missing!");
        tree.bostree_remove(&node.unwrap());
        remaining -= 1;

        if remaining > 0 && remaining % 2500 == 0 {
            println!("{:07} elements", tree.bostree_node_count());
            test_tree_sanity(&tree);
            println!(" sanity check passed");
        }
    }
}

#[test]
fn timing() {
    main();
}
