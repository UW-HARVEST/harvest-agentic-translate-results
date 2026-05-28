// Integration tests that load BOTH the C and Rust shared libraries via libloading
// and compare their behavior through the FFI boundary.

use libloading::{Library, Symbol};
use std::ffi::CString;
use std::os::raw::{c_char, c_int};
use std::path::PathBuf;

const MAX_CITY_NAME: usize = 64;
const MAX_EDGES: usize = 10;
const MAX_NODES: usize = 100;

#[repr(C)]
#[derive(Copy, Clone)]
struct edge_t {
    destination: *mut node_t,
    distance: c_int,
}

#[repr(C)]
#[derive(Copy, Clone)]
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

type CreateGraphFn = unsafe extern "C" fn() -> *mut graph_t;
type AddNodeFn = unsafe extern "C" fn(*mut graph_t, *const c_char) -> *mut node_t;
type AddEdgeFn = unsafe extern "C" fn(*mut node_t, *mut node_t, c_int) -> c_int;
type DeleteNodeFn = unsafe extern "C" fn(*mut node_t);
type ShallowCopyFn = unsafe extern "C" fn(*mut node_t) -> *mut node_t;
type FindShortestPathFn =
    unsafe extern "C" fn(*mut node_t, *mut node_t, *mut c_int) -> *mut *mut node_t;
type FreeGraphFn = unsafe extern "C" fn(*mut graph_t);
type GetNodeByNameFn = unsafe extern "C" fn(*mut graph_t, *const c_char) -> *mut node_t;

struct Lib {
    _lib: Library,
    create_graph: CreateGraphFn,
    add_node: AddNodeFn,
    add_edge: AddEdgeFn,
    delete_node: DeleteNodeFn,
    shallow_copy: ShallowCopyFn,
    find_shortest_path: FindShortestPathFn,
    free_graph: FreeGraphFn,
    get_node_by_name: GetNodeByNameFn,
}

impl Lib {
    unsafe fn load(path: &str) -> Lib {
        let lib = Library::new(path).expect("failed to load library");
        let create_graph: Symbol<CreateGraphFn> =
            lib.get(b"create_graph").expect("create_graph missing");
        let add_node: Symbol<AddNodeFn> = lib.get(b"add_node").expect("add_node missing");
        let add_edge: Symbol<AddEdgeFn> = lib.get(b"add_edge").expect("add_edge missing");
        let delete_node: Symbol<DeleteNodeFn> =
            lib.get(b"delete_node").expect("delete_node missing");
        let shallow_copy: Symbol<ShallowCopyFn> =
            lib.get(b"shallow_copy").expect("shallow_copy missing");
        let find_shortest_path: Symbol<FindShortestPathFn> = lib
            .get(b"find_shortest_path")
            .expect("find_shortest_path missing");
        let free_graph: Symbol<FreeGraphFn> =
            lib.get(b"free_graph").expect("free_graph missing");
        let get_node_by_name: Symbol<GetNodeByNameFn> = lib
            .get(b"get_node_by_name")
            .expect("get_node_by_name missing");
        let create_graph = *create_graph.into_raw();
        let add_node = *add_node.into_raw();
        let add_edge = *add_edge.into_raw();
        let delete_node = *delete_node.into_raw();
        let shallow_copy = *shallow_copy.into_raw();
        let find_shortest_path = *find_shortest_path.into_raw();
        let free_graph = *free_graph.into_raw();
        let get_node_by_name = *get_node_by_name.into_raw();
        Lib {
            _lib: lib,
            create_graph,
            add_node,
            add_edge,
            delete_node,
            shallow_copy,
            find_shortest_path,
            free_graph,
            get_node_by_name,
        }
    }
}

fn c_lib_path() -> String {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("c_src")
        .join("build")
        .join("libdag_c.so");
    p.to_string_lossy().into_owned()
}

fn rust_lib_path() -> String {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    // Try the current profile first (debug), fall back to release.
    let candidates = ["debug", "release"];
    for p in &candidates {
        let path = manifest.join("target").join(p).join("libtranslated_rust.so");
        if path.exists() {
            return path.to_string_lossy().into_owned();
        }
    }
    // Last resort: assume debug even if missing.
    manifest
        .join("target")
        .join("debug")
        .join("libtranslated_rust.so")
        .to_string_lossy()
        .into_owned()
}

unsafe fn cstring_from_slice(buf: &[c_char]) -> String {
    let mut bytes: Vec<u8> = Vec::with_capacity(buf.len());
    for &b in buf.iter() {
        if b == 0 {
            break;
        }
        bytes.push(b as u8);
    }
    String::from_utf8_lossy(&bytes).into_owned()
}

unsafe fn assert_node_eq(c_node: *mut node_t, r_node: *mut node_t, ctx: &str) {
    if c_node.is_null() && r_node.is_null() {
        return;
    }
    assert!(!c_node.is_null(), "{}: c null but rust not null", ctx);
    assert!(!r_node.is_null(), "{}: rust null but c not null", ctx);

    let c_name = cstring_from_slice(&(*c_node).city_name);
    let r_name = cstring_from_slice(&(*r_node).city_name);
    assert_eq!(c_name, r_name, "{}: city_name mismatch", ctx);
    assert_eq!(
        (*c_node).ref_count,
        (*r_node).ref_count,
        "{}: ref_count mismatch ({}: c={}, r={})",
        ctx,
        c_name,
        (*c_node).ref_count,
        (*r_node).ref_count
    );
    assert_eq!(
        (*c_node).edge_count,
        (*r_node).edge_count,
        "{}: edge_count mismatch",
        ctx
    );
    for i in 0..(*c_node).edge_count as usize {
        assert_eq!(
            (*c_node).edges[i].distance,
            (*r_node).edges[i].distance,
            "{}: edge[{}].distance mismatch",
            ctx,
            i
        );
        // We compare destinations by city_name since the pointer values differ
        // between the two libraries (independent allocations).
        let c_dest_name = cstring_from_slice(&(*(*c_node).edges[i].destination).city_name);
        let r_dest_name = cstring_from_slice(&(*(*r_node).edges[i].destination).city_name);
        assert_eq!(
            c_dest_name, r_dest_name,
            "{}: edge[{}].destination name mismatch",
            ctx, i
        );
    }
}

#[test]
fn test_create_and_free_graph() {
    unsafe {
        let c = Lib::load(&c_lib_path());
        let r = Lib::load(&rust_lib_path());
        let cg = (c.create_graph)();
        let rg = (r.create_graph)();
        assert!(!cg.is_null());
        assert!(!rg.is_null());
        assert_eq!((*cg).node_count, 0);
        assert_eq!((*rg).node_count, 0);
        (c.free_graph)(cg);
        (r.free_graph)(rg);
    }
}

#[test]
fn test_add_node_basic() {
    unsafe {
        let c = Lib::load(&c_lib_path());
        let r = Lib::load(&rust_lib_path());

        let cg = (c.create_graph)();
        let rg = (r.create_graph)();

        let names = ["Boston", "NYC", "Chicago", "LA"];
        for name in names.iter() {
            let cs = CString::new(*name).unwrap();
            let cn = (c.add_node)(cg, cs.as_ptr());
            let rn = (r.add_node)(rg, cs.as_ptr());
            assert_node_eq(cn, rn, name);
        }

        assert_eq!((*cg).node_count, (*rg).node_count);

        (c.free_graph)(cg);
        (r.free_graph)(rg);
    }
}

#[test]
fn test_add_node_duplicate() {
    unsafe {
        let c = Lib::load(&c_lib_path());
        let r = Lib::load(&rust_lib_path());

        let cg = (c.create_graph)();
        let rg = (r.create_graph)();

        let cs = CString::new("Boston").unwrap();
        let _ = (c.add_node)(cg, cs.as_ptr());
        let _ = (r.add_node)(rg, cs.as_ptr());

        let cn = (c.add_node)(cg, cs.as_ptr());
        let rn = (r.add_node)(rg, cs.as_ptr());
        assert!(cn.is_null());
        assert!(rn.is_null());
        assert_eq!((*cg).node_count, (*rg).node_count);

        (c.free_graph)(cg);
        (r.free_graph)(rg);
    }
}

#[test]
fn test_add_node_truncation() {
    unsafe {
        let c = Lib::load(&c_lib_path());
        let r = Lib::load(&rust_lib_path());

        let cg = (c.create_graph)();
        let rg = (r.create_graph)();

        // Make a name longer than MAX_CITY_NAME-1 (63) bytes
        let long_name = "A".repeat(100);
        let cs = CString::new(long_name).unwrap();
        let cn = (c.add_node)(cg, cs.as_ptr());
        let rn = (r.add_node)(rg, cs.as_ptr());
        assert_node_eq(cn, rn, "truncated");

        (c.free_graph)(cg);
        (r.free_graph)(rg);
    }
}

#[test]
fn test_add_edge_basic() {
    unsafe {
        let c = Lib::load(&c_lib_path());
        let r = Lib::load(&rust_lib_path());

        let cg = (c.create_graph)();
        let rg = (r.create_graph)();

        let a = CString::new("A").unwrap();
        let b = CString::new("B").unwrap();
        let ca = (c.add_node)(cg, a.as_ptr());
        let cb = (c.add_node)(cg, b.as_ptr());
        let ra = (r.add_node)(rg, a.as_ptr());
        let rb = (r.add_node)(rg, b.as_ptr());

        let cr = (c.add_edge)(ca, cb, 5);
        let rr = (r.add_edge)(ra, rb, 5);
        assert_eq!(cr, rr);
        assert_node_eq(ca, ra, "A after edge");

        // duplicate edge
        let cr2 = (c.add_edge)(ca, cb, 7);
        let rr2 = (r.add_edge)(ra, rb, 7);
        assert_eq!(cr2, rr2);

        // negative distance
        let cr3 = (c.add_edge)(ca, cb, -1);
        let rr3 = (r.add_edge)(ra, rb, -1);
        assert_eq!(cr3, rr3);

        // null
        let cr4 = (c.add_edge)(std::ptr::null_mut(), cb, 5);
        let rr4 = (r.add_edge)(std::ptr::null_mut(), rb, 5);
        assert_eq!(cr4, rr4);

        (c.free_graph)(cg);
        (r.free_graph)(rg);
    }
}

#[test]
fn test_add_edge_max() {
    unsafe {
        let c = Lib::load(&c_lib_path());
        let r = Lib::load(&rust_lib_path());

        let cg = (c.create_graph)();
        let rg = (r.create_graph)();

        let mut c_nodes: Vec<*mut node_t> = Vec::new();
        let mut r_nodes: Vec<*mut node_t> = Vec::new();
        for i in 0..(MAX_EDGES + 2) {
            let name = format!("N{}", i);
            let cs = CString::new(name).unwrap();
            c_nodes.push((c.add_node)(cg, cs.as_ptr()));
            r_nodes.push((r.add_node)(rg, cs.as_ptr()));
        }

        for i in 1..(MAX_EDGES + 2) {
            let cr = (c.add_edge)(c_nodes[0], c_nodes[i], i as c_int);
            let rr = (r.add_edge)(r_nodes[0], r_nodes[i], i as c_int);
            assert_eq!(cr, rr, "edge {}", i);
        }
        assert_eq!((*c_nodes[0]).edge_count, (*r_nodes[0]).edge_count);

        (c.free_graph)(cg);
        (r.free_graph)(rg);
    }
}

#[test]
fn test_get_node_by_name() {
    unsafe {
        let c = Lib::load(&c_lib_path());
        let r = Lib::load(&rust_lib_path());

        let cg = (c.create_graph)();
        let rg = (r.create_graph)();

        for n in ["X", "Y", "Z"].iter() {
            let cs = CString::new(*n).unwrap();
            (c.add_node)(cg, cs.as_ptr());
            (r.add_node)(rg, cs.as_ptr());
        }

        let cs = CString::new("Y").unwrap();
        let cn = (c.get_node_by_name)(cg, cs.as_ptr());
        let rn = (r.get_node_by_name)(rg, cs.as_ptr());
        assert_node_eq(cn, rn, "Y lookup");

        let cs2 = CString::new("missing").unwrap();
        let cn2 = (c.get_node_by_name)(cg, cs2.as_ptr());
        let rn2 = (r.get_node_by_name)(rg, cs2.as_ptr());
        assert!(cn2.is_null());
        assert!(rn2.is_null());

        // null graph or name
        let cn3 = (c.get_node_by_name)(std::ptr::null_mut(), cs.as_ptr());
        let rn3 = (r.get_node_by_name)(std::ptr::null_mut(), cs.as_ptr());
        assert!(cn3.is_null());
        assert!(rn3.is_null());

        (c.free_graph)(cg);
        (r.free_graph)(rg);
    }
}

#[test]
fn test_delete_node_decrements() {
    unsafe {
        let c = Lib::load(&c_lib_path());
        let r = Lib::load(&rust_lib_path());

        let cg = (c.create_graph)();
        let rg = (r.create_graph)();

        let cs = CString::new("A").unwrap();
        let ca = (c.add_node)(cg, cs.as_ptr());
        let ra = (r.add_node)(rg, cs.as_ptr());
        // Bump ref_count so we can decrement without freeing
        (*ca).ref_count = 5;
        (*ra).ref_count = 5;

        (c.delete_node)(ca);
        (r.delete_node)(ra);
        assert_eq!((*ca).ref_count, (*ra).ref_count);
        assert_eq!((*ca).ref_count, 4);

        // null
        (c.delete_node)(std::ptr::null_mut());
        (r.delete_node)(std::ptr::null_mut());

        (c.free_graph)(cg);
        (r.free_graph)(rg);
    }
}

#[test]
fn test_shallow_copy() {
    unsafe {
        let c = Lib::load(&c_lib_path());
        let r = Lib::load(&rust_lib_path());

        let cg = (c.create_graph)();
        let rg = (r.create_graph)();

        // Build A -> B -> C, A -> C; copy starting at A.
        let names = ["A", "B", "C"];
        let mut cn = Vec::new();
        let mut rn = Vec::new();
        for n in names.iter() {
            let cs = CString::new(*n).unwrap();
            cn.push((c.add_node)(cg, cs.as_ptr()));
            rn.push((r.add_node)(rg, cs.as_ptr()));
        }
        (c.add_edge)(cn[0], cn[1], 1);
        (r.add_edge)(rn[0], rn[1], 1);
        (c.add_edge)(cn[1], cn[2], 2);
        (r.add_edge)(rn[1], rn[2], 2);
        (c.add_edge)(cn[0], cn[2], 5);
        (r.add_edge)(rn[0], rn[2], 5);

        let cc = (c.shallow_copy)(cn[0]);
        let rc = (r.shallow_copy)(rn[0]);
        assert_eq!(cc as usize == cn[0] as usize, rc as usize == rn[0] as usize);

        // All ref counts should be 2 now (started at 1)
        for i in 0..3 {
            assert_eq!((*cn[i]).ref_count, (*rn[i]).ref_count, "node {}", i);
            assert_eq!((*cn[i]).ref_count, 2, "node {} expected ref_count 2", i);
        }

        // null
        let cc2 = (c.shallow_copy)(std::ptr::null_mut());
        let rc2 = (r.shallow_copy)(std::ptr::null_mut());
        assert!(cc2.is_null() && rc2.is_null());

        (c.free_graph)(cg);
        (r.free_graph)(rg);
    }
}

#[test]
fn test_shallow_copy_cycle() {
    unsafe {
        let c = Lib::load(&c_lib_path());
        let r = Lib::load(&rust_lib_path());

        let cg = (c.create_graph)();
        let rg = (r.create_graph)();

        // A -> B -> A cycle
        let mut cn = Vec::new();
        let mut rn = Vec::new();
        for n in &["A", "B"] {
            let cs = CString::new(*n).unwrap();
            cn.push((c.add_node)(cg, cs.as_ptr()));
            rn.push((r.add_node)(rg, cs.as_ptr()));
        }
        (c.add_edge)(cn[0], cn[1], 1);
        (r.add_edge)(rn[0], rn[1], 1);
        (c.add_edge)(cn[1], cn[0], 2);
        (r.add_edge)(rn[1], rn[0], 2);

        (c.shallow_copy)(cn[0]);
        (r.shallow_copy)(rn[0]);

        for i in 0..2 {
            assert_eq!((*cn[i]).ref_count, (*rn[i]).ref_count);
            assert_eq!((*cn[i]).ref_count, 2);
        }

        (c.free_graph)(cg);
        (r.free_graph)(rg);
    }
}

#[test]
fn test_find_shortest_path_simple() {
    unsafe {
        let c = Lib::load(&c_lib_path());
        let r = Lib::load(&rust_lib_path());

        let cg = (c.create_graph)();
        let rg = (r.create_graph)();

        // A -> B (1), A -> C (5), B -> C (1)
        // shortest A->C is A->B->C
        let names = ["A", "B", "C"];
        let mut cn = Vec::new();
        let mut rn = Vec::new();
        for n in names.iter() {
            let cs = CString::new(*n).unwrap();
            cn.push((c.add_node)(cg, cs.as_ptr()));
            rn.push((r.add_node)(rg, cs.as_ptr()));
        }
        (c.add_edge)(cn[0], cn[1], 1);
        (r.add_edge)(rn[0], rn[1], 1);
        (c.add_edge)(cn[0], cn[2], 5);
        (r.add_edge)(rn[0], rn[2], 5);
        (c.add_edge)(cn[1], cn[2], 1);
        (r.add_edge)(rn[1], rn[2], 1);

        let mut c_len: c_int = 0;
        let mut r_len: c_int = 0;
        let cp = (c.find_shortest_path)(cn[0], cn[2], &mut c_len);
        let rp = (r.find_shortest_path)(rn[0], rn[2], &mut r_len);
        assert!(!cp.is_null());
        assert!(!rp.is_null());
        assert_eq!(c_len, r_len);
        for i in 0..c_len as isize {
            let cn_p = *cp.offset(i);
            let rn_p = *rp.offset(i);
            let c_name = cstring_from_slice(&(*cn_p).city_name);
            let r_name = cstring_from_slice(&(*rn_p).city_name);
            assert_eq!(c_name, r_name, "path step {}", i);
        }

        libc::free(cp as *mut libc::c_void);
        libc::free(rp as *mut libc::c_void);

        (c.free_graph)(cg);
        (r.free_graph)(rg);
    }
}

#[test]
fn test_find_shortest_path_no_path() {
    unsafe {
        let c = Lib::load(&c_lib_path());
        let r = Lib::load(&rust_lib_path());

        let cg = (c.create_graph)();
        let rg = (r.create_graph)();

        // A and B disconnected
        let names = ["A", "B"];
        let mut cn = Vec::new();
        let mut rn = Vec::new();
        for n in names.iter() {
            let cs = CString::new(*n).unwrap();
            cn.push((c.add_node)(cg, cs.as_ptr()));
            rn.push((r.add_node)(rg, cs.as_ptr()));
        }

        let mut c_len: c_int = -42;
        let mut r_len: c_int = -42;
        let cp = (c.find_shortest_path)(cn[0], cn[1], &mut c_len);
        let rp = (r.find_shortest_path)(rn[0], rn[1], &mut r_len);
        assert!(cp.is_null());
        assert!(rp.is_null());
        assert_eq!(c_len, r_len);
        assert_eq!(c_len, 0);

        (c.free_graph)(cg);
        (r.free_graph)(rg);
    }
}

#[test]
fn test_find_shortest_path_same_node() {
    unsafe {
        let c = Lib::load(&c_lib_path());
        let r = Lib::load(&rust_lib_path());

        let cg = (c.create_graph)();
        let rg = (r.create_graph)();

        let cs = CString::new("A").unwrap();
        let ca = (c.add_node)(cg, cs.as_ptr());
        let ra = (r.add_node)(rg, cs.as_ptr());

        let mut c_len: c_int = 0;
        let mut r_len: c_int = 0;
        let cp = (c.find_shortest_path)(ca, ca, &mut c_len);
        let rp = (r.find_shortest_path)(ra, ra, &mut r_len);
        assert!(!cp.is_null());
        assert!(!rp.is_null());
        assert_eq!(c_len, r_len);
        assert_eq!(c_len, 1);

        libc::free(cp as *mut libc::c_void);
        libc::free(rp as *mut libc::c_void);

        (c.free_graph)(cg);
        (r.free_graph)(rg);
    }
}

#[test]
fn test_find_shortest_path_null() {
    unsafe {
        let c = Lib::load(&c_lib_path());
        let r = Lib::load(&rust_lib_path());

        let mut c_len: c_int = 7;
        let mut r_len: c_int = 7;
        let cp = (c.find_shortest_path)(std::ptr::null_mut(), std::ptr::null_mut(), &mut c_len);
        let rp = (r.find_shortest_path)(std::ptr::null_mut(), std::ptr::null_mut(), &mut r_len);
        assert!(cp.is_null());
        assert!(rp.is_null());
        // C function does NOT update *path_length when start/end NULL (returns early)
        assert_eq!(c_len, r_len);
    }
}

#[test]
fn test_find_shortest_path_complex() {
    unsafe {
        let c = Lib::load(&c_lib_path());
        let r = Lib::load(&rust_lib_path());

        let cg = (c.create_graph)();
        let rg = (r.create_graph)();

        // 5-node graph
        let names = ["A", "B", "C", "D", "E"];
        let mut cn = Vec::new();
        let mut rn = Vec::new();
        for n in names.iter() {
            let cs = CString::new(*n).unwrap();
            cn.push((c.add_node)(cg, cs.as_ptr()));
            rn.push((r.add_node)(rg, cs.as_ptr()));
        }
        // edges
        let edges = [
            (0, 1, 4), (0, 2, 1), (2, 1, 2), (1, 3, 1), (2, 3, 5), (3, 4, 3), (2, 4, 8),
        ];
        for (f, t, d) in edges.iter() {
            (c.add_edge)(cn[*f], cn[*t], *d);
            (r.add_edge)(rn[*f], rn[*t], *d);
        }

        let mut c_len: c_int = 0;
        let mut r_len: c_int = 0;
        let cp = (c.find_shortest_path)(cn[0], cn[4], &mut c_len);
        let rp = (r.find_shortest_path)(rn[0], rn[4], &mut r_len);
        assert!(!cp.is_null());
        assert!(!rp.is_null());
        assert_eq!(c_len, r_len);
        for i in 0..c_len as isize {
            let cnp = *cp.offset(i);
            let rnp = *rp.offset(i);
            let c_name = cstring_from_slice(&(*cnp).city_name);
            let r_name = cstring_from_slice(&(*rnp).city_name);
            assert_eq!(c_name, r_name);
        }

        libc::free(cp as *mut libc::c_void);
        libc::free(rp as *mut libc::c_void);

        (c.free_graph)(cg);
        (r.free_graph)(rg);
    }
}

#[test]
fn test_print_node_and_graph() {
    // Just exercise to ensure no crash. We don't capture stdout in this test.
    unsafe {
        let c = Lib::load(&c_lib_path());
        let r = Lib::load(&rust_lib_path());

        let cg = (c.create_graph)();
        let rg = (r.create_graph)();

        let names = ["X", "Y"];
        for n in names.iter() {
            let cs = CString::new(*n).unwrap();
            (c.add_node)(cg, cs.as_ptr());
            (r.add_node)(rg, cs.as_ptr());
        }

        // Don't print to pollute test output, but also don't fail.
        // We can verify by reading the symbols exist.
        let _: Symbol<unsafe extern "C" fn(*mut node_t)> = c._lib.get(b"print_node").unwrap();
        let _: Symbol<unsafe extern "C" fn(*mut node_t)> = r._lib.get(b"print_node").unwrap();
        let _: Symbol<unsafe extern "C" fn(*mut graph_t)> = c._lib.get(b"print_graph").unwrap();
        let _: Symbol<unsafe extern "C" fn(*mut graph_t)> = r._lib.get(b"print_graph").unwrap();

        (c.free_graph)(cg);
        (r.free_graph)(rg);
    }
}
