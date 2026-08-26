use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, CStr, CString};
use std::path::PathBuf;
use std::ptr;

const MAX_CITY_NAME: usize = 64;
const MAX_EDGES: usize = 10;
const MAX_NODES: usize = 100;

#[repr(C)]
struct edge_t {
    destination: *mut node_t,
    distance: c_int,
}

#[repr(C)]
struct node_t {
    city_name: [c_char; MAX_CITY_NAME],
    ref_count: c_int,
    edges: [edge_t; MAX_EDGES],
    edge_count: c_int,
}

#[repr(C)]
struct graph_t {
    nodes: [*mut node_t; MAX_NODES],
    node_count: c_int,
}

fn c_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("c_src/build/libdag.so")
}

fn rust_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target/debug/libdag_city_route.so")
}

type CreateGraphFn = unsafe extern "C" fn() -> *mut graph_t;
type AddNodeFn = unsafe extern "C" fn(*mut graph_t, *const c_char) -> *mut node_t;
type AddEdgeFn = unsafe extern "C" fn(*mut node_t, *mut node_t, c_int) -> c_int;
type DeleteNodeFn = unsafe extern "C" fn(*mut node_t);
type ShallowCopyFn = unsafe extern "C" fn(*mut node_t) -> *mut node_t;
type FindShortestPathFn = unsafe extern "C" fn(*mut node_t, *mut node_t, *mut c_int) -> *mut *mut node_t;
type FreeGraphFn = unsafe extern "C" fn(*mut graph_t);
type GetNodeByNameFn = unsafe extern "C" fn(*mut graph_t, *const c_char) -> *mut node_t;

extern "C" {
    #[link_name = "free"]
    fn libc_free(ptr: *mut std::ffi::c_void);
}

unsafe fn node_name(n: *mut node_t) -> String {
    CStr::from_ptr((*n).city_name.as_ptr()).to_string_lossy().into_owned()
}

/// Build a standard test graph: A->B(5), A->C(2), B->D(3), C->B(1), C->D(7)
/// Returns (graph, [A, B, C, D])
unsafe fn build_test_graph(
    create_graph: &Symbol<CreateGraphFn>,
    add_node: &Symbol<AddNodeFn>,
    add_edge: &Symbol<AddEdgeFn>,
) -> (*mut graph_t, Vec<*mut node_t>) {
    let g = (create_graph)();
    assert!(!g.is_null());
    let names = ["A", "B", "C", "D"];
    let mut nodes = Vec::new();
    for name in &names {
        let cs = CString::new(*name).unwrap();
        let n = (add_node)(g, cs.as_ptr());
        assert!(!n.is_null(), "add_node failed for {}", name);
        nodes.push(n);
    }
    // A->B(5), A->C(2), B->D(3), C->B(1), C->D(7)
    assert_eq!((add_edge)(nodes[0], nodes[1], 5), 0);
    assert_eq!((add_edge)(nodes[0], nodes[2], 2), 0);
    assert_eq!((add_edge)(nodes[1], nodes[3], 3), 0);
    assert_eq!((add_edge)(nodes[2], nodes[1], 1), 0);
    assert_eq!((add_edge)(nodes[2], nodes[3], 7), 0);
    (g, nodes)
}

/// Run a test body against both C and Rust libraries
fn run_both(test_fn: unsafe fn(&Library)) {
    let c_path = c_lib_path();
    let r_path = rust_lib_path();
    assert!(c_path.exists(), "C .so not found at {:?}", c_path);
    assert!(r_path.exists(), "Rust .so not found at {:?}", r_path);
    unsafe {
        let c_lib = Library::new(&c_path).expect("load C lib");
        test_fn(&c_lib);
        let r_lib = Library::new(&r_path).expect("load Rust lib");
        test_fn(&r_lib);
    }
}

#[test]
fn test_create_and_free_graph() {
    run_both(|lib| unsafe {
        let create: Symbol<CreateGraphFn> = lib.get(b"create_graph\0").unwrap();
        let free: Symbol<FreeGraphFn> = lib.get(b"free_graph\0").unwrap();
        let g = (create)();
        assert!(!g.is_null());
        assert_eq!((*g).node_count, 0);
        (free)(g);
    });
}

#[test]
fn test_add_node_basic() {
    run_both(|lib| unsafe {
        let create: Symbol<CreateGraphFn> = lib.get(b"create_graph\0").unwrap();
        let add: Symbol<AddNodeFn> = lib.get(b"add_node\0").unwrap();
        let free: Symbol<FreeGraphFn> = lib.get(b"free_graph\0").unwrap();

        let g = (create)();
        let name = CString::new("Boston").unwrap();
        let n = (add)(g, name.as_ptr());
        assert!(!n.is_null());
        assert_eq!(node_name(n), "Boston");
        assert_eq!((*n).ref_count, 1);
        assert_eq!((*n).edge_count, 0);
        assert_eq!((*g).node_count, 1);
        (free)(g);
    });
}

#[test]
fn test_add_node_duplicate_returns_null() {
    run_both(|lib| unsafe {
        let create: Symbol<CreateGraphFn> = lib.get(b"create_graph\0").unwrap();
        let add: Symbol<AddNodeFn> = lib.get(b"add_node\0").unwrap();
        let free: Symbol<FreeGraphFn> = lib.get(b"free_graph\0").unwrap();

        let g = (create)();
        let name = CString::new("X").unwrap();
        let n1 = (add)(g, name.as_ptr());
        assert!(!n1.is_null());
        let n2 = (add)(g, name.as_ptr());
        assert!(n2.is_null());
        assert_eq!((*g).node_count, 1);
        (free)(g);
    });
}

#[test]
fn test_add_node_null_params() {
    run_both(|lib| unsafe {
        let add: Symbol<AddNodeFn> = lib.get(b"add_node\0").unwrap();
        let n = (add)(ptr::null_mut(), ptr::null());
        assert!(n.is_null());
    });
}

#[test]
fn test_add_edge_basic() {
    run_both(|lib| unsafe {
        let create: Symbol<CreateGraphFn> = lib.get(b"create_graph\0").unwrap();
        let add_n: Symbol<AddNodeFn> = lib.get(b"add_node\0").unwrap();
        let add_e: Symbol<AddEdgeFn> = lib.get(b"add_edge\0").unwrap();
        let free: Symbol<FreeGraphFn> = lib.get(b"free_graph\0").unwrap();

        let g = (create)();
        let a = (add_n)(g, CString::new("A").unwrap().as_ptr());
        let b = (add_n)(g, CString::new("B").unwrap().as_ptr());
        assert_eq!((add_e)(a, b, 10), 0);
        assert_eq!((*a).edge_count, 1);
        assert_eq!((*a).edges[0].distance, 10);
        assert_eq!((*a).edges[0].destination, b);
        (free)(g);
    });
}

#[test]
fn test_add_edge_null_returns_neg1() {
    run_both(|lib| unsafe {
        let add_e: Symbol<AddEdgeFn> = lib.get(b"add_edge\0").unwrap();
        assert_eq!((add_e)(ptr::null_mut(), ptr::null_mut(), 1), -1);
    });
}

#[test]
fn test_add_edge_negative_distance() {
    run_both(|lib| unsafe {
        let create: Symbol<CreateGraphFn> = lib.get(b"create_graph\0").unwrap();
        let add_n: Symbol<AddNodeFn> = lib.get(b"add_node\0").unwrap();
        let add_e: Symbol<AddEdgeFn> = lib.get(b"add_edge\0").unwrap();
        let free: Symbol<FreeGraphFn> = lib.get(b"free_graph\0").unwrap();

        let g = (create)();
        let a = (add_n)(g, CString::new("A").unwrap().as_ptr());
        let b = (add_n)(g, CString::new("B").unwrap().as_ptr());
        assert_eq!((add_e)(a, b, -5), -1);
        assert_eq!((*a).edge_count, 0);
        (free)(g);
    });
}

#[test]
fn test_add_edge_duplicate() {
    run_both(|lib| unsafe {
        let create: Symbol<CreateGraphFn> = lib.get(b"create_graph\0").unwrap();
        let add_n: Symbol<AddNodeFn> = lib.get(b"add_node\0").unwrap();
        let add_e: Symbol<AddEdgeFn> = lib.get(b"add_edge\0").unwrap();
        let free: Symbol<FreeGraphFn> = lib.get(b"free_graph\0").unwrap();

        let g = (create)();
        let a = (add_n)(g, CString::new("A").unwrap().as_ptr());
        let b = (add_n)(g, CString::new("B").unwrap().as_ptr());
        assert_eq!((add_e)(a, b, 10), 0);
        assert_eq!((add_e)(a, b, 20), -1);
        assert_eq!((*a).edge_count, 1);
        (free)(g);
    });
}

#[test]
fn test_get_node_by_name() {
    run_both(|lib| unsafe {
        let create: Symbol<CreateGraphFn> = lib.get(b"create_graph\0").unwrap();
        let add_n: Symbol<AddNodeFn> = lib.get(b"add_node\0").unwrap();
        let get: Symbol<GetNodeByNameFn> = lib.get(b"get_node_by_name\0").unwrap();
        let free: Symbol<FreeGraphFn> = lib.get(b"free_graph\0").unwrap();

        let g = (create)();
        let a = (add_n)(g, CString::new("Alpha").unwrap().as_ptr());
        let _b = (add_n)(g, CString::new("Beta").unwrap().as_ptr());

        let found = (get)(g, CString::new("Alpha").unwrap().as_ptr());
        assert_eq!(found, a);
        let not_found = (get)(g, CString::new("Gamma").unwrap().as_ptr());
        assert!(not_found.is_null());
        let null_g = (get)(ptr::null_mut(), CString::new("Alpha").unwrap().as_ptr());
        assert!(null_g.is_null());
        (free)(g);
    });
}

#[test]
fn test_delete_node_ref_count() {
    run_both(|lib| unsafe {
        let create: Symbol<CreateGraphFn> = lib.get(b"create_graph\0").unwrap();
        let add_n: Symbol<AddNodeFn> = lib.get(b"add_node\0").unwrap();
        let del: Symbol<DeleteNodeFn> = lib.get(b"delete_node\0").unwrap();
        let free: Symbol<FreeGraphFn> = lib.get(b"free_graph\0").unwrap();

        let g = (create)();
        let a = (add_n)(g, CString::new("A").unwrap().as_ptr());
        assert_eq!((*a).ref_count, 1);
        // Manually bump ref_count to test decrement without free
        (*a).ref_count = 3;
        (del)(a);
        assert_eq!((*a).ref_count, 2);
        (del)(a);
        assert_eq!((*a).ref_count, 1);
        // Don't call delete again - free_graph will handle it
        // But free_graph calls delete_node which will decrement to 0 and free
        // We need to be careful - let's just leak the graph to avoid double-free
        // Actually free_graph decrements ref_count. At 1, it will go to 0 and free. That's fine.
        (free)(g);
    });
}

#[test]
fn test_delete_node_null() {
    run_both(|lib| unsafe {
        let del: Symbol<DeleteNodeFn> = lib.get(b"delete_node\0").unwrap();
        (del)(ptr::null_mut()); // should not crash
    });
}

#[test]
fn test_shallow_copy_increments_refs() {
    run_both(|lib| unsafe {
        let create: Symbol<CreateGraphFn> = lib.get(b"create_graph\0").unwrap();
        let add_n: Symbol<AddNodeFn> = lib.get(b"add_node\0").unwrap();
        let add_e: Symbol<AddEdgeFn> = lib.get(b"add_edge\0").unwrap();
        let scopy: Symbol<ShallowCopyFn> = lib.get(b"shallow_copy\0").unwrap();
        let free: Symbol<FreeGraphFn> = lib.get(b"free_graph\0").unwrap();

        let g = (create)();
        let a = (add_n)(g, CString::new("A").unwrap().as_ptr());
        let b = (add_n)(g, CString::new("B").unwrap().as_ptr());
        let c = (add_n)(g, CString::new("C").unwrap().as_ptr());
        (add_e)(a, b, 1);
        (add_e)(b, c, 2);

        // Before shallow_copy: all ref_counts = 1
        assert_eq!((*a).ref_count, 1);
        assert_eq!((*b).ref_count, 1);
        assert_eq!((*c).ref_count, 1);

        let result = (scopy)(a);
        assert_eq!(result, a); // returns same pointer

        // After shallow_copy from A: A, B, C all reachable -> ref_count = 2
        assert_eq!((*a).ref_count, 2);
        assert_eq!((*b).ref_count, 2);
        assert_eq!((*c).ref_count, 2);

        // Decrement the extra refs so free_graph works cleanly
        (*a).ref_count = 1;
        (*b).ref_count = 1;
        (*c).ref_count = 1;
        (free)(g);
    });
}

#[test]
fn test_shallow_copy_null() {
    run_both(|lib| unsafe {
        let scopy: Symbol<ShallowCopyFn> = lib.get(b"shallow_copy\0").unwrap();
        let result = (scopy)(ptr::null_mut());
        assert!(result.is_null());
    });
}

#[test]
fn test_find_shortest_path_basic() {
    run_both(|lib| unsafe {
        let create: Symbol<CreateGraphFn> = lib.get(b"create_graph\0").unwrap();
        let add_n: Symbol<AddNodeFn> = lib.get(b"add_node\0").unwrap();
        let add_e: Symbol<AddEdgeFn> = lib.get(b"add_edge\0").unwrap();
        let fsp: Symbol<FindShortestPathFn> = lib.get(b"find_shortest_path\0").unwrap();
        let free: Symbol<FreeGraphFn> = lib.get(b"free_graph\0").unwrap();

        let (g, nodes) = build_test_graph(&create, &add_n, &add_e);
        // Graph: A->B(5), A->C(2), B->D(3), C->B(1), C->D(7)
        // Shortest A->D: A->C(2)->B(1)->D(3) = 6

        let mut path_len: c_int = 0;
        let path = (fsp)(nodes[0], nodes[3], &mut path_len);
        assert!(!path.is_null());
        assert_eq!(path_len, 4); // A -> C -> B -> D

        let mut names = Vec::new();
        for i in 0..path_len as usize {
            names.push(node_name(*path.add(i)));
        }
        assert_eq!(names, vec!["A", "C", "B", "D"]);

        // Free the path (it was malloc'd)
        libc_free(path as *mut std::ffi::c_void);
        (free)(g);
    });
}

#[test]
fn test_find_shortest_path_direct() {
    run_both(|lib| unsafe {
        let create: Symbol<CreateGraphFn> = lib.get(b"create_graph\0").unwrap();
        let add_n: Symbol<AddNodeFn> = lib.get(b"add_node\0").unwrap();
        let add_e: Symbol<AddEdgeFn> = lib.get(b"add_edge\0").unwrap();
        let fsp: Symbol<FindShortestPathFn> = lib.get(b"find_shortest_path\0").unwrap();
        let free: Symbol<FreeGraphFn> = lib.get(b"free_graph\0").unwrap();

        let g = (create)();
        let a = (add_n)(g, CString::new("X").unwrap().as_ptr());
        let b = (add_n)(g, CString::new("Y").unwrap().as_ptr());
        (add_e)(a, b, 42);

        let mut path_len: c_int = 0;
        let path = (fsp)(a, b, &mut path_len);
        assert!(!path.is_null());
        assert_eq!(path_len, 2);
        assert_eq!(node_name(*path.add(0)), "X");
        assert_eq!(node_name(*path.add(1)), "Y");

        libc_free(path as *mut std::ffi::c_void);
        (free)(g);
    });
}

#[test]
fn test_find_shortest_path_no_path() {
    run_both(|lib| unsafe {
        let create: Symbol<CreateGraphFn> = lib.get(b"create_graph\0").unwrap();
        let add_n: Symbol<AddNodeFn> = lib.get(b"add_node\0").unwrap();
        let fsp: Symbol<FindShortestPathFn> = lib.get(b"find_shortest_path\0").unwrap();
        let free: Symbol<FreeGraphFn> = lib.get(b"free_graph\0").unwrap();

        let g = (create)();
        let a = (add_n)(g, CString::new("A").unwrap().as_ptr());
        let b = (add_n)(g, CString::new("B").unwrap().as_ptr());
        // No edge between them

        let mut path_len: c_int = 0;
        let path = (fsp)(a, b, &mut path_len);
        assert!(path.is_null());
        assert_eq!(path_len, 0);
        (free)(g);
    });
}

#[test]
fn test_find_shortest_path_null_params() {
    run_both(|lib| unsafe {
        let fsp: Symbol<FindShortestPathFn> = lib.get(b"find_shortest_path\0").unwrap();
        let result = (fsp)(ptr::null_mut(), ptr::null_mut(), ptr::null_mut());
        assert!(result.is_null());
    });
}

#[test]
fn test_find_shortest_path_same_node() {
    run_both(|lib| unsafe {
        let create: Symbol<CreateGraphFn> = lib.get(b"create_graph\0").unwrap();
        let add_n: Symbol<AddNodeFn> = lib.get(b"add_node\0").unwrap();
        let fsp: Symbol<FindShortestPathFn> = lib.get(b"find_shortest_path\0").unwrap();
        let free: Symbol<FreeGraphFn> = lib.get(b"free_graph\0").unwrap();

        let g = (create)();
        let a = (add_n)(g, CString::new("A").unwrap().as_ptr());

        let mut path_len: c_int = 0;
        let path = (fsp)(a, a, &mut path_len);
        assert!(!path.is_null());
        assert_eq!(path_len, 1);
        assert_eq!(node_name(*path.add(0)), "A");

        libc_free(path as *mut std::ffi::c_void);
        (free)(g);
    });
}

#[test]
fn test_free_graph_null() {
    run_both(|lib| unsafe {
        let free: Symbol<FreeGraphFn> = lib.get(b"free_graph\0").unwrap();
        (free)(ptr::null_mut()); // should not crash
    });
}

#[test]
fn test_multiple_nodes_and_edges() {
    run_both(|lib| unsafe {
        let create: Symbol<CreateGraphFn> = lib.get(b"create_graph\0").unwrap();
        let add_n: Symbol<AddNodeFn> = lib.get(b"add_node\0").unwrap();
        let add_e: Symbol<AddEdgeFn> = lib.get(b"add_edge\0").unwrap();
        let get: Symbol<GetNodeByNameFn> = lib.get(b"get_node_by_name\0").unwrap();
        let free: Symbol<FreeGraphFn> = lib.get(b"free_graph\0").unwrap();

        let g = (create)();
        let cities = ["NYC", "LA", "Chicago", "Houston", "Phoenix"];
        for c in &cities {
            let cs = CString::new(*c).unwrap();
            let n = (add_n)(g, cs.as_ptr());
            assert!(!n.is_null());
        }
        assert_eq!((*g).node_count, 5);

        // Verify all findable
        for c in &cities {
            let cs = CString::new(*c).unwrap();
            let n = (get)(g, cs.as_ptr());
            assert!(!n.is_null());
            assert_eq!(node_name(n), *c);
        }

        // Add edges: NYC->LA(100), LA->Chicago(50), NYC->Chicago(80)
        let nyc = (get)(g, CString::new("NYC").unwrap().as_ptr());
        let la = (get)(g, CString::new("LA").unwrap().as_ptr());
        let chi = (get)(g, CString::new("Chicago").unwrap().as_ptr());
        assert_eq!((add_e)(nyc, la, 100), 0);
        assert_eq!((add_e)(la, chi, 50), 0);
        assert_eq!((add_e)(nyc, chi, 80), 0);

        assert_eq!((*nyc).edge_count, 2);
        assert_eq!((*la).edge_count, 1);
        (free)(g);
    });
}

#[test]
fn test_add_edge_max_edges() {
    run_both(|lib| unsafe {
        let create: Symbol<CreateGraphFn> = lib.get(b"create_graph\0").unwrap();
        let add_n: Symbol<AddNodeFn> = lib.get(b"add_node\0").unwrap();
        let add_e: Symbol<AddEdgeFn> = lib.get(b"add_edge\0").unwrap();
        let free: Symbol<FreeGraphFn> = lib.get(b"free_graph\0").unwrap();

        let g = (create)();
        let from = (add_n)(g, CString::new("From").unwrap().as_ptr());
        // Create MAX_EDGES destination nodes and add edges
        let mut dests = Vec::new();
        for i in 0..10 {
            let name = CString::new(format!("D{}", i)).unwrap();
            dests.push((add_n)(g, name.as_ptr()));
        }
        for i in 0..10 {
            assert_eq!((add_e)(from, dests[i], i as c_int), 0);
        }
        // 11th edge should fail
        let extra = (add_n)(g, CString::new("Extra").unwrap().as_ptr());
        assert_eq!((add_e)(from, extra, 99), -1);
        (free)(g);
    });
}

#[test]
fn test_long_city_name_truncation() {
    run_both(|lib| unsafe {
        let create: Symbol<CreateGraphFn> = lib.get(b"create_graph\0").unwrap();
        let add_n: Symbol<AddNodeFn> = lib.get(b"add_node\0").unwrap();
        let free: Symbol<FreeGraphFn> = lib.get(b"free_graph\0").unwrap();

        let g = (create)();
        // Name longer than MAX_CITY_NAME (64)
        let long_name = "A".repeat(100);
        let cs = CString::new(long_name).unwrap();
        let n = (add_n)(g, cs.as_ptr());
        assert!(!n.is_null());
        let stored = node_name(n);
        assert_eq!(stored.len(), 63); // truncated to MAX_CITY_NAME - 1
        (free)(g);
    });
}
