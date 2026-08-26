//! Faithful translation of `c_src/src/main.c`.
//!
//! Every non-`static` function of `main.c` is exported under its original name,
//! so the translated shared object presents exactly the same symbols as the C
//! one.

use core::ffi::{c_int, c_void};

use crate::hashmap::*;
use crate::tree::*;

/// `strcmp(node->data, s)`
unsafe fn strcmp_data(node: *mut tree_node_t, s: &str) -> c_int {
    let p = core::ptr::addr_of!((*node).data) as *const u8;
    let b = s.as_bytes();
    let mut i: usize = 0;
    loop {
        let a = *p.add(i);
        let c = if i < b.len() { b[i] } else { 0 };
        if a != c {
            return if a < c { -1 } else { 1 };
        }
        if a == 0 {
            return 0;
        }
        i += 1;
    }
}

/// `#define TEST_PASS printf("\u{2713} PASS: %s\n", __func__)`
macro_rules! test_pass {
    ($func:expr) => {
        cprintf!("\u{2713} PASS: {}\n", $func)
    };
}

/// `void test_hashmap_basic(void)`
#[no_mangle]
pub unsafe extern "C" fn test_hashmap_basic() {
    cprintf!("\n=== Testing Hashmap Basic Operations ===\n");

    let map = hashmap_create();
    cassert!(!map.is_null());
    cassert!(hashmap_size(map) == 0);

    // Test put and get
    let mut val1: c_int = 42;
    let mut val2: c_int = 100;
    let mut val3: c_int = 200;
    cassert!(hashmap_put(map, 1, &mut val1 as *mut c_int as *mut c_void) == 0);
    cassert!(hashmap_put(map, 2, &mut val2 as *mut c_int as *mut c_void) == 0);
    cassert!(hashmap_put(map, 3, &mut val3 as *mut c_int as *mut c_void) == 0);
    cassert!(hashmap_size(map) == 3);

    cassert!(*(hashmap_get(map, 1) as *mut c_int) == 42);
    cassert!(*(hashmap_get(map, 2) as *mut c_int) == 100);
    cassert!(*(hashmap_get(map, 3) as *mut c_int) == 200);

    // Test update
    let mut val4: c_int = 500;
    cassert!(hashmap_put(map, 1, &mut val4 as *mut c_int as *mut c_void) == 0);
    cassert!(hashmap_size(map) == 3);
    cassert!(*(hashmap_get(map, 1) as *mut c_int) == 500);

    // Test remove
    let removed = hashmap_remove(map, 2);
    cassert!(removed == &mut val2 as *mut c_int as *mut c_void);
    cassert!(hashmap_size(map) == 2);
    cassert!(hashmap_get(map, 2).is_null());

    // Test contains
    cassert!(hashmap_contains(map, 1) == 1);
    cassert!(hashmap_contains(map, 2) == 0);
    cassert!(hashmap_contains(map, 3) == 1);

    hashmap_destroy(map);
    test_pass!("test_hashmap_basic");
}

/// `void test_hashmap_collisions(void)`
#[no_mangle]
pub unsafe extern "C" fn test_hashmap_collisions() {
    cprintf!("\n=== Testing Hashmap Collisions ===\n");

    let map = hashmap_create();

    // Add many items to force collisions
    let mut values = [0 as c_int; 100];
    for i in 0..100 {
        values[i as usize] = i * 10;
        cassert!(
            hashmap_put(
                map,
                i as tree_id_t,
                &mut values[i as usize] as *mut c_int as *mut c_void
            ) == 0
        );
    }

    cassert!(hashmap_size(map) == 100);

    // Verify all values
    for i in 0..100 {
        let val = hashmap_get(map, i as tree_id_t) as *mut c_int;
        cassert!(!val.is_null());
        cassert!(*val == i * 10);
    }

    hashmap_destroy(map);
    test_pass!("test_hashmap_collisions");
}

/// `void test_tree_creation(void)`
#[no_mangle]
pub unsafe extern "C" fn test_tree_creation() {
    cprintf!("\n=== Testing Tree Creation ===\n");

    let tree = tree_create();
    cassert!(!tree.is_null());
    cassert!(tree_size(tree) == 0);
    cassert!((*tree).has_root == 0);

    tree_delete(tree);
    test_pass!("test_tree_creation");
}

/// `void test_tree_add_root(void)`
#[no_mangle]
pub unsafe extern "C" fn test_tree_add_root() {
    cprintf!("\n=== Testing Tree Add Root ===\n");

    let tree = tree_create();

    // Add root node
    cassert!(tree_add_node(tree, 1, 0, c_lit!("root")) == 0);
    cassert!(tree_size(tree) == 1);
    cassert!((*tree).has_root == 1);
    cassert!((*tree).root_id == 1);

    let root = tree_get_node(tree, 1);
    cassert!(!root.is_null());
    cassert!((*root).id == 1);
    cassert!(strcmp_data(root, "root") == 0);
    cassert!((*root).child_count == 0);

    tree_delete(tree);
    test_pass!("test_tree_add_root");
}

/// `void test_tree_add_children(void)`
#[no_mangle]
pub unsafe extern "C" fn test_tree_add_children() {
    cprintf!("\n=== Testing Tree Add Children ===\n");

    let tree = tree_create();

    // Build tree structure
    cassert!(tree_add_node(tree, 1, 0, c_lit!("root")) == 0);
    cassert!(tree_add_node(tree, 2, 1, c_lit!("child1")) == 0);
    cassert!(tree_add_node(tree, 3, 1, c_lit!("child2")) == 0);
    cassert!(tree_add_node(tree, 4, 1, c_lit!("child3")) == 0);

    cassert!(tree_size(tree) == 4);

    let root = tree_get_node(tree, 1);
    cassert!((*root).child_count == 3);
    cassert!((*root).child_ids[0] == 2);
    cassert!((*root).child_ids[1] == 3);
    cassert!((*root).child_ids[2] == 4);

    tree_delete(tree);
    test_pass!("test_tree_add_children");
}

/// `void test_tree_deep_hierarchy(void)`
#[no_mangle]
pub unsafe extern "C" fn test_tree_deep_hierarchy() {
    cprintf!("\n=== Testing Tree Deep Hierarchy ===\n");

    let tree = tree_create();

    // Build deep tree
    cassert!(tree_add_node(tree, 1, 0, c_lit!("level0")) == 0);
    cassert!(tree_add_node(tree, 2, 1, c_lit!("level1")) == 0);
    cassert!(tree_add_node(tree, 3, 2, c_lit!("level2")) == 0);
    cassert!(tree_add_node(tree, 4, 3, c_lit!("level3")) == 0);
    cassert!(tree_add_node(tree, 5, 4, c_lit!("level4")) == 0);

    cassert!(tree_size(tree) == 5);

    cassert!(tree_get_depth(tree, 1) == 0);
    cassert!(tree_get_depth(tree, 2) == 1);
    cassert!(tree_get_depth(tree, 3) == 2);
    cassert!(tree_get_depth(tree, 4) == 3);
    cassert!(tree_get_depth(tree, 5) == 4);

    cassert!(tree_get_height(tree, 1) == 4);
    cassert!(tree_get_height(tree, 2) == 3);
    cassert!(tree_get_height(tree, 5) == 0);

    tree_delete(tree);
    test_pass!("test_tree_deep_hierarchy");
}

/// `void test_tree_remove_leaf(void)`
#[no_mangle]
pub unsafe extern "C" fn test_tree_remove_leaf() {
    cprintf!("\n=== Testing Tree Remove Leaf ===\n");

    let tree = tree_create();

    cassert!(tree_add_node(tree, 1, 0, c_lit!("root")) == 0);
    cassert!(tree_add_node(tree, 2, 1, c_lit!("child1")) == 0);
    cassert!(tree_add_node(tree, 3, 1, c_lit!("child2")) == 0);

    cassert!(tree_size(tree) == 3);

    // Remove leaf
    cassert!(tree_remove_node(tree, 3) == 0);
    cassert!(tree_size(tree) == 2);
    cassert!(tree_contains(tree, 3) == 0);

    let root = tree_get_node(tree, 1);
    cassert!((*root).child_count == 1);
    cassert!((*root).child_ids[0] == 2);

    tree_delete(tree);
    test_pass!("test_tree_remove_leaf");
}

/// `void test_tree_remove_subtree(void)`
#[no_mangle]
pub unsafe extern "C" fn test_tree_remove_subtree() {
    cprintf!("\n=== Testing Tree Remove Subtree ===\n");

    let tree = tree_create();

    // Build tree with subtree
    cassert!(tree_add_node(tree, 1, 0, c_lit!("root")) == 0);
    cassert!(tree_add_node(tree, 2, 1, c_lit!("child1")) == 0);
    cassert!(tree_add_node(tree, 3, 2, c_lit!("grandchild1")) == 0);
    cassert!(tree_add_node(tree, 4, 2, c_lit!("grandchild2")) == 0);
    cassert!(tree_add_node(tree, 5, 1, c_lit!("child2")) == 0);

    cassert!(tree_size(tree) == 5);

    // Remove node 2 and its children
    cassert!(tree_remove_node(tree, 2) == 0);
    cassert!(tree_size(tree) == 2);
    cassert!(tree_contains(tree, 2) == 0);
    cassert!(tree_contains(tree, 3) == 0);
    cassert!(tree_contains(tree, 4) == 0);
    cassert!(tree_contains(tree, 1) == 1);
    cassert!(tree_contains(tree, 5) == 1);

    tree_delete(tree);
    test_pass!("test_tree_remove_subtree");
}

/// `void test_tree_remove_root(void)`
#[no_mangle]
pub unsafe extern "C" fn test_tree_remove_root() {
    cprintf!("\n=== Testing Tree Remove Root ===\n");

    let tree = tree_create();

    cassert!(tree_add_node(tree, 1, 0, c_lit!("root")) == 0);
    cassert!(tree_add_node(tree, 2, 1, c_lit!("child1")) == 0);
    cassert!(tree_add_node(tree, 3, 1, c_lit!("child2")) == 0);

    cassert!(tree_size(tree) == 3);

    // Remove root
    cassert!(tree_remove_node(tree, 1) == 0);
    cassert!(tree_size(tree) == 0);
    cassert!((*tree).has_root == 0);

    tree_delete(tree);
    test_pass!("test_tree_remove_root");
}

/// `void test_tree_count_descendants(void)`
#[no_mangle]
pub unsafe extern "C" fn test_tree_count_descendants() {
    cprintf!("\n=== Testing Tree Count Descendants ===\n");

    let tree = tree_create();

    /*
     * Build tree:
     *        1
     *       / \
     *      2   5
     *     / \
     *    3   4
     */
    cassert!(tree_add_node(tree, 1, 0, c_lit!("root")) == 0);
    cassert!(tree_add_node(tree, 2, 1, c_lit!("child1")) == 0);
    cassert!(tree_add_node(tree, 3, 2, c_lit!("grandchild1")) == 0);
    cassert!(tree_add_node(tree, 4, 2, c_lit!("grandchild2")) == 0);
    cassert!(tree_add_node(tree, 5, 1, c_lit!("child2")) == 0);

    cassert!(tree_count_descendants(tree, 1) == 4);
    cassert!(tree_count_descendants(tree, 2) == 2);
    cassert!(tree_count_descendants(tree, 3) == 0);
    cassert!(tree_count_descendants(tree, 5) == 0);

    tree_delete(tree);
    test_pass!("test_tree_count_descendants");
}

/// `void test_tree_find_path(void)`
#[no_mangle]
pub unsafe extern "C" fn test_tree_find_path() {
    cprintf!("\n=== Testing Tree Find Path ===\n");

    let tree = tree_create();

    cassert!(tree_add_node(tree, 1, 0, c_lit!("root")) == 0);
    cassert!(tree_add_node(tree, 2, 1, c_lit!("child")) == 0);
    cassert!(tree_add_node(tree, 3, 2, c_lit!("grandchild")) == 0);

    let mut path = [0 as tree_id_t; 10];
    let mut length: c_int;

    length = tree_find_path(tree, 3, path.as_mut_ptr(), 10);
    cassert!(length == 3);
    cassert!(path[0] == 1);
    cassert!(path[1] == 2);
    cassert!(path[2] == 3);

    length = tree_find_path(tree, 1, path.as_mut_ptr(), 10);
    cassert!(length == 1);
    cassert!(path[0] == 1);

    tree_delete(tree);
    test_pass!("test_tree_find_path");
}

/// `void test_tree_duplicate_id(void)`
#[no_mangle]
pub unsafe extern "C" fn test_tree_duplicate_id() {
    cprintf!("\n=== Testing Tree Duplicate ID ===\n");

    let tree = tree_create();

    cassert!(tree_add_node(tree, 1, 0, c_lit!("root")) == 0);
    cassert!(tree_add_node(tree, 2, 1, c_lit!("child")) == 0);

    // Try to add duplicate
    cassert!(tree_add_node(tree, 2, 1, c_lit!("duplicate")) != 0);
    cassert!(tree_size(tree) == 2);

    tree_delete(tree);
    test_pass!("test_tree_duplicate_id");
}

/// `void test_tree_max_children(void)`
#[no_mangle]
pub unsafe extern "C" fn test_tree_max_children() {
    cprintf!("\n=== Testing Tree Max Children ===\n");

    let tree = tree_create();

    cassert!(tree_add_node(tree, 1, 0, c_lit!("root")) == 0);

    // Add MAX_CHILDREN children
    for i in 0..MAX_CHILDREN as c_int {
        cassert!(tree_add_node(tree, (i + 2) as tree_id_t, 1, c_lit!("child")) == 0);
    }

    // Try to add one more (should fail)
    cassert!(tree_add_node(tree, (MAX_CHILDREN + 2) as tree_id_t, 1, c_lit!("overflow")) != 0);
    cassert!(tree_size(tree) == MAX_CHILDREN + 1);

    tree_delete(tree);
    test_pass!("test_tree_max_children");
}

/// `void test_tree_complex_structure(void)`
#[no_mangle]
pub unsafe extern "C" fn test_tree_complex_structure() {
    cprintf!("\n=== Testing Tree Complex Structure ===\n");

    let tree = tree_create();

    /*
     * Build complex tree:
     *           1
     *        /  |  \
     *       2   3   4
     *      /|   |   |\
     *     5 6   7   8 9
     *           |
     *          10
     */
    cassert!(tree_add_node(tree, 1, 0, c_lit!("root")) == 0);
    cassert!(tree_add_node(tree, 2, 1, c_lit!("child1")) == 0);
    cassert!(tree_add_node(tree, 3, 1, c_lit!("child2")) == 0);
    cassert!(tree_add_node(tree, 4, 1, c_lit!("child3")) == 0);
    cassert!(tree_add_node(tree, 5, 2, c_lit!("gc1")) == 0);
    cassert!(tree_add_node(tree, 6, 2, c_lit!("gc2")) == 0);
    cassert!(tree_add_node(tree, 7, 3, c_lit!("gc3")) == 0);
    cassert!(tree_add_node(tree, 8, 4, c_lit!("gc4")) == 0);
    cassert!(tree_add_node(tree, 9, 4, c_lit!("gc5")) == 0);
    cassert!(tree_add_node(tree, 10, 7, c_lit!("ggc1")) == 0);

    cassert!(tree_size(tree) == 10);
    cassert!(tree_get_height(tree, 1) == 3);
    cassert!(tree_count_descendants(tree, 1) == 9);
    cassert!(tree_count_descendants(tree, 2) == 2);
    cassert!(tree_count_descendants(tree, 7) == 1);

    tree_print(tree);

    tree_delete(tree);
    test_pass!("test_tree_complex_structure");
}

/// The body of `int main(void)`.
pub unsafe fn run_main() -> c_int {
    // 40 U+2550 (box drawings double horizontal) between the corner glyphs.
    let bar: String = "\u{2550}".repeat(40);
    cprintf!("\u{2554}{}\u{2557}\n", bar);
    cprintf!("\u{2551}  TREE WITH HASHMAP ID MAPPING TESTS   \u{2551}\n");
    cprintf!("\u{255A}{}\u{255D}\n", bar);

    // Hashmap tests
    test_hashmap_basic();
    test_hashmap_collisions();

    // Tree creation tests
    test_tree_creation();
    test_tree_add_root();
    test_tree_add_children();

    // Tree structure tests
    test_tree_deep_hierarchy();
    test_tree_complex_structure();

    // Tree removal tests
    test_tree_remove_leaf();
    test_tree_remove_subtree();
    test_tree_remove_root();

    // Tree query tests
    test_tree_count_descendants();
    test_tree_find_path();

    // Error handling tests
    test_tree_duplicate_id();
    test_tree_max_children();

    cprintf!("\n");
    cprintf!("========================================\n");
    cprintf!("  All tests passed successfully!\n");
    cprintf!("========================================\n");

    0
}
