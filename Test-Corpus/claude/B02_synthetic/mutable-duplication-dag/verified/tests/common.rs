// Shared test helpers: declare the FFI types and a thin loader for both
// implementations.
//
// Layout MUST match the C dag_lib.h struct layout, since we read these fields
// out of pointers returned by both libraries.

#![allow(non_camel_case_types)]
#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::os::raw::{c_char, c_int};

pub const MAX_CITY_NAME: usize = 64;
pub const MAX_EDGES: usize = 10;
pub const MAX_NODES: usize = 100;

#[repr(C)]
pub struct edge_t {
    pub destination: *mut node_t,
    pub distance: c_int,
}

#[repr(C)]
pub struct node_t {
    pub city_name: [c_char; MAX_CITY_NAME],
    pub ref_count: c_int,
    pub edges: [edge_t; MAX_EDGES],
    pub edge_count: c_int,
}

#[repr(C)]
pub struct graph_t {
    pub nodes: [*mut node_t; MAX_NODES],
    pub node_count: c_int,
}

pub type CreateGraph = unsafe extern "C" fn() -> *mut graph_t;
pub type AddNode = unsafe extern "C" fn(*mut graph_t, *const c_char) -> *mut node_t;
pub type AddEdge = unsafe extern "C" fn(*mut node_t, *mut node_t, c_int) -> c_int;
pub type DeleteNode = unsafe extern "C" fn(*mut node_t);
pub type ShallowCopy = unsafe extern "C" fn(*mut node_t) -> *mut node_t;
pub type FindShortestPath =
    unsafe extern "C" fn(*mut node_t, *mut node_t, *mut c_int) -> *mut *mut node_t;
pub type FreeGraph = unsafe extern "C" fn(*mut graph_t);
pub type GetNodeByName = unsafe extern "C" fn(*mut graph_t, *const c_char) -> *mut node_t;
pub type PrintNode = unsafe extern "C" fn(*mut node_t);
pub type PrintGraph = unsafe extern "C" fn(*mut graph_t);

pub struct DagLib {
    _lib: Library,
    pub create_graph: unsafe extern "C" fn() -> *mut graph_t,
    pub add_node: unsafe extern "C" fn(*mut graph_t, *const c_char) -> *mut node_t,
    pub add_edge: unsafe extern "C" fn(*mut node_t, *mut node_t, c_int) -> c_int,
    pub delete_node: unsafe extern "C" fn(*mut node_t),
    pub shallow_copy: unsafe extern "C" fn(*mut node_t) -> *mut node_t,
    pub find_shortest_path:
        unsafe extern "C" fn(*mut node_t, *mut node_t, *mut c_int) -> *mut *mut node_t,
    pub free_graph: unsafe extern "C" fn(*mut graph_t),
    pub get_node_by_name: unsafe extern "C" fn(*mut graph_t, *const c_char) -> *mut node_t,
    pub print_node: unsafe extern "C" fn(*mut node_t),
    pub print_graph: unsafe extern "C" fn(*mut graph_t),
}

impl DagLib {
    pub fn load(path: &str) -> Self {
        unsafe {
            let lib = Library::new(path)
                .unwrap_or_else(|e| panic!("failed to load {}: {}", path, e));
            let create_graph: Symbol<CreateGraph> =
                lib.get(b"create_graph").expect("create_graph");
            let add_node: Symbol<AddNode> = lib.get(b"add_node").expect("add_node");
            let add_edge: Symbol<AddEdge> = lib.get(b"add_edge").expect("add_edge");
            let delete_node: Symbol<DeleteNode> =
                lib.get(b"delete_node").expect("delete_node");
            let shallow_copy: Symbol<ShallowCopy> =
                lib.get(b"shallow_copy").expect("shallow_copy");
            let find_shortest_path: Symbol<FindShortestPath> = lib
                .get(b"find_shortest_path")
                .expect("find_shortest_path");
            let free_graph: Symbol<FreeGraph> = lib.get(b"free_graph").expect("free_graph");
            let get_node_by_name: Symbol<GetNodeByName> =
                lib.get(b"get_node_by_name").expect("get_node_by_name");
            let print_node: Symbol<PrintNode> = lib.get(b"print_node").expect("print_node");
            let print_graph: Symbol<PrintGraph> = lib.get(b"print_graph").expect("print_graph");

            DagLib {
                create_graph: *create_graph.into_raw(),
                add_node: *add_node.into_raw(),
                add_edge: *add_edge.into_raw(),
                delete_node: *delete_node.into_raw(),
                shallow_copy: *shallow_copy.into_raw(),
                find_shortest_path: *find_shortest_path.into_raw(),
                free_graph: *free_graph.into_raw(),
                get_node_by_name: *get_node_by_name.into_raw(),
                print_node: *print_node.into_raw(),
                print_graph: *print_graph.into_raw(),
                _lib: lib,
            }
        }
    }
}

/// Convert a NUL-terminated city_name byte array to a Vec<u8> excluding NUL.
pub fn city_name_bytes(arr: &[c_char; MAX_CITY_NAME]) -> Vec<u8> {
    let mut out = Vec::new();
    for &c in arr.iter() {
        if c == 0 {
            break;
        }
        out.push(c as u8);
    }
    out
}

/// Build a NUL-terminated buffer from a string for FFI.
pub fn cstr(s: &str) -> Vec<u8> {
    let mut v = s.as_bytes().to_vec();
    v.push(0);
    v
}

/// Locate the C and Rust .so paths from CARGO_MANIFEST_DIR.
pub fn lib_paths() -> (String, String) {
    let manifest = env!("CARGO_MANIFEST_DIR");
    let c_path = format!("{}/build_c/libdag_c.so", manifest);
    let rust_path = format!("{}/target/release/libdriver.so", manifest);
    (c_path, rust_path)
}
