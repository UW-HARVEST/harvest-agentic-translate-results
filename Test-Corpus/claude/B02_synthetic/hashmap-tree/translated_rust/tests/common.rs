// Common helpers for libloading-based comparison tests.
//
// The tests load both the C and Rust shared libraries and compare
// outputs through the FFI boundary. This file is included as a module
// (not as a test target) by every test file via `mod common;`.

#![allow(dead_code)]
#![allow(non_camel_case_types)]

use libloading::{Library, Symbol};
use std::os::raw::{c_char, c_int, c_void};

pub type tree_id_t = u64;

pub const MAX_CHILDREN: usize = 32;
pub const MAX_DATA_LENGTH: usize = 256;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct hashmap_entry_t {
    pub key: tree_id_t,
    pub value: *mut c_void,
    pub occupied: c_int,
    pub deleted: c_int,
}

#[repr(C)]
pub struct hashmap_t {
    pub entries: *mut hashmap_entry_t,
    pub capacity: usize,
    pub size: usize,
    pub deleted_count: usize,
}

#[repr(C)]
pub struct tree_node_t {
    pub id: tree_id_t,
    pub parent_id: tree_id_t,
    pub child_ids: [tree_id_t; MAX_CHILDREN],
    pub child_count: c_int,
    pub data: [c_char; MAX_DATA_LENGTH],
}

#[repr(C)]
pub struct tree_t {
    pub node_map: *mut hashmap_t,
    pub root_id: tree_id_t,
    pub has_root: c_int,
    pub node_count: usize,
}

pub struct DriverLib {
    _lib: Library,
    pub hashmap_create: unsafe extern "C" fn() -> *mut hashmap_t,
    pub hashmap_destroy: unsafe extern "C" fn(*mut hashmap_t),
    pub hashmap_put: unsafe extern "C" fn(*mut hashmap_t, tree_id_t, *mut c_void) -> c_int,
    pub hashmap_get: unsafe extern "C" fn(*mut hashmap_t, tree_id_t) -> *mut c_void,
    pub hashmap_remove: unsafe extern "C" fn(*mut hashmap_t, tree_id_t) -> *mut c_void,
    pub hashmap_contains: unsafe extern "C" fn(*mut hashmap_t, tree_id_t) -> c_int,
    pub hashmap_size: unsafe extern "C" fn(*mut hashmap_t) -> usize,
    pub hashmap_clear: unsafe extern "C" fn(*mut hashmap_t),

    pub tree_create: unsafe extern "C" fn() -> *mut tree_t,
    pub tree_delete: unsafe extern "C" fn(*mut tree_t),
    pub tree_add_node:
        unsafe extern "C" fn(*mut tree_t, tree_id_t, tree_id_t, *const c_char) -> c_int,
    pub tree_remove_node: unsafe extern "C" fn(*mut tree_t, tree_id_t) -> c_int,
    pub tree_get_node: unsafe extern "C" fn(*mut tree_t, tree_id_t) -> *mut tree_node_t,
    pub tree_contains: unsafe extern "C" fn(*mut tree_t, tree_id_t) -> c_int,
    pub tree_size: unsafe extern "C" fn(*mut tree_t) -> usize,
    pub tree_print: unsafe extern "C" fn(*mut tree_t),
    pub tree_get_depth: unsafe extern "C" fn(*mut tree_t, tree_id_t) -> c_int,
    pub tree_get_height: unsafe extern "C" fn(*mut tree_t, tree_id_t) -> c_int,
    pub tree_count_descendants: unsafe extern "C" fn(*mut tree_t, tree_id_t) -> c_int,
    pub tree_find_path:
        unsafe extern "C" fn(*mut tree_t, tree_id_t, *mut tree_id_t, c_int) -> c_int,
}

impl DriverLib {
    pub unsafe fn load(path: &str) -> Self {
        let lib = Library::new(path).unwrap_or_else(|e| panic!("Failed to load {}: {}", path, e));

        macro_rules! sym {
            ($name:ident) => {{
                let s: Symbol<unsafe extern "C" fn()> = lib
                    .get(stringify!($name).as_bytes())
                    .unwrap_or_else(|e| panic!("Missing symbol {}: {}", stringify!($name), e));
                std::mem::transmute(*s.into_raw())
            }};
        }

        DriverLib {
            hashmap_create: sym!(hashmap_create),
            hashmap_destroy: sym!(hashmap_destroy),
            hashmap_put: sym!(hashmap_put),
            hashmap_get: sym!(hashmap_get),
            hashmap_remove: sym!(hashmap_remove),
            hashmap_contains: sym!(hashmap_contains),
            hashmap_size: sym!(hashmap_size),
            hashmap_clear: sym!(hashmap_clear),

            tree_create: sym!(tree_create),
            tree_delete: sym!(tree_delete),
            tree_add_node: sym!(tree_add_node),
            tree_remove_node: sym!(tree_remove_node),
            tree_get_node: sym!(tree_get_node),
            tree_contains: sym!(tree_contains),
            tree_size: sym!(tree_size),
            tree_print: sym!(tree_print),
            tree_get_depth: sym!(tree_get_depth),
            tree_get_height: sym!(tree_get_height),
            tree_count_descendants: sym!(tree_count_descendants),
            tree_find_path: sym!(tree_find_path),
            _lib: lib,
        }
    }
}

pub fn c_lib_path() -> String {
    let p = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    format!("{}/c_src/build/libdriver_c.so", p)
}

pub fn rust_lib_path() -> String {
    let p = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    // Try release first, then debug
    let release = format!("{}/target/release/libdriver.so", p);
    if std::path::Path::new(&release).exists() {
        return release;
    }
    format!("{}/target/debug/libdriver.so", p)
}

pub unsafe fn load_c() -> DriverLib {
    DriverLib::load(&c_lib_path())
}

pub unsafe fn load_rust() -> DriverLib {
    DriverLib::load(&rust_lib_path())
}

/// Helper: build a tree_node_t snapshot equivalent. We mainly compare
/// the public fields the C implementation exposes.
pub unsafe fn dump_node(node: *const tree_node_t) -> Option<NodeSnapshot> {
    if node.is_null() {
        return None;
    }
    let n = &*node;
    let cnt = n.child_count as usize;
    let mut child_ids = Vec::new();
    for i in 0..cnt {
        child_ids.push(n.child_ids[i]);
    }
    // C string from data
    let mut end = 0usize;
    while end < MAX_DATA_LENGTH && n.data[end] != 0 {
        end += 1;
    }
    let bytes: &[u8] = std::slice::from_raw_parts(n.data.as_ptr() as *const u8, end);
    let data = bytes.to_vec();
    Some(NodeSnapshot {
        id: n.id,
        parent_id: n.parent_id,
        child_count: n.child_count,
        child_ids,
        data,
    })
}

#[derive(Debug, PartialEq, Eq)]
pub struct NodeSnapshot {
    pub id: tree_id_t,
    pub parent_id: tree_id_t,
    pub child_count: c_int,
    pub child_ids: Vec<tree_id_t>,
    pub data: Vec<u8>,
}
