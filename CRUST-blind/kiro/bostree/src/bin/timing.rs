use Bostree::bostree::BOSTree;
use Bostree::test_tree_sanity;
use rand::Rng;
use std::time::Instant;

fn strcmp_cmp(a: &str, b: &str) -> i32 {
    a.cmp(b) as i32
}

fn main() {
    let mut rng = rand::rng();
    let mut tree = BOSTree::bostree_new(strcmp_cmp, None);

    let n = 100_000;

    for i in 0..n {
        let mut name: String;
        loop {
            name = (0..32)
                .map(|_| (b'A' + rng.random_range(0..26u8)) as char)
                .collect();
            if tree.bostree_lookup(&name).is_none() {
                break;
            }
        }
        tree.bostree_insert(name, None);

        if i % 10000 == 0 {
            let start = Instant::now();
            test_tree_sanity(&tree);
            let elapsed = start.elapsed();
            println!(
                "{:07} elements, sanity check passed ({:.3?})",
                tree.bostree_node_count(),
                elapsed
            );
        }
    }

    let mut remaining = n as u32;
    for _ in (1..=n).rev() {
        let w = rng.random_range(0..remaining);
        let node = tree.bostree_select(w).expect("Node missing!");
        tree.bostree_remove(&node);
        remaining -= 1;

        if remaining % 10000 == 0 && remaining > 0 {
            let start = Instant::now();
            test_tree_sanity(&tree);
            let elapsed = start.elapsed();
            println!(
                "{:07} elements, sanity check passed ({:.3?})",
                tree.bostree_node_count(),
                elapsed
            );
        }
    }
}

#[test]
fn test_timing() {
    main();
}
