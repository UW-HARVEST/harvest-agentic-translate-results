//! Integration tests: compare C shared library outputs vs Rust reimplementation.
//!
//! Hierarchy (bottom-up):
//!   1. hashmap (lowest level)
//!   2. tree (depends on hashmap)
//!   3. main binary stdout comparison

use libloading::{Library, Symbol};
use std::ffi::CString;
use std::os::raw::{c_char, c_int, c_void};
use std::process::Command;

type TreeId = u64;

// C struct layouts matching the headers
#[repr(C)]
struct CHashmapEntry {
    key: u64,
    value: *mut c_void,
    occupied: c_int,
    deleted: c_int,
}

#[repr(C)]
struct CHashmap {
    entries: *mut CHashmapEntry,
    capacity: usize,
    size: usize,
    deleted_count: usize,
}

#[repr(C)]
struct CTreeNode {
    id: u64,
    parent_id: u64,
    child_ids: [u64; 32],
    child_count: c_int,
    data: [u8; 256],
}

#[repr(C)]
struct CTree {
    node_map: *mut CHashmap,
    root_id: u64,
    has_root: c_int,
    node_count: usize,
}

fn lib_path() -> String {
    format!(
        "{}/translated_rust/c_src/build/libhashmap_tree.so",
        env!("CARGO_MANIFEST_DIR").trim_end_matches("/translated_rust")
    )
}

fn load_lib() -> Library {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("c_src/build/libhashmap_tree.so");
    unsafe { Library::new(&path).expect("Failed to load C shared library") }
}

// ============================================================
// Rust re-exports for testing (copy the core logic inline since
// main.rs doesn't expose a library API)
// ============================================================

// We include the Rust source as a module for testing
// Since main.rs is a binary, we replicate the key structures and functions.
// Instead, let's just run both binaries and compare output.

// But first, let's test the C functions directly and compare with Rust logic.

// ---- Hashmap tests (lowest level) ----

#[test]
fn test_hashmap_create_destroy() {
    let lib = load_lib();
    unsafe {
        let create: Symbol<unsafe extern "C" fn() -> *mut CHashmap> =
            lib.get(b"hashmap_create").unwrap();
        let destroy: Symbol<unsafe extern "C" fn(*mut CHashmap)> =
            lib.get(b"hashmap_destroy").unwrap();
        let size: Symbol<unsafe extern "C" fn(*mut CHashmap) -> usize> =
            lib.get(b"hashmap_size").unwrap();

        let map = create();
        assert!(!map.is_null());
        assert_eq!(size(map), 0);
        destroy(map);
    }
}

#[test]
fn test_hashmap_put_get_size() {
    let lib = load_lib();
    unsafe {
        let create: Symbol<unsafe extern "C" fn() -> *mut CHashmap> =
            lib.get(b"hashmap_create").unwrap();
        let destroy: Symbol<unsafe extern "C" fn(*mut CHashmap)> =
            lib.get(b"hashmap_destroy").unwrap();
        let put: Symbol<unsafe extern "C" fn(*mut CHashmap, u64, *mut c_void) -> c_int> =
            lib.get(b"hashmap_put").unwrap();
        let get: Symbol<unsafe extern "C" fn(*mut CHashmap, u64) -> *mut c_void> =
            lib.get(b"hashmap_get").unwrap();
        let size_fn: Symbol<unsafe extern "C" fn(*mut CHashmap) -> usize> =
            lib.get(b"hashmap_size").unwrap();

        let map = create();

        let mut vals: Vec<i32> = vec![42, 100, 200];
        assert_eq!(put(map, 1, &mut vals[0] as *mut i32 as *mut c_void), 0);
        assert_eq!(put(map, 2, &mut vals[1] as *mut i32 as *mut c_void), 0);
        assert_eq!(put(map, 3, &mut vals[2] as *mut i32 as *mut c_void), 0);
        assert_eq!(size_fn(map), 3);

        let v1 = get(map, 1) as *mut i32;
        assert_eq!(*v1, 42);
        let v2 = get(map, 2) as *mut i32;
        assert_eq!(*v2, 100);
        let v3 = get(map, 3) as *mut i32;
        assert_eq!(*v3, 200);

        // Non-existent key
        let v_none = get(map, 999);
        assert!(v_none.is_null());

        destroy(map);
    }
}

#[test]
fn test_hashmap_remove() {
    let lib = load_lib();
    unsafe {
        let create: Symbol<unsafe extern "C" fn() -> *mut CHashmap> =
            lib.get(b"hashmap_create").unwrap();
        let destroy: Symbol<unsafe extern "C" fn(*mut CHashmap)> =
            lib.get(b"hashmap_destroy").unwrap();
        let put: Symbol<unsafe extern "C" fn(*mut CHashmap, u64, *mut c_void) -> c_int> =
            lib.get(b"hashmap_put").unwrap();
        let remove_fn: Symbol<unsafe extern "C" fn(*mut CHashmap, u64) -> *mut c_void> =
            lib.get(b"hashmap_remove").unwrap();
        let size_fn: Symbol<unsafe extern "C" fn(*mut CHashmap) -> usize> =
            lib.get(b"hashmap_size").unwrap();
        let contains: Symbol<unsafe extern "C" fn(*mut CHashmap, u64) -> c_int> =
            lib.get(b"hashmap_contains").unwrap();

        let map = create();
        let mut vals: Vec<i32> = vec![42, 100, 200];
        put(map, 1, &mut vals[0] as *mut i32 as *mut c_void);
        put(map, 2, &mut vals[1] as *mut i32 as *mut c_void);
        put(map, 3, &mut vals[2] as *mut i32 as *mut c_void);

        let removed = remove_fn(map, 2);
        assert!(!removed.is_null());
        assert_eq!(*(removed as *mut i32), 100);
        assert_eq!(size_fn(map), 2);
        assert_eq!(contains(map, 1), 1);
        assert_eq!(contains(map, 2), 0);
        assert_eq!(contains(map, 3), 1);

        // Remove non-existent
        let removed2 = remove_fn(map, 999);
        assert!(removed2.is_null());

        destroy(map);
    }
}

#[test]
fn test_hashmap_clear() {
    let lib = load_lib();
    unsafe {
        let create: Symbol<unsafe extern "C" fn() -> *mut CHashmap> =
            lib.get(b"hashmap_create").unwrap();
        let destroy: Symbol<unsafe extern "C" fn(*mut CHashmap)> =
            lib.get(b"hashmap_destroy").unwrap();
        let put: Symbol<unsafe extern "C" fn(*mut CHashmap, u64, *mut c_void) -> c_int> =
            lib.get(b"hashmap_put").unwrap();
        let clear: Symbol<unsafe extern "C" fn(*mut CHashmap)> =
            lib.get(b"hashmap_clear").unwrap();
        let size_fn: Symbol<unsafe extern "C" fn(*mut CHashmap) -> usize> =
            lib.get(b"hashmap_size").unwrap();

        let map = create();
        let mut v = 42i32;
        put(map, 1, &mut v as *mut i32 as *mut c_void);
        put(map, 2, &mut v as *mut i32 as *mut c_void);
        assert_eq!(size_fn(map), 2);

        clear(map);
        assert_eq!(size_fn(map), 0);

        destroy(map);
    }
}

#[test]
fn test_hashmap_collisions_100() {
    let lib = load_lib();
    unsafe {
        let create: Symbol<unsafe extern "C" fn() -> *mut CHashmap> =
            lib.get(b"hashmap_create").unwrap();
        let destroy: Symbol<unsafe extern "C" fn(*mut CHashmap)> =
            lib.get(b"hashmap_destroy").unwrap();
        let put: Symbol<unsafe extern "C" fn(*mut CHashmap, u64, *mut c_void) -> c_int> =
            lib.get(b"hashmap_put").unwrap();
        let get: Symbol<unsafe extern "C" fn(*mut CHashmap, u64) -> *mut c_void> =
            lib.get(b"hashmap_get").unwrap();
        let size_fn: Symbol<unsafe extern "C" fn(*mut CHashmap) -> usize> =
            lib.get(b"hashmap_size").unwrap();

        let map = create();
        let mut values: Vec<i32> = (0..100).map(|i| i * 10).collect();

        for i in 0..100u64 {
            assert_eq!(put(map, i, &mut values[i as usize] as *mut i32 as *mut c_void), 0);
        }
        assert_eq!(size_fn(map), 100);

        for i in 0..100u64 {
            let v = get(map, i) as *mut i32;
            assert!(!v.is_null());
            assert_eq!(*v, (i as i32) * 10);
        }

        destroy(map);
    }
}

// ---- Tree tests (higher level, uses hashmap internally) ----

#[test]
fn test_tree_create_delete() {
    let lib = load_lib();
    unsafe {
        let create: Symbol<unsafe extern "C" fn() -> *mut CTree> =
            lib.get(b"tree_create").unwrap();
        let delete: Symbol<unsafe extern "C" fn(*mut CTree)> =
            lib.get(b"tree_delete").unwrap();
        let size_fn: Symbol<unsafe extern "C" fn(*mut CTree) -> usize> =
            lib.get(b"tree_size").unwrap();

        let tree = create();
        assert!(!tree.is_null());
        assert_eq!(size_fn(tree), 0);
        assert_eq!((*tree).has_root, 0);
        delete(tree);
    }
}

#[test]
fn test_tree_add_and_query() {
    let lib = load_lib();
    unsafe {
        let create: Symbol<unsafe extern "C" fn() -> *mut CTree> =
            lib.get(b"tree_create").unwrap();
        let delete: Symbol<unsafe extern "C" fn(*mut CTree)> =
            lib.get(b"tree_delete").unwrap();
        let add_node: Symbol<unsafe extern "C" fn(*mut CTree, u64, u64, *const c_char) -> c_int> =
            lib.get(b"tree_add_node").unwrap();
        let size_fn: Symbol<unsafe extern "C" fn(*mut CTree) -> usize> =
            lib.get(b"tree_size").unwrap();
        let contains: Symbol<unsafe extern "C" fn(*mut CTree, u64) -> c_int> =
            lib.get(b"tree_contains").unwrap();
        let get_node: Symbol<unsafe extern "C" fn(*mut CTree, u64) -> *mut CTreeNode> =
            lib.get(b"tree_get_node").unwrap();
        let get_depth: Symbol<unsafe extern "C" fn(*mut CTree, u64) -> c_int> =
            lib.get(b"tree_get_depth").unwrap();
        let get_height: Symbol<unsafe extern "C" fn(*mut CTree, u64) -> c_int> =
            lib.get(b"tree_get_height").unwrap();
        let count_desc: Symbol<unsafe extern "C" fn(*mut CTree, u64) -> c_int> =
            lib.get(b"tree_count_descendants").unwrap();
        let find_path: Symbol<unsafe extern "C" fn(*mut CTree, u64, *mut u64, c_int) -> c_int> =
            lib.get(b"tree_find_path").unwrap();

        let tree = create();

        // Build tree:
        //        1
        //       / \
        //      2   5
        //     / \
        //    3   4
        let s = |s: &str| CString::new(s).unwrap();
        assert_eq!(add_node(tree, 1, 0, s("root").as_ptr()), 0);
        assert_eq!(add_node(tree, 2, 1, s("child1").as_ptr()), 0);
        assert_eq!(add_node(tree, 3, 2, s("grandchild1").as_ptr()), 0);
        assert_eq!(add_node(tree, 4, 2, s("grandchild2").as_ptr()), 0);
        assert_eq!(add_node(tree, 5, 1, s("child2").as_ptr()), 0);

        // Size
        assert_eq!(size_fn(tree), 5);

        // Contains
        assert_eq!(contains(tree, 1), 1);
        assert_eq!(contains(tree, 5), 1);
        assert_eq!(contains(tree, 99), 0);

        // Depth
        assert_eq!(get_depth(tree, 1), 0);
        assert_eq!(get_depth(tree, 2), 1);
        assert_eq!(get_depth(tree, 3), 2);
        assert_eq!(get_depth(tree, 5), 1);

        // Height
        assert_eq!(get_height(tree, 1), 2);
        assert_eq!(get_height(tree, 2), 1);
        assert_eq!(get_height(tree, 3), 0);

        // Count descendants
        assert_eq!(count_desc(tree, 1), 4);
        assert_eq!(count_desc(tree, 2), 2);
        assert_eq!(count_desc(tree, 3), 0);
        assert_eq!(count_desc(tree, 5), 0);

        // Find path
        let mut path = [0u64; 10];
        let len = find_path(tree, 3, path.as_mut_ptr(), 10);
        assert_eq!(len, 3);
        assert_eq!(path[0], 1);
        assert_eq!(path[1], 2);
        assert_eq!(path[2], 3);

        let len = find_path(tree, 1, path.as_mut_ptr(), 10);
        assert_eq!(len, 1);
        assert_eq!(path[0], 1);

        // Get node and check data
        let node = get_node(tree, 1);
        assert!(!node.is_null());
        let data_bytes = &(*node).data;
        let data_str = std::ffi::CStr::from_ptr(data_bytes.as_ptr() as *const c_char)
            .to_str()
            .unwrap();
        assert_eq!(data_str, "root");
        assert_eq!((*node).child_count, 2);

        delete(tree);
    }
}

#[test]
fn test_tree_remove_operations() {
    let lib = load_lib();
    unsafe {
        let create: Symbol<unsafe extern "C" fn() -> *mut CTree> =
            lib.get(b"tree_create").unwrap();
        let delete: Symbol<unsafe extern "C" fn(*mut CTree)> =
            lib.get(b"tree_delete").unwrap();
        let add_node: Symbol<unsafe extern "C" fn(*mut CTree, u64, u64, *const c_char) -> c_int> =
            lib.get(b"tree_add_node").unwrap();
        let remove_node: Symbol<unsafe extern "C" fn(*mut CTree, u64) -> c_int> =
            lib.get(b"tree_remove_node").unwrap();
        let size_fn: Symbol<unsafe extern "C" fn(*mut CTree) -> usize> =
            lib.get(b"tree_size").unwrap();
        let contains: Symbol<unsafe extern "C" fn(*mut CTree, u64) -> c_int> =
            lib.get(b"tree_contains").unwrap();

        let s = |s: &str| CString::new(s).unwrap();

        // Test remove leaf
        let tree = create();
        add_node(tree, 1, 0, s("root").as_ptr());
        add_node(tree, 2, 1, s("child1").as_ptr());
        add_node(tree, 3, 1, s("child2").as_ptr());
        assert_eq!(remove_node(tree, 3), 0);
        assert_eq!(size_fn(tree), 2);
        assert_eq!(contains(tree, 3), 0);
        delete(tree);

        // Test remove subtree
        let tree = create();
        add_node(tree, 1, 0, s("root").as_ptr());
        add_node(tree, 2, 1, s("child1").as_ptr());
        add_node(tree, 3, 2, s("gc1").as_ptr());
        add_node(tree, 4, 2, s("gc2").as_ptr());
        add_node(tree, 5, 1, s("child2").as_ptr());
        assert_eq!(remove_node(tree, 2), 0);
        assert_eq!(size_fn(tree), 2);
        assert_eq!(contains(tree, 2), 0);
        assert_eq!(contains(tree, 3), 0);
        assert_eq!(contains(tree, 4), 0);
        assert_eq!(contains(tree, 1), 1);
        assert_eq!(contains(tree, 5), 1);
        delete(tree);

        // Test remove root
        let tree = create();
        add_node(tree, 1, 0, s("root").as_ptr());
        add_node(tree, 2, 1, s("child1").as_ptr());
        add_node(tree, 3, 1, s("child2").as_ptr());
        assert_eq!(remove_node(tree, 1), 0);
        assert_eq!(size_fn(tree), 0);
        assert_eq!((*tree).has_root, 0);
        delete(tree);
    }
}

#[test]
fn test_tree_duplicate_and_max_children() {
    let lib = load_lib();
    unsafe {
        let create: Symbol<unsafe extern "C" fn() -> *mut CTree> =
            lib.get(b"tree_create").unwrap();
        let delete: Symbol<unsafe extern "C" fn(*mut CTree)> =
            lib.get(b"tree_delete").unwrap();
        let add_node: Symbol<unsafe extern "C" fn(*mut CTree, u64, u64, *const c_char) -> c_int> =
            lib.get(b"tree_add_node").unwrap();
        let size_fn: Symbol<unsafe extern "C" fn(*mut CTree) -> usize> =
            lib.get(b"tree_size").unwrap();

        let s = |s: &str| CString::new(s).unwrap();

        // Duplicate ID
        let tree = create();
        assert_eq!(add_node(tree, 1, 0, s("root").as_ptr()), 0);
        assert_eq!(add_node(tree, 2, 1, s("child").as_ptr()), 0);
        assert!(add_node(tree, 2, 1, s("dup").as_ptr()) != 0);
        assert_eq!(size_fn(tree), 2);
        delete(tree);

        // Max children
        let tree = create();
        assert_eq!(add_node(tree, 1, 0, s("root").as_ptr()), 0);
        for i in 0..32u64 {
            assert_eq!(add_node(tree, i + 2, 1, s("child").as_ptr()), 0);
        }
        assert!(add_node(tree, 34, 1, s("overflow").as_ptr()) != 0);
        assert_eq!(size_fn(tree), 33);
        delete(tree);
    }
}

// ---- Now compare C vs Rust by running both binaries and comparing stdout ----

#[test]
fn test_binary_output_matches() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");

    // Run C binary
    let c_binary = format!("{}/c_src/build/driver", manifest_dir);
    let c_output = Command::new(&c_binary)
        .output()
        .expect("Failed to run C binary");
    let c_stdout = String::from_utf8_lossy(&c_output.stdout);

    // Build and run Rust binary
    let rust_build = Command::new("timeout")
        .args(&["600", "cargo", "build", "--bin", "driver"])
        .current_dir(manifest_dir)
        .output()
        .expect("Failed to build Rust binary");
    assert!(
        rust_build.status.success(),
        "Rust build failed: {}",
        String::from_utf8_lossy(&rust_build.stderr)
    );

    let rust_output = Command::new("timeout")
        .args(&["60", "cargo", "run", "--bin", "driver"])
        .current_dir(manifest_dir)
        .output()
        .expect("Failed to run Rust binary");
    let rust_stdout = String::from_utf8_lossy(&rust_output.stdout);

    // Compare byte-for-byte
    if c_stdout != rust_stdout {
        // Show diff for debugging
        let c_lines: Vec<&str> = c_stdout.lines().collect();
        let r_lines: Vec<&str> = rust_stdout.lines().collect();
        let max = c_lines.len().max(r_lines.len());
        for i in 0..max {
            let cl = c_lines.get(i).unwrap_or(&"<missing>");
            let rl = r_lines.get(i).unwrap_or(&"<missing>");
            if cl != rl {
                eprintln!("DIFF at line {}: C='{}' Rust='{}'", i + 1, cl, rl);
            }
        }
        panic!(
            "Binary outputs differ!\n--- C stdout ({} bytes) ---\n{}\n--- Rust stdout ({} bytes) ---\n{}",
            c_stdout.len(),
            c_stdout,
            rust_stdout.len(),
            rust_stdout
        );
    }
}

// ---- Compare Rust reimplementation logic vs C for specific operations ----
// These tests call C via FFI and Rust directly, comparing results.

// We need to include the Rust code. Since it's in main.rs (binary),
// we'll test by comparing the binary outputs above and the C FFI tests
// verify the C library works correctly. The key comparison is the binary
// output test which exercises all functions.

#[test]
fn test_tree_complex_structure_c_values() {
    // Verify C produces expected values for the complex tree test
    let lib = load_lib();
    unsafe {
        let create: Symbol<unsafe extern "C" fn() -> *mut CTree> =
            lib.get(b"tree_create").unwrap();
        let delete: Symbol<unsafe extern "C" fn(*mut CTree)> =
            lib.get(b"tree_delete").unwrap();
        let add_node: Symbol<unsafe extern "C" fn(*mut CTree, u64, u64, *const c_char) -> c_int> =
            lib.get(b"tree_add_node").unwrap();
        let size_fn: Symbol<unsafe extern "C" fn(*mut CTree) -> usize> =
            lib.get(b"tree_size").unwrap();
        let get_height: Symbol<unsafe extern "C" fn(*mut CTree, u64) -> c_int> =
            lib.get(b"tree_get_height").unwrap();
        let count_desc: Symbol<unsafe extern "C" fn(*mut CTree, u64) -> c_int> =
            lib.get(b"tree_count_descendants").unwrap();

        let s = |s: &str| CString::new(s).unwrap();

        let tree = create();
        add_node(tree, 1, 0, s("root").as_ptr());
        add_node(tree, 2, 1, s("child1").as_ptr());
        add_node(tree, 3, 1, s("child2").as_ptr());
        add_node(tree, 4, 1, s("child3").as_ptr());
        add_node(tree, 5, 2, s("gc1").as_ptr());
        add_node(tree, 6, 2, s("gc2").as_ptr());
        add_node(tree, 7, 3, s("gc3").as_ptr());
        add_node(tree, 8, 4, s("gc4").as_ptr());
        add_node(tree, 9, 4, s("gc5").as_ptr());
        add_node(tree, 10, 7, s("ggc1").as_ptr());

        assert_eq!(size_fn(tree), 10);
        assert_eq!(get_height(tree, 1), 3);
        assert_eq!(count_desc(tree, 1), 9);
        assert_eq!(count_desc(tree, 2), 2);
        assert_eq!(count_desc(tree, 7), 1);

        delete(tree);
    }
}

#[test]
fn test_tree_deep_hierarchy_c_values() {
    let lib = load_lib();
    unsafe {
        let create: Symbol<unsafe extern "C" fn() -> *mut CTree> =
            lib.get(b"tree_create").unwrap();
        let delete: Symbol<unsafe extern "C" fn(*mut CTree)> =
            lib.get(b"tree_delete").unwrap();
        let add_node: Symbol<unsafe extern "C" fn(*mut CTree, u64, u64, *const c_char) -> c_int> =
            lib.get(b"tree_add_node").unwrap();
        let get_depth: Symbol<unsafe extern "C" fn(*mut CTree, u64) -> c_int> =
            lib.get(b"tree_get_depth").unwrap();
        let get_height: Symbol<unsafe extern "C" fn(*mut CTree, u64) -> c_int> =
            lib.get(b"tree_get_height").unwrap();

        let s = |s: &str| CString::new(s).unwrap();

        let tree = create();
        add_node(tree, 1, 0, s("level0").as_ptr());
        add_node(tree, 2, 1, s("level1").as_ptr());
        add_node(tree, 3, 2, s("level2").as_ptr());
        add_node(tree, 4, 3, s("level3").as_ptr());
        add_node(tree, 5, 4, s("level4").as_ptr());

        assert_eq!(get_depth(tree, 1), 0);
        assert_eq!(get_depth(tree, 2), 1);
        assert_eq!(get_depth(tree, 3), 2);
        assert_eq!(get_depth(tree, 4), 3);
        assert_eq!(get_depth(tree, 5), 4);

        assert_eq!(get_height(tree, 1), 4);
        assert_eq!(get_height(tree, 2), 3);
        assert_eq!(get_height(tree, 5), 0);

        delete(tree);
    }
}
