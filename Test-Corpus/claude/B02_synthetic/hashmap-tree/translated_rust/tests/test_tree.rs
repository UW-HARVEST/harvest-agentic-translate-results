// Compare tree functions between the C and Rust shared libraries.

mod common;

use common::*;
use std::ffi::CString;
use std::os::raw::c_int;

fn run_pair<F>(name: &str, f: F)
where
    F: Fn(&DriverLib) -> Vec<u8>,
{
    let c = unsafe { load_c() };
    let r = unsafe { load_rust() };
    let c_out = f(&c);
    let r_out = f(&r);
    assert_eq!(
        c_out, r_out,
        "Mismatch in test '{}'",
        name
    );
}

unsafe fn snapshot_tree(lib: &DriverLib, t: *mut tree_t, ids: &[tree_id_t], out: &mut Vec<u8>) {
    out.extend_from_slice(&((lib.tree_size)(t) as u64).to_le_bytes());
    let tree = &*t;
    out.push(tree.has_root as u8);
    out.extend_from_slice(&(tree.root_id as u64).to_le_bytes());
    out.extend_from_slice(&(tree.node_count as u64).to_le_bytes());

    for &id in ids {
        out.push((lib.tree_contains)(t, id) as u8);
        let n = (lib.tree_get_node)(t, id);
        if n.is_null() {
            out.push(0);
        } else {
            out.push(1);
            let snap = dump_node(n).unwrap();
            out.extend_from_slice(&(snap.id as u64).to_le_bytes());
            out.extend_from_slice(&(snap.parent_id as u64).to_le_bytes());
            out.extend_from_slice(&(snap.child_count as u32).to_le_bytes());
            for cid in &snap.child_ids {
                out.extend_from_slice(&(cid).to_le_bytes());
            }
            out.extend_from_slice(&(snap.data.len() as u32).to_le_bytes());
            out.extend_from_slice(&snap.data);
        }
        // depth, height, count_descendants
        out.extend_from_slice(&((lib.tree_get_depth)(t, id) as i32).to_le_bytes());
        out.extend_from_slice(&((lib.tree_get_height)(t, id) as i32).to_le_bytes());
        out.extend_from_slice(&((lib.tree_count_descendants)(t, id) as i32).to_le_bytes());
    }
}

#[test]
fn test_tree_create_destroy() {
    run_pair("create_destroy", |lib| unsafe {
        let t = (lib.tree_create)();
        assert!(!t.is_null());
        let mut out = Vec::new();
        snapshot_tree(lib, t, &[], &mut out);
        (lib.tree_delete)(t);
        out
    });
}

#[test]
fn test_tree_add_root() {
    run_pair("add_root", |lib| unsafe {
        let t = (lib.tree_create)();
        let s = CString::new("root").unwrap();
        let mut out = Vec::new();
        out.push((lib.tree_add_node)(t, 1, 0, s.as_ptr()) as u8);
        snapshot_tree(lib, t, &[1, 2], &mut out);
        (lib.tree_delete)(t);
        out
    });
}

#[test]
fn test_tree_add_children() {
    run_pair("add_children", |lib| unsafe {
        let t = (lib.tree_create)();
        let mut out = Vec::new();
        let names = ["root", "child1", "child2", "child3"];
        let parents = [0u64, 1, 1, 1];
        for (i, name) in names.iter().enumerate() {
            let s = CString::new(*name).unwrap();
            out.push((lib.tree_add_node)(t, (i + 1) as tree_id_t, parents[i], s.as_ptr()) as u8);
        }
        snapshot_tree(lib, t, &[1, 2, 3, 4], &mut out);
        (lib.tree_delete)(t);
        out
    });
}

#[test]
fn test_tree_deep_hierarchy() {
    run_pair("deep_hierarchy", |lib| unsafe {
        let t = (lib.tree_create)();
        let mut out = Vec::new();
        let levels = ["level0", "level1", "level2", "level3", "level4"];
        let parents = [0u64, 1, 2, 3, 4];
        for (i, name) in levels.iter().enumerate() {
            let s = CString::new(*name).unwrap();
            out.push((lib.tree_add_node)(t, (i + 1) as tree_id_t, parents[i], s.as_ptr()) as u8);
        }
        snapshot_tree(lib, t, &[1, 2, 3, 4, 5], &mut out);
        (lib.tree_delete)(t);
        out
    });
}

#[test]
fn test_tree_remove_leaf() {
    run_pair("remove_leaf", |lib| unsafe {
        let t = (lib.tree_create)();
        let mut out = Vec::new();
        let names = ["root", "child1", "child2"];
        let parents = [0u64, 1, 1];
        for (i, name) in names.iter().enumerate() {
            let s = CString::new(*name).unwrap();
            (lib.tree_add_node)(t, (i + 1) as tree_id_t, parents[i], s.as_ptr());
        }
        out.push((lib.tree_remove_node)(t, 3) as u8);
        snapshot_tree(lib, t, &[1, 2, 3], &mut out);
        (lib.tree_delete)(t);
        out
    });
}

#[test]
fn test_tree_remove_subtree() {
    run_pair("remove_subtree", |lib| unsafe {
        let t = (lib.tree_create)();
        let mut out = Vec::new();
        let names = ["root", "child1", "grandchild1", "grandchild2", "child2"];
        let parents = [0u64, 1, 2, 2, 1];
        for (i, name) in names.iter().enumerate() {
            let s = CString::new(*name).unwrap();
            (lib.tree_add_node)(t, (i + 1) as tree_id_t, parents[i], s.as_ptr());
        }
        out.push((lib.tree_remove_node)(t, 2) as u8);
        snapshot_tree(lib, t, &[1, 2, 3, 4, 5], &mut out);
        (lib.tree_delete)(t);
        out
    });
}

#[test]
fn test_tree_remove_root() {
    run_pair("remove_root", |lib| unsafe {
        let t = (lib.tree_create)();
        let mut out = Vec::new();
        let names = ["root", "child1", "child2"];
        let parents = [0u64, 1, 1];
        for (i, name) in names.iter().enumerate() {
            let s = CString::new(*name).unwrap();
            (lib.tree_add_node)(t, (i + 1) as tree_id_t, parents[i], s.as_ptr());
        }
        out.push((lib.tree_remove_node)(t, 1) as u8);
        snapshot_tree(lib, t, &[1, 2, 3], &mut out);
        (lib.tree_delete)(t);
        out
    });
}

#[test]
fn test_tree_count_descendants_complex() {
    run_pair("count_descendants_complex", |lib| unsafe {
        let t = (lib.tree_create)();
        let mut out = Vec::new();
        let names = ["root", "child1", "grandchild1", "grandchild2", "child2"];
        let parents = [0u64, 1, 2, 2, 1];
        for (i, name) in names.iter().enumerate() {
            let s = CString::new(*name).unwrap();
            (lib.tree_add_node)(t, (i + 1) as tree_id_t, parents[i], s.as_ptr());
        }
        for id in 1u64..=5 {
            out.extend_from_slice(&((lib.tree_count_descendants)(t, id) as i32).to_le_bytes());
        }
        // Non-existent
        out.extend_from_slice(&((lib.tree_count_descendants)(t, 99) as i32).to_le_bytes());

        snapshot_tree(lib, t, &[1, 2, 3, 4, 5], &mut out);
        (lib.tree_delete)(t);
        out
    });
}

#[test]
fn test_tree_find_path() {
    run_pair("find_path", |lib| unsafe {
        let t = (lib.tree_create)();
        let mut out = Vec::new();
        let names = ["root", "child", "grandchild"];
        let parents = [0u64, 1, 2];
        for (i, name) in names.iter().enumerate() {
            let s = CString::new(*name).unwrap();
            (lib.tree_add_node)(t, (i + 1) as tree_id_t, parents[i], s.as_ptr());
        }

        let mut path = [0u64; 10];
        let len = (lib.tree_find_path)(t, 3, path.as_mut_ptr(), 10);
        out.extend_from_slice(&(len as i32).to_le_bytes());
        for &p in path.iter() {
            out.extend_from_slice(&p.to_le_bytes());
        }

        let mut path2 = [0u64; 10];
        let len2 = (lib.tree_find_path)(t, 1, path2.as_mut_ptr(), 10);
        out.extend_from_slice(&(len2 as i32).to_le_bytes());
        for &p in path2.iter() {
            out.extend_from_slice(&p.to_le_bytes());
        }

        // Truncated max_length
        let mut path3 = [0u64; 10];
        let len3 = (lib.tree_find_path)(t, 3, path3.as_mut_ptr(), 2);
        out.extend_from_slice(&(len3 as i32).to_le_bytes());
        for &p in path3.iter() {
            out.extend_from_slice(&p.to_le_bytes());
        }

        // Invalid id
        let mut path4 = [0u64; 10];
        let len4 = (lib.tree_find_path)(t, 99, path4.as_mut_ptr(), 10);
        out.extend_from_slice(&(len4 as i32).to_le_bytes());

        (lib.tree_delete)(t);
        out
    });
}

#[test]
fn test_tree_duplicate_id() {
    run_pair("duplicate_id", |lib| unsafe {
        let t = (lib.tree_create)();
        let mut out = Vec::new();
        let s = CString::new("root").unwrap();
        out.push((lib.tree_add_node)(t, 1, 0, s.as_ptr()) as u8);
        let s2 = CString::new("child").unwrap();
        out.push((lib.tree_add_node)(t, 2, 1, s2.as_ptr()) as u8);
        let s3 = CString::new("duplicate").unwrap();
        out.extend_from_slice(&((lib.tree_add_node)(t, 2, 1, s3.as_ptr()) as i32).to_le_bytes());
        snapshot_tree(lib, t, &[1, 2], &mut out);
        (lib.tree_delete)(t);
        out
    });
}

#[test]
fn test_tree_max_children() {
    run_pair("max_children", |lib| unsafe {
        let t = (lib.tree_create)();
        let mut out = Vec::new();
        let s = CString::new("root").unwrap();
        out.push((lib.tree_add_node)(t, 1, 0, s.as_ptr()) as u8);

        for i in 0..MAX_CHILDREN as u64 {
            let s = CString::new("child").unwrap();
            out.push((lib.tree_add_node)(t, i + 2, 1, s.as_ptr()) as u8);
        }
        let s = CString::new("overflow").unwrap();
        out.extend_from_slice(
            &((lib.tree_add_node)(t, (MAX_CHILDREN + 2) as u64, 1, s.as_ptr()) as i32)
                .to_le_bytes(),
        );

        snapshot_tree(lib, t, &(0..(MAX_CHILDREN as u64 + 5)).collect::<Vec<_>>(), &mut out);
        (lib.tree_delete)(t);
        out
    });
}

#[test]
fn test_tree_complex_structure() {
    run_pair("complex_structure", |lib| unsafe {
        let t = (lib.tree_create)();
        let mut out = Vec::new();

        let entries = [
            (1u64, 0u64, "root"),
            (2, 1, "child1"),
            (3, 1, "child2"),
            (4, 1, "child3"),
            (5, 2, "gc1"),
            (6, 2, "gc2"),
            (7, 3, "gc3"),
            (8, 4, "gc4"),
            (9, 4, "gc5"),
            (10, 7, "ggc1"),
        ];
        for (id, pid, name) in entries.iter() {
            let s = CString::new(*name).unwrap();
            out.push((lib.tree_add_node)(t, *id, *pid, s.as_ptr()) as u8);
        }

        for id in 1u64..=10 {
            out.extend_from_slice(&((lib.tree_get_height)(t, id) as i32).to_le_bytes());
            out.extend_from_slice(&((lib.tree_get_depth)(t, id) as i32).to_le_bytes());
            out.extend_from_slice(&((lib.tree_count_descendants)(t, id) as i32).to_le_bytes());
        }

        snapshot_tree(lib, t, &(1u64..=10).collect::<Vec<_>>(), &mut out);
        (lib.tree_delete)(t);
        out
    });
}

#[test]
fn test_tree_no_data() {
    // Test passing NULL as data
    run_pair("no_data", |lib| unsafe {
        let t = (lib.tree_create)();
        let mut out = Vec::new();
        out.push((lib.tree_add_node)(t, 1, 0, std::ptr::null()) as u8);
        let s = CString::new("child").unwrap();
        out.push((lib.tree_add_node)(t, 2, 1, s.as_ptr()) as u8);
        out.push((lib.tree_add_node)(t, 3, 1, std::ptr::null()) as u8);

        snapshot_tree(lib, t, &[1, 2, 3], &mut out);
        (lib.tree_delete)(t);
        out
    });
}

#[test]
fn test_tree_long_data_truncation() {
    // Test that data is properly truncated at MAX_DATA_LENGTH - 1.
    run_pair("long_data_truncation", |lib| unsafe {
        let t = (lib.tree_create)();
        let mut out = Vec::new();
        let big = "a".repeat(500);
        let s = CString::new(big).unwrap();
        out.push((lib.tree_add_node)(t, 1, 0, s.as_ptr()) as u8);
        snapshot_tree(lib, t, &[1], &mut out);
        (lib.tree_delete)(t);
        out
    });
}

#[test]
fn test_tree_remove_invalid() {
    run_pair("remove_invalid", |lib| unsafe {
        let t = (lib.tree_create)();
        let mut out = Vec::new();
        let s = CString::new("root").unwrap();
        (lib.tree_add_node)(t, 1, 0, s.as_ptr());
        out.extend_from_slice(&((lib.tree_remove_node)(t, 99) as i32).to_le_bytes());
        snapshot_tree(lib, t, &[1, 99], &mut out);
        (lib.tree_delete)(t);
        out
    });
}

#[test]
fn test_tree_null_inputs() {
    run_pair("null_inputs", |lib| unsafe {
        let mut out = Vec::new();
        let s = CString::new("x").unwrap();
        out.extend_from_slice(
            &((lib.tree_add_node)(std::ptr::null_mut(), 1, 0, s.as_ptr()) as i32).to_le_bytes(),
        );
        out.extend_from_slice(
            &((lib.tree_remove_node)(std::ptr::null_mut(), 1) as i32).to_le_bytes(),
        );
        out.push(if (lib.tree_get_node)(std::ptr::null_mut(), 1).is_null() {
            1
        } else {
            0
        });
        out.push((lib.tree_contains)(std::ptr::null_mut(), 1) as u8);
        out.extend_from_slice(&((lib.tree_size)(std::ptr::null_mut()) as u64).to_le_bytes());
        out.extend_from_slice(
            &((lib.tree_get_depth)(std::ptr::null_mut(), 1) as i32).to_le_bytes(),
        );
        out.extend_from_slice(
            &((lib.tree_get_height)(std::ptr::null_mut(), 1) as i32).to_le_bytes(),
        );
        out.extend_from_slice(
            &((lib.tree_count_descendants)(std::ptr::null_mut(), 1) as i32).to_le_bytes(),
        );

        // tree_find_path: tree=null
        let mut p = [0u64; 10];
        out.extend_from_slice(
            &((lib.tree_find_path)(std::ptr::null_mut(), 1, p.as_mut_ptr(), 10) as i32)
                .to_le_bytes(),
        );

        // tree_print on null: prints "(empty tree)" - we don't capture stdout, but
        // it shouldn't crash
        (lib.tree_print)(std::ptr::null_mut());

        out
    });
}

#[test]
fn test_tree_add_to_missing_parent() {
    run_pair("missing_parent", |lib| unsafe {
        let t = (lib.tree_create)();
        let mut out = Vec::new();
        let s = CString::new("root").unwrap();
        (lib.tree_add_node)(t, 1, 0, s.as_ptr());
        let s2 = CString::new("orphan").unwrap();
        out.extend_from_slice(&((lib.tree_add_node)(t, 2, 99, s2.as_ptr()) as i32).to_le_bytes());
        snapshot_tree(lib, t, &[1, 2], &mut out);
        (lib.tree_delete)(t);
        out
    });
}

#[test]
fn test_tree_grow_under_resizing() {
    // Force the underlying hashmap to resize multiple times by adding many
    // nodes; verify both libraries produce identical layouts.
    run_pair("grow_resize", |lib| unsafe {
        let t = (lib.tree_create)();
        let mut out = Vec::new();
        let s = CString::new("root").unwrap();
        (lib.tree_add_node)(t, 1, 0, s.as_ptr());

        // Build a "linear" tree: each new node's parent is the previous one.
        for i in 2u64..=200 {
            let label = format!("n{}", i);
            let s = CString::new(label).unwrap();
            (lib.tree_add_node)(t, i, i - 1, s.as_ptr());
        }

        let ids: Vec<u64> = (1u64..=200).collect();
        // Just compare basic stats and a few selected nodes (depth/height).
        out.extend_from_slice(&((lib.tree_size)(t) as u64).to_le_bytes());
        for id in [1u64, 50, 100, 150, 200].iter() {
            out.extend_from_slice(&((lib.tree_get_depth)(t, *id) as i32).to_le_bytes());
            out.extend_from_slice(&((lib.tree_get_height)(t, *id) as i32).to_le_bytes());
        }

        // Check map capacity (should match).
        let tree = &*t;
        let map = &*tree.node_map;
        out.extend_from_slice(&(map.capacity as u64).to_le_bytes());
        out.extend_from_slice(&(map.size as u64).to_le_bytes());

        // Random sample of nodes
        for id in [1u64, 25, 75, 125, 175].iter() {
            let n = (lib.tree_get_node)(t, *id);
            assert!(!n.is_null());
            let snap = dump_node(n).unwrap();
            out.extend_from_slice(&(snap.id as u64).to_le_bytes());
            out.extend_from_slice(&(snap.parent_id as u64).to_le_bytes());
            out.extend_from_slice(&(snap.child_count as i32).to_le_bytes());
            for c in &snap.child_ids {
                out.extend_from_slice(&c.to_le_bytes());
            }
        }
        let _ = ids;
        (lib.tree_delete)(t);
        out
    });
}
