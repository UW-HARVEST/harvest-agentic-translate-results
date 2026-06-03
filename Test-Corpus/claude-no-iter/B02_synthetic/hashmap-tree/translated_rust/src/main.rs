// main.rs - translation of main.c
//
// Reproduces the C executable's exact stdout and stderr output, including
// the "Error: ..." messages emitted to stderr by tree_add_node when
// duplicate IDs or maximum-children conditions are encountered.
//
// We mirror C stdio buffering: when stdout is not a TTY, glibc uses full
// (block) buffering and stderr is unbuffered. To get byte-identical merged
// output, all stdout writes go through a single BufWriter that is flushed
// only at program exit. stderr writes go directly via eprintln!.

mod hashmap;
mod tree;

use hashmap::Hashmap;
use std::io::{BufWriter, Stdout, Write};
use tree::{Tree, TreeNode, MAX_CHILDREN};

// We model the C `assert` macro as a runtime check. The C builds without
// -DNDEBUG, so all asserts are active.
macro_rules! c_assert {
    ($cond:expr) => {
        if !$cond {
            std::process::abort();
        }
    };
}

fn cstr_eq(buf: &[u8], s: &str) -> bool {
    let sb = s.as_bytes();
    let mut end = 0;
    while end < buf.len() && buf[end] != 0 {
        end += 1;
    }
    &buf[..end] == sb
}

type Out = BufWriter<Stdout>;

fn test_hashmap_basic(out: &mut Out) {
    writeln!(out).unwrap();
    writeln!(out, "=== Testing Hashmap Basic Operations ===").unwrap();

    let mut map = Hashmap::create();
    c_assert!(map.size() == 0);

    let val1 = Box::leak(Box::new(42i32));
    let val2 = Box::leak(Box::new(100i32));
    let val3 = Box::leak(Box::new(200i32));

    let p1 = val1 as *mut i32 as *mut TreeNode;
    let p2 = val2 as *mut i32 as *mut TreeNode;
    let p3 = val3 as *mut i32 as *mut TreeNode;

    c_assert!(map.put(1, p1) == 0);
    c_assert!(map.put(2, p2) == 0);
    c_assert!(map.put(3, p3) == 0);
    c_assert!(map.size() == 3);

    unsafe {
        c_assert!(*(map.get(1) as *mut i32) == 42);
        c_assert!(*(map.get(2) as *mut i32) == 100);
        c_assert!(*(map.get(3) as *mut i32) == 200);
    }

    let val4 = Box::leak(Box::new(500i32));
    let p4 = val4 as *mut i32 as *mut TreeNode;
    c_assert!(map.put(1, p4) == 0);
    c_assert!(map.size() == 3);
    unsafe {
        c_assert!(*(map.get(1) as *mut i32) == 500);
    }

    let removed = map.remove(2);
    c_assert!(removed == p2);
    c_assert!(map.size() == 2);
    c_assert!(map.get(2).is_null());

    c_assert!(map.contains(1));
    c_assert!(!map.contains(2));
    c_assert!(map.contains(3));

    drop(map);
    writeln!(out, "✓ PASS: test_hashmap_basic").unwrap();
}

fn test_hashmap_collisions(out: &mut Out) {
    writeln!(out).unwrap();
    writeln!(out, "=== Testing Hashmap Collisions ===").unwrap();

    let mut map = Hashmap::create();

    let mut values: Vec<Box<i32>> = (0..100).map(|i| Box::new(i * 10)).collect();
    let ptrs: Vec<*mut TreeNode> = values
        .iter_mut()
        .map(|b| (&mut **b) as *mut i32 as *mut TreeNode)
        .collect();

    for i in 0..100i64 {
        c_assert!(map.put(i as u64, ptrs[i as usize]) == 0);
    }

    c_assert!(map.size() == 100);

    for i in 0..100i64 {
        let val = map.get(i as u64);
        c_assert!(!val.is_null());
        unsafe {
            c_assert!(*(val as *mut i32) == (i as i32) * 10);
        }
    }

    drop(map);
    writeln!(out, "✓ PASS: test_hashmap_collisions").unwrap();
}

fn test_tree_creation(out: &mut Out) {
    writeln!(out).unwrap();
    writeln!(out, "=== Testing Tree Creation ===").unwrap();

    let tree = Tree::create();
    c_assert!(tree.size() == 0);
    c_assert!(!tree.has_root);

    Tree::delete(tree);
    writeln!(out, "✓ PASS: test_tree_creation").unwrap();
}

fn test_tree_add_root(out: &mut Out) {
    writeln!(out).unwrap();
    writeln!(out, "=== Testing Tree Add Root ===").unwrap();

    let mut tree = Tree::create();

    c_assert!(tree.add_node(1, 0, Some("root")) == 0);
    c_assert!(tree.size() == 1);
    c_assert!(tree.has_root);
    c_assert!(tree.root_id == 1);

    let root_ptr = tree.get_node(1);
    c_assert!(!root_ptr.is_null());
    let root: &TreeNode = unsafe { &*root_ptr };
    c_assert!(root.id == 1);
    c_assert!(cstr_eq(&root.data, "root"));
    c_assert!(root.child_count == 0);

    Tree::delete(tree);
    writeln!(out, "✓ PASS: test_tree_add_root").unwrap();
}

fn test_tree_add_children(out: &mut Out) {
    writeln!(out).unwrap();
    writeln!(out, "=== Testing Tree Add Children ===").unwrap();

    let mut tree = Tree::create();

    c_assert!(tree.add_node(1, 0, Some("root")) == 0);
    c_assert!(tree.add_node(2, 1, Some("child1")) == 0);
    c_assert!(tree.add_node(3, 1, Some("child2")) == 0);
    c_assert!(tree.add_node(4, 1, Some("child3")) == 0);

    c_assert!(tree.size() == 4);

    let root_ptr = tree.get_node(1);
    let root: &TreeNode = unsafe { &*root_ptr };
    c_assert!(root.child_count == 3);
    c_assert!(root.child_ids[0] == 2);
    c_assert!(root.child_ids[1] == 3);
    c_assert!(root.child_ids[2] == 4);

    Tree::delete(tree);
    writeln!(out, "✓ PASS: test_tree_add_children").unwrap();
}

fn test_tree_deep_hierarchy(out: &mut Out) {
    writeln!(out).unwrap();
    writeln!(out, "=== Testing Tree Deep Hierarchy ===").unwrap();

    let mut tree = Tree::create();

    c_assert!(tree.add_node(1, 0, Some("level0")) == 0);
    c_assert!(tree.add_node(2, 1, Some("level1")) == 0);
    c_assert!(tree.add_node(3, 2, Some("level2")) == 0);
    c_assert!(tree.add_node(4, 3, Some("level3")) == 0);
    c_assert!(tree.add_node(5, 4, Some("level4")) == 0);

    c_assert!(tree.size() == 5);

    c_assert!(tree.get_depth(1) == 0);
    c_assert!(tree.get_depth(2) == 1);
    c_assert!(tree.get_depth(3) == 2);
    c_assert!(tree.get_depth(4) == 3);
    c_assert!(tree.get_depth(5) == 4);

    c_assert!(tree.get_height(1) == 4);
    c_assert!(tree.get_height(2) == 3);
    c_assert!(tree.get_height(5) == 0);

    Tree::delete(tree);
    writeln!(out, "✓ PASS: test_tree_deep_hierarchy").unwrap();
}

fn test_tree_remove_leaf(out: &mut Out) {
    writeln!(out).unwrap();
    writeln!(out, "=== Testing Tree Remove Leaf ===").unwrap();

    let mut tree = Tree::create();

    c_assert!(tree.add_node(1, 0, Some("root")) == 0);
    c_assert!(tree.add_node(2, 1, Some("child1")) == 0);
    c_assert!(tree.add_node(3, 1, Some("child2")) == 0);
    c_assert!(tree.size() == 3);

    c_assert!(tree.remove_node(3) == 0);
    c_assert!(tree.size() == 2);
    c_assert!(!tree.contains(3));

    let root_ptr = tree.get_node(1);
    let root: &TreeNode = unsafe { &*root_ptr };
    c_assert!(root.child_count == 1);
    c_assert!(root.child_ids[0] == 2);

    Tree::delete(tree);
    writeln!(out, "✓ PASS: test_tree_remove_leaf").unwrap();
}

fn test_tree_remove_subtree(out: &mut Out) {
    writeln!(out).unwrap();
    writeln!(out, "=== Testing Tree Remove Subtree ===").unwrap();

    let mut tree = Tree::create();

    c_assert!(tree.add_node(1, 0, Some("root")) == 0);
    c_assert!(tree.add_node(2, 1, Some("child1")) == 0);
    c_assert!(tree.add_node(3, 2, Some("grandchild1")) == 0);
    c_assert!(tree.add_node(4, 2, Some("grandchild2")) == 0);
    c_assert!(tree.add_node(5, 1, Some("child2")) == 0);

    c_assert!(tree.size() == 5);

    c_assert!(tree.remove_node(2) == 0);
    c_assert!(tree.size() == 2);
    c_assert!(!tree.contains(2));
    c_assert!(!tree.contains(3));
    c_assert!(!tree.contains(4));
    c_assert!(tree.contains(1));
    c_assert!(tree.contains(5));

    Tree::delete(tree);
    writeln!(out, "✓ PASS: test_tree_remove_subtree").unwrap();
}

fn test_tree_remove_root(out: &mut Out) {
    writeln!(out).unwrap();
    writeln!(out, "=== Testing Tree Remove Root ===").unwrap();

    let mut tree = Tree::create();

    c_assert!(tree.add_node(1, 0, Some("root")) == 0);
    c_assert!(tree.add_node(2, 1, Some("child1")) == 0);
    c_assert!(tree.add_node(3, 1, Some("child2")) == 0);

    c_assert!(tree.size() == 3);

    c_assert!(tree.remove_node(1) == 0);
    c_assert!(tree.size() == 0);
    c_assert!(!tree.has_root);

    Tree::delete(tree);
    writeln!(out, "✓ PASS: test_tree_remove_root").unwrap();
}

fn test_tree_count_descendants(out: &mut Out) {
    writeln!(out).unwrap();
    writeln!(out, "=== Testing Tree Count Descendants ===").unwrap();

    let mut tree = Tree::create();

    c_assert!(tree.add_node(1, 0, Some("root")) == 0);
    c_assert!(tree.add_node(2, 1, Some("child1")) == 0);
    c_assert!(tree.add_node(3, 2, Some("grandchild1")) == 0);
    c_assert!(tree.add_node(4, 2, Some("grandchild2")) == 0);
    c_assert!(tree.add_node(5, 1, Some("child2")) == 0);

    c_assert!(tree.count_descendants(1) == 4);
    c_assert!(tree.count_descendants(2) == 2);
    c_assert!(tree.count_descendants(3) == 0);
    c_assert!(tree.count_descendants(5) == 0);

    Tree::delete(tree);
    writeln!(out, "✓ PASS: test_tree_count_descendants").unwrap();
}

fn test_tree_find_path(out: &mut Out) {
    writeln!(out).unwrap();
    writeln!(out, "=== Testing Tree Find Path ===").unwrap();

    let mut tree = Tree::create();

    c_assert!(tree.add_node(1, 0, Some("root")) == 0);
    c_assert!(tree.add_node(2, 1, Some("child")) == 0);
    c_assert!(tree.add_node(3, 2, Some("grandchild")) == 0);

    let mut path = [0u64; 10];
    let length = tree.find_path(3, &mut path, 10);
    c_assert!(length == 3);
    c_assert!(path[0] == 1);
    c_assert!(path[1] == 2);
    c_assert!(path[2] == 3);

    let length = tree.find_path(1, &mut path, 10);
    c_assert!(length == 1);
    c_assert!(path[0] == 1);

    Tree::delete(tree);
    writeln!(out, "✓ PASS: test_tree_find_path").unwrap();
}

fn test_tree_duplicate_id(out: &mut Out) {
    writeln!(out).unwrap();
    writeln!(out, "=== Testing Tree Duplicate ID ===").unwrap();

    let mut tree = Tree::create();

    c_assert!(tree.add_node(1, 0, Some("root")) == 0);
    c_assert!(tree.add_node(2, 1, Some("child")) == 0);

    // This will print the error message to stderr.
    c_assert!(tree.add_node(2, 1, Some("duplicate")) != 0);
    c_assert!(tree.size() == 2);

    Tree::delete(tree);
    writeln!(out, "✓ PASS: test_tree_duplicate_id").unwrap();
}

fn test_tree_max_children(out: &mut Out) {
    writeln!(out).unwrap();
    writeln!(out, "=== Testing Tree Max Children ===").unwrap();

    let mut tree = Tree::create();

    c_assert!(tree.add_node(1, 0, Some("root")) == 0);

    for i in 0..MAX_CHILDREN as i32 {
        c_assert!(tree.add_node((i + 2) as u64, 1, Some("child")) == 0);
    }

    c_assert!(tree.add_node((MAX_CHILDREN as i32 + 2) as u64, 1, Some("overflow")) != 0);
    c_assert!(tree.size() == MAX_CHILDREN + 1);

    Tree::delete(tree);
    writeln!(out, "✓ PASS: test_tree_max_children").unwrap();
}

fn test_tree_complex_structure(out: &mut Out) {
    writeln!(out).unwrap();
    writeln!(out, "=== Testing Tree Complex Structure ===").unwrap();

    let mut tree = Tree::create();

    c_assert!(tree.add_node(1, 0, Some("root")) == 0);
    c_assert!(tree.add_node(2, 1, Some("child1")) == 0);
    c_assert!(tree.add_node(3, 1, Some("child2")) == 0);
    c_assert!(tree.add_node(4, 1, Some("child3")) == 0);
    c_assert!(tree.add_node(5, 2, Some("gc1")) == 0);
    c_assert!(tree.add_node(6, 2, Some("gc2")) == 0);
    c_assert!(tree.add_node(7, 3, Some("gc3")) == 0);
    c_assert!(tree.add_node(8, 4, Some("gc4")) == 0);
    c_assert!(tree.add_node(9, 4, Some("gc5")) == 0);
    c_assert!(tree.add_node(10, 7, Some("ggc1")) == 0);

    c_assert!(tree.size() == 10);
    c_assert!(tree.get_height(1) == 3);
    c_assert!(tree.count_descendants(1) == 9);
    c_assert!(tree.count_descendants(2) == 2);
    c_assert!(tree.count_descendants(7) == 1);

    tree.print(out);

    Tree::delete(tree);
    writeln!(out, "✓ PASS: test_tree_complex_structure").unwrap();
}

fn main() {
    // Use a 64 KiB buffered stdout. Larger than total program output, so
    // nothing is written to fd 1 until we flush at end of main, mirroring
    // glibc full-buffering of redirected stdout.
    let mut out: Out = BufWriter::with_capacity(1 << 16, std::io::stdout());

    writeln!(out, "╔════════════════════════════════════════╗").unwrap();
    writeln!(out, "║  TREE WITH HASHMAP ID MAPPING TESTS   ║").unwrap();
    writeln!(out, "╚════════════════════════════════════════╝").unwrap();

    test_hashmap_basic(&mut out);
    test_hashmap_collisions(&mut out);

    test_tree_creation(&mut out);
    test_tree_add_root(&mut out);
    test_tree_add_children(&mut out);

    test_tree_deep_hierarchy(&mut out);
    test_tree_complex_structure(&mut out);

    test_tree_remove_leaf(&mut out);
    test_tree_remove_subtree(&mut out);
    test_tree_remove_root(&mut out);

    test_tree_count_descendants(&mut out);
    test_tree_find_path(&mut out);

    test_tree_duplicate_id(&mut out);
    test_tree_max_children(&mut out);

    writeln!(out).unwrap();
    writeln!(out, "========================================").unwrap();
    writeln!(out, "  All tests passed successfully!").unwrap();
    writeln!(out, "========================================").unwrap();

    out.flush().unwrap();
}
