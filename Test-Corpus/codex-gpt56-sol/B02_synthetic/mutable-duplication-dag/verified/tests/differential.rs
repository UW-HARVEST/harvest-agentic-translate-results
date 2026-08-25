use libloading::Library;
use std::ffi::{c_char, c_int, c_void, CStr, CString};
use std::fs::File;
use std::io::Read;
use std::os::fd::{FromRawFd, RawFd};
use std::path::{Path, PathBuf};
use std::ptr;
use std::sync::Mutex;

const MAX_CITY_NAME: usize = 64;
const MAX_EDGES: usize = 10;
const MAX_NODES: usize = 100;
static TEST_LOCK: Mutex<()> = Mutex::new(());

#[repr(C)]
#[derive(Clone, Copy)]
struct Edge {
    destination: *mut Node,
    distance: c_int,
}

#[repr(C)]
struct Node {
    city_name: [c_char; MAX_CITY_NAME],
    ref_count: c_int,
    edges: [Edge; MAX_EDGES],
    edge_count: c_int,
}

#[repr(C)]
struct Graph {
    nodes: [*mut Node; MAX_NODES],
    node_count: c_int,
}

type CreateGraph = unsafe extern "C" fn() -> *mut Graph;
type AddNode = unsafe extern "C" fn(*mut Graph, *const c_char) -> *mut Node;
type AddEdge = unsafe extern "C" fn(*mut Node, *mut Node, c_int) -> c_int;
type DeleteNode = unsafe extern "C" fn(*mut Node);
type ShallowCopy = unsafe extern "C" fn(*mut Node) -> *mut Node;
type FindShortestPath = unsafe extern "C" fn(*mut Node, *mut Node, *mut c_int) -> *mut *mut Node;
type FreeGraph = unsafe extern "C" fn(*mut Graph);
type GetNodeByName = unsafe extern "C" fn(*mut Graph, *const c_char) -> *mut Node;
type PrintNode = unsafe extern "C" fn(*mut Node);
type PrintGraph = unsafe extern "C" fn(*mut Graph);
type SetFailAlloc = unsafe extern "C" fn(isize);

struct Api {
    create_graph: CreateGraph,
    add_node: AddNode,
    add_edge: AddEdge,
    delete_node: DeleteNode,
    shallow_copy: ShallowCopy,
    find_shortest_path: FindShortestPath,
    free_graph: FreeGraph,
    get_node_by_name: GetNodeByName,
    print_node: PrintNode,
    print_graph: PrintGraph,
    set_fail_alloc: Option<SetFailAlloc>,
    _library: Library,
}

impl Api {
    unsafe fn load(path: &Path) -> Self {
        let library = unsafe { Library::new(path) }
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()));
        macro_rules! symbol {
            ($name:literal, $ty:ty) => {
                *unsafe { library.get::<$ty>(concat!($name, "\0").as_bytes()) }
                    .unwrap_or_else(|error| panic!("missing {}: {error}", $name))
            };
        }
        let set_fail_alloc = unsafe {
            library
                .get::<SetFailAlloc>(b"dag_test_fail_alloc_after\0")
                .ok()
                .map(|symbol| *symbol)
        };
        Self {
            create_graph: symbol!("create_graph", CreateGraph),
            add_node: symbol!("add_node", AddNode),
            add_edge: symbol!("add_edge", AddEdge),
            delete_node: symbol!("delete_node", DeleteNode),
            shallow_copy: symbol!("shallow_copy", ShallowCopy),
            find_shortest_path: symbol!("find_shortest_path", FindShortestPath),
            free_graph: symbol!("free_graph", FreeGraph),
            get_node_by_name: symbol!("get_node_by_name", GetNodeByName),
            print_node: symbol!("print_node", PrintNode),
            print_graph: symbol!("print_graph", PrintGraph),
            set_fail_alloc,
            _library: library,
        }
    }
}

unsafe extern "C" {
    fn free(pointer: *mut c_void);
    fn pipe(fds: *mut c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(old_fd: c_int, new_fd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
}

fn rust_library_path() -> PathBuf {
    std::env::current_exe()
        .expect("test executable path")
        .parent()
        .expect("deps directory")
        .parent()
        .expect("profile directory")
        .join("libdriver.so")
}

unsafe fn load_pair() -> (Api, Api) {
    (unsafe { Api::load(Path::new(env!("DAG_C_SO"))) }, unsafe {
        Api::load(&rust_library_path())
    })
}

#[derive(Debug, PartialEq, Eq)]
struct NodeSnapshot {
    name: Vec<u8>,
    ref_count: c_int,
    edges: Vec<(usize, c_int)>,
}

#[derive(Debug, PartialEq, Eq)]
struct GraphSnapshot {
    node_count: c_int,
    unused_are_null: bool,
    nodes: Vec<NodeSnapshot>,
}

unsafe fn graph_snapshot(graph: *mut Graph) -> GraphSnapshot {
    let count = unsafe { (*graph).node_count } as usize;
    let mut nodes = Vec::with_capacity(count);
    for index in 0..count {
        let node = unsafe { (*graph).nodes[index] };
        let name = unsafe { CStr::from_ptr((*node).city_name.as_ptr()) }
            .to_bytes()
            .to_vec();
        let mut edges = Vec::new();
        for edge_index in 0..unsafe { (*node).edge_count } as usize {
            let edge = unsafe { (*node).edges[edge_index] };
            let destination = if edge.destination.is_null() {
                usize::MAX
            } else {
                (0..count)
                    .find(|candidate| unsafe { (*graph).nodes[*candidate] } == edge.destination)
                    .unwrap_or(MAX_NODES + 1)
            };
            edges.push((destination, edge.distance));
        }
        nodes.push(NodeSnapshot {
            name,
            ref_count: unsafe { (*node).ref_count },
            edges,
        });
    }
    GraphSnapshot {
        node_count: unsafe { (*graph).node_count },
        unused_are_null: (count..MAX_NODES).all(|index| unsafe { (*graph).nodes[index].is_null() }),
        nodes,
    }
}

unsafe fn assert_graphs_equal(c_graph: *mut Graph, rust_graph: *mut Graph) {
    assert_eq!(unsafe { graph_snapshot(c_graph) }, unsafe {
        graph_snapshot(rust_graph)
    });
}

unsafe fn create_pair(c: &Api, rust: &Api) -> (*mut Graph, *mut Graph) {
    let c_graph = unsafe { (c.create_graph)() };
    let rust_graph = unsafe { (rust.create_graph)() };
    assert_eq!(c_graph.is_null(), rust_graph.is_null());
    assert!(!c_graph.is_null());
    (c_graph, rust_graph)
}

unsafe fn add_pair(
    c: &Api,
    rust: &Api,
    c_graph: *mut Graph,
    rust_graph: *mut Graph,
    name: &CString,
) -> (*mut Node, *mut Node) {
    let c_node = unsafe { (c.add_node)(c_graph, name.as_ptr()) };
    let rust_node = unsafe { (rust.add_node)(rust_graph, name.as_ptr()) };
    assert_eq!(c_node.is_null(), rust_node.is_null());
    if !c_node.is_null() {
        unsafe { assert_graphs_equal(c_graph, rust_graph) };
    }
    (c_node, rust_node)
}

struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.0
    }

    fn name(&mut self, length: usize) -> CString {
        let bytes = (0..length)
            .map(|_| b'a' + (self.next() % 26) as u8)
            .collect::<Vec<_>>();
        CString::new(bytes).unwrap()
    }
}

#[test]
fn graph_and_name_surface_matches() {
    let _guard = TEST_LOCK.lock().unwrap();
    unsafe {
        let (c, rust) = load_pair();

        let (c_graph, rust_graph) = create_pair(&c, &rust);
        let c_empty = graph_snapshot(c_graph);
        let rust_empty = graph_snapshot(rust_graph);
        assert_eq!(c_empty, rust_empty);
        assert_eq!(c_empty.node_count, 0);
        assert!(c_empty.unused_are_null);
        (c.free_graph)(c_graph);
        (rust.free_graph)(rust_graph);

        let mut rng = Lcg(0x4d59_5df4_d0f3_3173);
        for &length in &[0, 1, 7, 31, 62, 63, 64, 79, 127] {
            for _ in 0..24 {
                let name = rng.name(length);
                let (c_graph, rust_graph) = create_pair(&c, &rust);
                let (c_node, rust_node) = add_pair(&c, &rust, c_graph, rust_graph, &name);
                assert!(!c_node.is_null());
                assert!(!rust_node.is_null());
                assert_eq!(
                    CStr::from_ptr((*c_node).city_name.as_ptr()).to_bytes(),
                    CStr::from_ptr((*rust_node).city_name.as_ptr()).to_bytes()
                );
                (c.free_graph)(c_graph);
                (rust.free_graph)(rust_graph);
            }
        }

        let (c_graph, rust_graph) = create_pair(&c, &rust);
        for index in 0..12 {
            let name = CString::new(format!("scan-{index:02}")).unwrap();
            add_pair(&c, &rust, c_graph, rust_graph, &name);
        }
        let first = CString::new("scan-00").unwrap();
        let later = CString::new("scan-11").unwrap();
        let c_first = (c.get_node_by_name)(c_graph, first.as_ptr());
        let r_first = (rust.get_node_by_name)(rust_graph, first.as_ptr());
        let c_later = (c.get_node_by_name)(c_graph, later.as_ptr());
        let r_later = (rust.get_node_by_name)(rust_graph, later.as_ptr());
        assert_eq!((*c_first).city_name, (*r_first).city_name);
        assert_eq!((*c_later).city_name, (*r_later).city_name);
        (c.free_graph)(c_graph);
        (rust.free_graph)(rust_graph);

        for _ in 0..24 {
            let length = 64 + (rng.next() % 96) as usize;
            let long_name = rng.name(length);
            let truncated = CString::new(&long_name.as_bytes()[..63]).unwrap();
            let (c_graph, rust_graph) = create_pair(&c, &rust);
            add_pair(&c, &rust, c_graph, rust_graph, &long_name);
            assert_eq!(
                (c.get_node_by_name)(c_graph, long_name.as_ptr()).is_null(),
                (rust.get_node_by_name)(rust_graph, long_name.as_ptr()).is_null()
            );
            assert!((c.get_node_by_name)(c_graph, long_name.as_ptr()).is_null());
            assert_eq!(
                (c.get_node_by_name)(c_graph, truncated.as_ptr()).is_null(),
                (rust.get_node_by_name)(rust_graph, truncated.as_ptr()).is_null()
            );
            assert!(!(c.get_node_by_name)(c_graph, truncated.as_ptr()).is_null());
            let (c_duplicate, rust_duplicate) =
                add_pair(&c, &rust, c_graph, rust_graph, &long_name);
            assert!(!c_duplicate.is_null());
            assert!(!rust_duplicate.is_null());
            assert_eq!((*c_graph).node_count, 2);
            assert_eq!((*rust_graph).node_count, 2);
            (c.free_graph)(c_graph);
            (rust.free_graph)(rust_graph);
        }

        let (c_graph, rust_graph) = create_pair(&c, &rust);
        for index in 0..MAX_NODES {
            let name = CString::new(format!("capacity-{index:03}")).unwrap();
            let (c_node, rust_node) = add_pair(&c, &rust, c_graph, rust_graph, &name);
            assert_eq!(c_node.is_null(), rust_node.is_null());
            assert!(!c_node.is_null());
        }
        assert_eq!((*c_graph).node_count, MAX_NODES as c_int);
        assert_graphs_equal(c_graph, rust_graph);
        let overflow = CString::new("capacity-overflow").unwrap();
        assert_eq!(
            (c.add_node)(c_graph, overflow.as_ptr()).is_null(),
            (rust.add_node)(rust_graph, overflow.as_ptr()).is_null()
        );
        assert!((c.add_node)(c_graph, overflow.as_ptr()).is_null());
        (c.free_graph)(c_graph);
        (rust.free_graph)(rust_graph);

        let (c_graph, rust_graph) = create_pair(&c, &rust);
        let duplicate = CString::new("duplicate").unwrap();
        add_pair(&c, &rust, c_graph, rust_graph, &duplicate);
        assert_eq!(
            (c.add_node)(c_graph, duplicate.as_ptr()).is_null(),
            (rust.add_node)(rust_graph, duplicate.as_ptr()).is_null()
        );
        assert!((c.add_node)(c_graph, duplicate.as_ptr()).is_null());

        assert_eq!(
            (c.add_node)(ptr::null_mut(), duplicate.as_ptr()).is_null(),
            (rust.add_node)(ptr::null_mut(), duplicate.as_ptr()).is_null()
        );
        assert_eq!(
            (c.add_node)(c_graph, ptr::null()).is_null(),
            (rust.add_node)(rust_graph, ptr::null()).is_null()
        );
        assert_eq!(
            (c.get_node_by_name)(ptr::null_mut(), duplicate.as_ptr()).is_null(),
            (rust.get_node_by_name)(ptr::null_mut(), duplicate.as_ptr()).is_null()
        );
        assert_eq!(
            (c.get_node_by_name)(c_graph, ptr::null()).is_null(),
            (rust.get_node_by_name)(rust_graph, ptr::null()).is_null()
        );
        let missing = CString::new("missing").unwrap();
        assert_eq!(
            (c.get_node_by_name)(c_graph, missing.as_ptr()).is_null(),
            (rust.get_node_by_name)(rust_graph, missing.as_ptr()).is_null()
        );
        assert!((c.get_node_by_name)(c_graph, missing.as_ptr()).is_null());
        (c.free_graph)(c_graph);
        (rust.free_graph)(rust_graph);
    }
}

unsafe fn add_named_nodes(
    c: &Api,
    rust: &Api,
    c_graph: *mut Graph,
    rust_graph: *mut Graph,
    count: usize,
) -> (Vec<*mut Node>, Vec<*mut Node>) {
    let mut c_nodes = Vec::with_capacity(count);
    let mut rust_nodes = Vec::with_capacity(count);
    for index in 0..count {
        let name = CString::new(format!("node-{index:03}")).unwrap();
        let (c_node, rust_node) = unsafe { add_pair(c, rust, c_graph, rust_graph, &name) };
        c_nodes.push(c_node);
        rust_nodes.push(rust_node);
    }
    (c_nodes, rust_nodes)
}

unsafe fn add_edge_pair(
    c: &Api,
    rust: &Api,
    c_from: *mut Node,
    c_to: *mut Node,
    rust_from: *mut Node,
    rust_to: *mut Node,
    distance: c_int,
) -> c_int {
    let c_result = unsafe { (c.add_edge)(c_from, c_to, distance) };
    let rust_result = unsafe { (rust.add_edge)(rust_from, rust_to, distance) };
    assert_eq!(c_result, rust_result);
    c_result
}

unsafe fn release_shallow_graph(api: &Api, graph: *mut Graph) {
    let nodes = unsafe { (&(*graph).nodes)[..(*graph).node_count as usize].to_vec() };
    unsafe { (api.free_graph)(graph) };
    for node in nodes {
        unsafe { (api.delete_node)(node) };
    }
}

fn blank_node() -> Box<Node> {
    Box::new(Node {
        city_name: [0; MAX_CITY_NAME],
        ref_count: 1,
        edges: [Edge {
            destination: ptr::null_mut(),
            distance: 0,
        }; MAX_EDGES],
        edge_count: 0,
    })
}

#[test]
fn edge_delete_and_shallow_copy_surface_matches() {
    let _guard = TEST_LOCK.lock().unwrap();
    unsafe {
        let (c, rust) = load_pair();
        let mut rng = Lcg(0xc6a4_a793_5bd1_e995);

        for _ in 0..48 {
            let (c_graph, rust_graph) = create_pair(&c, &rust);
            let (c_nodes, rust_nodes) = add_named_nodes(&c, &rust, c_graph, rust_graph, 2);
            let distance = if rng.next() & 3 == 0 {
                0
            } else {
                (rng.next() % 1_000_000) as c_int + 1
            };
            assert_eq!(
                add_edge_pair(
                    &c,
                    &rust,
                    c_nodes[0],
                    c_nodes[1],
                    rust_nodes[0],
                    rust_nodes[1],
                    distance,
                ),
                0
            );
            assert_graphs_equal(c_graph, rust_graph);
            (c.free_graph)(c_graph);
            (rust.free_graph)(rust_graph);
        }

        for _ in 0..24 {
            let (c_graph, rust_graph) = create_pair(&c, &rust);
            let (c_nodes, rust_nodes) = add_named_nodes(&c, &rust, c_graph, rust_graph, 1);
            let distance = (rng.next() % 10_000) as c_int;
            assert_eq!(
                add_edge_pair(
                    &c,
                    &rust,
                    c_nodes[0],
                    c_nodes[0],
                    rust_nodes[0],
                    rust_nodes[0],
                    distance,
                ),
                0
            );
            assert_graphs_equal(c_graph, rust_graph);
            (c.free_graph)(c_graph);
            (rust.free_graph)(rust_graph);
        }

        let (c_graph, rust_graph) = create_pair(&c, &rust);
        let (c_nodes, rust_nodes) = add_named_nodes(&c, &rust, c_graph, rust_graph, MAX_EDGES + 1);
        for destination in 1..=MAX_EDGES {
            assert_eq!(
                add_edge_pair(
                    &c,
                    &rust,
                    c_nodes[0],
                    c_nodes[destination],
                    rust_nodes[0],
                    rust_nodes[destination],
                    destination as c_int,
                ),
                0
            );
        }
        assert_eq!((*c_nodes[0]).edge_count, MAX_EDGES as c_int);
        assert_graphs_equal(c_graph, rust_graph);
        assert_eq!(
            add_edge_pair(
                &c,
                &rust,
                c_nodes[0],
                c_nodes[0],
                rust_nodes[0],
                rust_nodes[0],
                1,
            ),
            -1
        );
        (c.free_graph)(c_graph);
        (rust.free_graph)(rust_graph);

        let (c_graph, rust_graph) = create_pair(&c, &rust);
        let (c_nodes, rust_nodes) = add_named_nodes(&c, &rust, c_graph, rust_graph, 2);
        assert_eq!(
            add_edge_pair(
                &c,
                &rust,
                ptr::null_mut(),
                c_nodes[1],
                ptr::null_mut(),
                rust_nodes[1],
                1,
            ),
            -1
        );
        assert_eq!(
            add_edge_pair(
                &c,
                &rust,
                c_nodes[0],
                ptr::null_mut(),
                rust_nodes[0],
                ptr::null_mut(),
                1,
            ),
            -1
        );
        for distance in [-1, -2, c_int::MIN] {
            assert_eq!(
                add_edge_pair(
                    &c,
                    &rust,
                    c_nodes[0],
                    c_nodes[1],
                    rust_nodes[0],
                    rust_nodes[1],
                    distance,
                ),
                -1
            );
        }
        assert_eq!(
            add_edge_pair(
                &c,
                &rust,
                c_nodes[0],
                c_nodes[1],
                rust_nodes[0],
                rust_nodes[1],
                9,
            ),
            0
        );
        assert_eq!(
            add_edge_pair(
                &c,
                &rust,
                c_nodes[0],
                c_nodes[1],
                rust_nodes[0],
                rust_nodes[1],
                10,
            ),
            -1
        );
        assert_graphs_equal(c_graph, rust_graph);
        (c.delete_node)(ptr::null_mut());
        (rust.delete_node)(ptr::null_mut());

        assert_eq!((c.shallow_copy)(c_nodes[0]), c_nodes[0]);
        assert_eq!((rust.shallow_copy)(rust_nodes[0]), rust_nodes[0]);
        assert_eq!((*c_nodes[0]).ref_count, 2);
        assert_eq!((*rust_nodes[0]).ref_count, 2);
        (c.delete_node)(c_nodes[0]);
        (rust.delete_node)(rust_nodes[0]);
        assert_eq!((*c_nodes[0]).ref_count, 1);
        assert_eq!((*rust_nodes[0]).ref_count, 1);
        (c.free_graph)(c_graph);
        (rust.free_graph)(rust_graph);

        assert!((c.shallow_copy)(ptr::null_mut()).is_null());
        assert!((rust.shallow_copy)(ptr::null_mut()).is_null());

        for _ in 0..24 {
            let length = 2 + (rng.next() % 12) as usize;
            let (c_graph, rust_graph) = create_pair(&c, &rust);
            let (c_nodes, rust_nodes) = add_named_nodes(&c, &rust, c_graph, rust_graph, length);
            for index in 0..length - 1 {
                add_edge_pair(
                    &c,
                    &rust,
                    c_nodes[index],
                    c_nodes[index + 1],
                    rust_nodes[index],
                    rust_nodes[index + 1],
                    (rng.next() % 1000) as c_int,
                );
            }
            assert_eq!((c.shallow_copy)(c_nodes[0]), c_nodes[0]);
            assert_eq!((rust.shallow_copy)(rust_nodes[0]), rust_nodes[0]);
            assert_graphs_equal(c_graph, rust_graph);
            assert!(c_nodes.iter().all(|node| (**node).ref_count == 2));
            release_shallow_graph(&c, c_graph);
            release_shallow_graph(&rust, rust_graph);
        }

        let (c_graph, rust_graph) = create_pair(&c, &rust);
        let (c_nodes, rust_nodes) = add_named_nodes(&c, &rust, c_graph, rust_graph, 4);
        for &(from, to) in &[(0, 1), (0, 2), (1, 3), (2, 3)] {
            add_edge_pair(
                &c,
                &rust,
                c_nodes[from],
                c_nodes[to],
                rust_nodes[from],
                rust_nodes[to],
                1,
            );
        }
        (c.shallow_copy)(c_nodes[0]);
        (rust.shallow_copy)(rust_nodes[0]);
        assert_graphs_equal(c_graph, rust_graph);
        assert_eq!((*c_nodes[3]).ref_count, 2);
        release_shallow_graph(&c, c_graph);
        release_shallow_graph(&rust, rust_graph);

        for edges in [
            vec![(0, 0)],
            vec![(0, 1), (1, 0)],
            vec![(0, 1), (1, 2), (2, 0)],
        ] {
            let node_count = edges
                .iter()
                .map(|(from, to)| (*from).max(*to))
                .max()
                .unwrap()
                + 1;
            let (c_graph, rust_graph) = create_pair(&c, &rust);
            let (c_nodes, rust_nodes) = add_named_nodes(&c, &rust, c_graph, rust_graph, node_count);
            for &(from, to) in &edges {
                add_edge_pair(
                    &c,
                    &rust,
                    c_nodes[from],
                    c_nodes[to],
                    rust_nodes[from],
                    rust_nodes[to],
                    1,
                );
            }
            (c.shallow_copy)(c_nodes[0]);
            (rust.shallow_copy)(rust_nodes[0]);
            assert_graphs_equal(c_graph, rust_graph);
            assert!(c_nodes.iter().all(|node| (**node).ref_count == 2));
            release_shallow_graph(&c, c_graph);
            release_shallow_graph(&rust, rust_graph);
        }

        let mut c_null = blank_node();
        let mut rust_null = blank_node();
        c_null.edge_count = 1;
        rust_null.edge_count = 1;
        (c.shallow_copy)(&mut *c_null);
        (rust.shallow_copy)(&mut *rust_null);
        assert_eq!(c_null.ref_count, rust_null.ref_count);
        assert_eq!(c_null.ref_count, 2);

        let mut c_chain = (0..=MAX_NODES).map(|_| blank_node()).collect::<Vec<_>>();
        let mut rust_chain = (0..=MAX_NODES).map(|_| blank_node()).collect::<Vec<_>>();
        for index in 0..MAX_NODES {
            c_chain[index].edges[0].destination = &mut *c_chain[index + 1];
            c_chain[index].edge_count = 1;
            rust_chain[index].edges[0].destination = &mut *rust_chain[index + 1];
            rust_chain[index].edge_count = 1;
        }
        (c.shallow_copy)(&mut *c_chain[0]);
        (rust.shallow_copy)(&mut *rust_chain[0]);
        assert_eq!(
            c_chain
                .iter()
                .map(|node| node.ref_count)
                .collect::<Vec<_>>(),
            rust_chain
                .iter()
                .map(|node| node.ref_count)
                .collect::<Vec<_>>()
        );
        assert!(c_chain.iter().all(|node| node.ref_count == 2));

        let mut c_zero = blank_node();
        let mut rust_zero = blank_node();
        c_zero.ref_count = 0;
        rust_zero.ref_count = 0;
        (c.delete_node)(&mut *c_zero);
        (rust.delete_node)(&mut *rust_zero);
        assert_eq!(c_zero.ref_count, rust_zero.ref_count);
        assert_eq!(c_zero.ref_count, -1);

        let (c_graph, rust_graph) = create_pair(&c, &rust);
        let one = CString::new("free-on-zero").unwrap();
        let (c_node, rust_node) = add_pair(&c, &rust, c_graph, rust_graph, &one);
        (*c_graph).node_count = 0;
        (*rust_graph).node_count = 0;
        (c.delete_node)(c_node);
        (rust.delete_node)(rust_node);
        (c.free_graph)(c_graph);
        (rust.free_graph)(rust_graph);
    }
}

unsafe fn path_names(
    api: &Api,
    start: *mut Node,
    end: *mut Node,
    initial_length: c_int,
) -> (Option<Vec<Vec<u8>>>, c_int) {
    let mut length = initial_length;
    let result = unsafe { (api.find_shortest_path)(start, end, &mut length) };
    if result.is_null() {
        return (None, length);
    }
    let names = (0..length as usize)
        .map(|index| {
            let node = unsafe { *result.add(index) };
            unsafe { CStr::from_ptr((*node).city_name.as_ptr()) }
                .to_bytes()
                .to_vec()
        })
        .collect();
    unsafe { free(result.cast()) };
    (Some(names), length)
}

unsafe fn assert_paths_equal(
    c: &Api,
    rust: &Api,
    c_start: *mut Node,
    c_end: *mut Node,
    rust_start: *mut Node,
    rust_end: *mut Node,
) -> Option<Vec<Vec<u8>>> {
    let c_path = unsafe { path_names(c, c_start, c_end, -777) };
    let rust_path = unsafe { path_names(rust, rust_start, rust_end, -777) };
    assert_eq!(c_path, rust_path);
    c_path.0
}

#[test]
fn shortest_path_surface_matches() {
    let _guard = TEST_LOCK.lock().unwrap();
    unsafe {
        let (c, rust) = load_pair();
        let mut rng = Lcg(0x9e37_79b9_7f4a_7c15);

        for _ in 0..32 {
            let length = 1 + (rng.next() % 20) as usize;
            let (c_graph, rust_graph) = create_pair(&c, &rust);
            let (c_nodes, rust_nodes) = add_named_nodes(&c, &rust, c_graph, rust_graph, length);
            for index in 0..length.saturating_sub(1) {
                let distance = (rng.next() % 10_000) as c_int;
                add_edge_pair(
                    &c,
                    &rust,
                    c_nodes[index],
                    c_nodes[index + 1],
                    rust_nodes[index],
                    rust_nodes[index + 1],
                    distance,
                );
            }
            let path = assert_paths_equal(
                &c,
                &rust,
                c_nodes[0],
                c_nodes[length - 1],
                rust_nodes[0],
                rust_nodes[length - 1],
            )
            .unwrap();
            assert_eq!(path.len(), length);
            (c.free_graph)(c_graph);
            (rust.free_graph)(rust_graph);
        }

        for _ in 0..48 {
            let (c_graph, rust_graph) = create_pair(&c, &rust);
            let (c_nodes, rust_nodes) = add_named_nodes(&c, &rust, c_graph, rust_graph, 4);
            let first_total = 2 + (rng.next() % 1000) as c_int;
            let second_total = first_total + 1 + (rng.next() % 1000) as c_int;
            let first_a = (rng.next() % first_total as u64) as c_int;
            let first_b = first_total - first_a;
            let second_a = (rng.next() % second_total as u64) as c_int;
            let second_b = second_total - second_a;
            for &(from, to, distance) in &[
                (0, 1, first_a),
                (1, 3, first_b),
                (0, 2, second_a),
                (2, 3, second_b),
            ] {
                add_edge_pair(
                    &c,
                    &rust,
                    c_nodes[from],
                    c_nodes[to],
                    rust_nodes[from],
                    rust_nodes[to],
                    distance,
                );
            }
            let path = assert_paths_equal(
                &c,
                &rust,
                c_nodes[0],
                c_nodes[3],
                rust_nodes[0],
                rust_nodes[3],
            )
            .unwrap();
            assert_eq!(path[1], b"node-001");
            (c.free_graph)(c_graph);
            (rust.free_graph)(rust_graph);
        }

        let (c_graph, rust_graph) = create_pair(&c, &rust);
        let (c_nodes, rust_nodes) = add_named_nodes(&c, &rust, c_graph, rust_graph, 4);
        for &(from, to, distance) in &[(0, 1, 5), (0, 2, 5), (1, 3, 7), (2, 3, 7)] {
            add_edge_pair(
                &c,
                &rust,
                c_nodes[from],
                c_nodes[to],
                rust_nodes[from],
                rust_nodes[to],
                distance,
            );
        }
        let tied = assert_paths_equal(
            &c,
            &rust,
            c_nodes[0],
            c_nodes[3],
            rust_nodes[0],
            rust_nodes[3],
        )
        .unwrap();
        assert_eq!(tied[1], b"node-001");
        (c.free_graph)(c_graph);
        (rust.free_graph)(rust_graph);

        let (c_graph, rust_graph) = create_pair(&c, &rust);
        let (c_nodes, rust_nodes) = add_named_nodes(&c, &rust, c_graph, rust_graph, 4);
        for &(from, to, distance) in &[(0, 1, 0), (1, 2, 0), (2, 3, 9), (0, 3, 10), (2, 0, 0)] {
            add_edge_pair(
                &c,
                &rust,
                c_nodes[from],
                c_nodes[to],
                rust_nodes[from],
                rust_nodes[to],
                distance,
            );
        }
        let zero_cycle = assert_paths_equal(
            &c,
            &rust,
            c_nodes[0],
            c_nodes[3],
            rust_nodes[0],
            rust_nodes[3],
        )
        .unwrap();
        assert_eq!(zero_cycle.len(), 4);
        (c.free_graph)(c_graph);
        (rust.free_graph)(rust_graph);

        let (c_graph, rust_graph) = create_pair(&c, &rust);
        let (c_nodes, rust_nodes) = add_named_nodes(&c, &rust, c_graph, rust_graph, 2);
        assert_eq!(
            add_edge_pair(
                &c,
                &rust,
                c_nodes[0],
                c_nodes[1],
                rust_nodes[0],
                rust_nodes[1],
                c_int::MAX,
            ),
            0
        );
        assert_graphs_equal(c_graph, rust_graph);
        let c_maximum = path_names(&c, c_nodes[0], c_nodes[1], 88);
        let rust_maximum = path_names(&rust, rust_nodes[0], rust_nodes[1], 88);
        assert_eq!(c_maximum, rust_maximum);
        assert_eq!(c_maximum, (None, 0));
        (c.free_graph)(c_graph);
        (rust.free_graph)(rust_graph);

        let (c_graph, rust_graph) = create_pair(&c, &rust);
        let (c_nodes, rust_nodes) = add_named_nodes(&c, &rust, c_graph, rust_graph, MAX_NODES);
        for index in 0..MAX_NODES - 1 {
            add_edge_pair(
                &c,
                &rust,
                c_nodes[index],
                c_nodes[index + 1],
                rust_nodes[index],
                rust_nodes[index + 1],
                1,
            );
        }
        let boundary = assert_paths_equal(
            &c,
            &rust,
            c_nodes[0],
            c_nodes[MAX_NODES - 1],
            rust_nodes[0],
            rust_nodes[MAX_NODES - 1],
        )
        .unwrap();
        assert_eq!(boundary.len(), MAX_NODES);
        (c.free_graph)(c_graph);
        (rust.free_graph)(rust_graph);

        let mut c_overflow_state = (0..=MAX_NODES).map(|_| blank_node()).collect::<Vec<_>>();
        let mut rust_overflow_state = (0..=MAX_NODES).map(|_| blank_node()).collect::<Vec<_>>();
        for index in 0..MAX_NODES {
            c_overflow_state[index].edges[0].destination = &mut *c_overflow_state[index + 1];
            c_overflow_state[index].edges[0].distance = 1;
            c_overflow_state[index].edge_count = 1;
            rust_overflow_state[index].edges[0].destination = &mut *rust_overflow_state[index + 1];
            rust_overflow_state[index].edges[0].distance = 1;
            rust_overflow_state[index].edge_count = 1;
        }
        let c_capped = path_names(
            &c,
            &mut *c_overflow_state[0],
            &mut *c_overflow_state[MAX_NODES],
            77,
        );
        let rust_capped = path_names(
            &rust,
            &mut *rust_overflow_state[0],
            &mut *rust_overflow_state[MAX_NODES],
            77,
        );
        assert_eq!(c_capped, rust_capped);
        assert_eq!(c_capped, (None, 0));

        let (c_graph, rust_graph) = create_pair(&c, &rust);
        let (c_nodes, rust_nodes) = add_named_nodes(&c, &rust, c_graph, rust_graph, 2);
        let c_missing = path_names(&c, c_nodes[0], c_nodes[1], 99);
        let rust_missing = path_names(&rust, rust_nodes[0], rust_nodes[1], 99);
        assert_eq!(c_missing, rust_missing);
        assert_eq!(c_missing, (None, 0));

        for (c_start, c_end, rust_start, rust_end) in [
            (ptr::null_mut(), c_nodes[1], ptr::null_mut(), rust_nodes[1]),
            (c_nodes[0], ptr::null_mut(), rust_nodes[0], ptr::null_mut()),
            (
                ptr::null_mut(),
                ptr::null_mut(),
                ptr::null_mut(),
                ptr::null_mut(),
            ),
        ] {
            let c_result = path_names(&c, c_start, c_end, 123);
            let rust_result = path_names(&rust, rust_start, rust_end, 123);
            assert_eq!(c_result, rust_result);
            assert_eq!(c_result, (None, 123));
        }
        assert!((c.find_shortest_path)(c_nodes[0], c_nodes[1], ptr::null_mut()).is_null());
        assert!((rust.find_shortest_path)(rust_nodes[0], rust_nodes[1], ptr::null_mut()).is_null());
        (c.free_graph)(c_graph);
        (rust.free_graph)(rust_graph);
    }
}

#[test]
fn allocation_failure_surface_matches() {
    let _guard = TEST_LOCK.lock().unwrap();
    unsafe {
        let c = Api::load(Path::new(env!("DAG_C_FAIL_SO")));
        let rust = Api::load(&rust_library_path());
        let c_set = c.set_fail_alloc.expect("C allocation control");
        let rust_set = rust.set_fail_alloc.expect("Rust allocation control");

        c_set(0);
        rust_set(0);
        assert!((c.create_graph)().is_null());
        assert!((rust.create_graph)().is_null());

        c_set(-1);
        rust_set(-1);
        let (c_graph, rust_graph) = create_pair(&c, &rust);
        let name = CString::new("allocation").unwrap();
        c_set(0);
        rust_set(0);
        assert!((c.add_node)(c_graph, name.as_ptr()).is_null());
        assert!((rust.add_node)(rust_graph, name.as_ptr()).is_null());
        assert_eq!((*c_graph).node_count, 0);
        assert_eq!((*rust_graph).node_count, 0);

        c_set(-1);
        rust_set(-1);
        let (c_nodes, rust_nodes) = add_named_nodes(&c, &rust, c_graph, rust_graph, 2);
        add_edge_pair(
            &c,
            &rust,
            c_nodes[0],
            c_nodes[1],
            rust_nodes[0],
            rust_nodes[1],
            1,
        );
        c_set(0);
        rust_set(0);
        let c_path = path_names(&c, c_nodes[0], c_nodes[1], 91);
        let rust_path = path_names(&rust, rust_nodes[0], rust_nodes[1], 91);
        assert_eq!(c_path, rust_path);
        assert_eq!(c_path, (None, 0));

        c_set(-1);
        rust_set(-1);
        (c.free_graph)(c_graph);
        (rust.free_graph)(rust_graph);
    }
}

unsafe fn capture_stdout(action: impl FnOnce()) -> Vec<u8> {
    unsafe { fflush(ptr::null_mut()) };
    let mut descriptors: [RawFd; 2] = [-1, -1];
    assert_eq!(unsafe { pipe(descriptors.as_mut_ptr()) }, 0);
    let saved_stdout = unsafe { dup(1) };
    assert!(saved_stdout >= 0);
    assert_eq!(unsafe { dup2(descriptors[1], 1) }, 1);
    unsafe { close(descriptors[1]) };

    action();
    unsafe { fflush(ptr::null_mut()) };

    assert_eq!(unsafe { dup2(saved_stdout, 1) }, 1);
    unsafe { close(saved_stdout) };

    let mut output = Vec::new();
    let mut reader = unsafe { File::from_raw_fd(descriptors[0]) };
    reader.read_to_end(&mut output).unwrap();
    output
}

#[test]
fn print_and_free_surface_matches() {
    let _guard = TEST_LOCK.lock().unwrap();
    unsafe {
        let (c, rust) = load_pair();

        let c_null_node = capture_stdout(|| (c.print_node)(ptr::null_mut()));
        let rust_null_node = capture_stdout(|| (rust.print_node)(ptr::null_mut()));
        assert_eq!(c_null_node, rust_null_node);
        assert_eq!(c_null_node, b"NULL node\n");

        let c_null_graph = capture_stdout(|| (c.print_graph)(ptr::null_mut()));
        let rust_null_graph = capture_stdout(|| (rust.print_graph)(ptr::null_mut()));
        assert_eq!(c_null_graph, rust_null_graph);
        assert_eq!(c_null_graph, b"NULL graph\n");

        let (c_graph, rust_graph) = create_pair(&c, &rust);
        let c_empty = capture_stdout(|| (c.print_graph)(c_graph));
        let rust_empty = capture_stdout(|| (rust.print_graph)(rust_graph));
        assert_eq!(c_empty, rust_empty);
        assert_eq!(c_empty, b"Graph with 0 nodes:\n");
        (c.free_graph)(c_graph);
        (rust.free_graph)(rust_graph);

        let mut rng = Lcg(0xd1b5_4a32_d192_ed03);
        for count in 1..=24 {
            let (c_graph, rust_graph) = create_pair(&c, &rust);
            let (c_nodes, rust_nodes) = add_named_nodes(&c, &rust, c_graph, rust_graph, count);
            for index in 0..count.saturating_sub(1).min(MAX_EDGES) {
                add_edge_pair(
                    &c,
                    &rust,
                    c_nodes[0],
                    c_nodes[index + 1],
                    rust_nodes[0],
                    rust_nodes[index + 1],
                    (rng.next() % 100_000) as c_int,
                );
            }
            let c_node_output = capture_stdout(|| (c.print_node)(c_nodes[0]));
            let rust_node_output = capture_stdout(|| (rust.print_node)(rust_nodes[0]));
            assert_eq!(c_node_output, rust_node_output);

            let c_graph_output = capture_stdout(|| (c.print_graph)(c_graph));
            let rust_graph_output = capture_stdout(|| (rust.print_graph)(rust_graph));
            assert_eq!(c_graph_output, rust_graph_output);
            (c.free_graph)(c_graph);
            (rust.free_graph)(rust_graph);
        }

        (c.free_graph)(ptr::null_mut());
        (rust.free_graph)(ptr::null_mut());

        let (c_graph, rust_graph) = create_pair(&c, &rust);
        let name = CString::new("retained").unwrap();
        let (c_node, rust_node) = add_pair(&c, &rust, c_graph, rust_graph, &name);
        (c.shallow_copy)(c_node);
        (rust.shallow_copy)(rust_node);
        assert_eq!((*c_node).ref_count, 2);
        assert_eq!((*rust_node).ref_count, 2);
        (c.free_graph)(c_graph);
        (rust.free_graph)(rust_graph);
        assert_eq!((*c_node).ref_count, 1);
        assert_eq!((*rust_node).ref_count, 1);
        assert_eq!(
            CStr::from_ptr((*c_node).city_name.as_ptr()).to_bytes(),
            CStr::from_ptr((*rust_node).city_name.as_ptr()).to_bytes()
        );
        (c.delete_node)(c_node);
        (rust.delete_node)(rust_node);
    }
}
