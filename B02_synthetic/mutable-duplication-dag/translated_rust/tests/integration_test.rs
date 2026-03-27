use libloading::{Library, Symbol};
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int, c_void};
use std::path::PathBuf;

#[repr(C)]
struct CEdge {
    destination: *mut CNode,
    distance: c_int,
}

#[repr(C)]
struct CNode {
    city_name: [c_char; 64],
    ref_count: c_int,
    edges: [CEdge; 10],
    edge_count: c_int,
}

#[repr(C)]
struct CGraph {
    nodes: [*mut CNode; 100],
    node_count: c_int,
}

fn c_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("c_src")
        .join("build")
        .join("libdag.so")
}

fn c_str(s: &str) -> CString {
    CString::new(s).unwrap()
}

fn read_c_city_name(node: *const CNode) -> String {
    unsafe {
        CStr::from_ptr((*node).city_name.as_ptr())
            .to_string_lossy()
            .into_owned()
    }
}

fn capture_stdout<F: FnOnce()>(f: F) -> String {
    use std::io::{Read, Write};
    use std::os::unix::io::FromRawFd;
    std::io::stdout().flush().unwrap();

    let (read_fd, write_fd) = unsafe {
        let mut fds = [0i32; 2];
        assert_eq!(libc::pipe(fds.as_mut_ptr()), 0);
        (fds[0], fds[1])
    };

    let saved_stdout = unsafe { libc::dup(1) };
    unsafe { libc::dup2(write_fd, 1); }

    f();

    unsafe {
        extern "C" { fn fflush(stream: *mut c_void) -> c_int; }
        fflush(std::ptr::null_mut());
    }
    std::io::stdout().flush().unwrap();

    unsafe {
        libc::dup2(saved_stdout, 1);
        libc::close(saved_stdout);
        libc::close(write_fd);
    }

    let mut result = String::new();
    let mut read_file = unsafe { std::fs::File::from_raw_fd(read_fd) };
    read_file.read_to_string(&mut result).unwrap();
    result
}

#[test]
fn test_create_graph_and_add_nodes() {
    let lib = unsafe { Library::new(c_lib_path()).unwrap() };
    unsafe {
        let c_create: Symbol<unsafe extern "C" fn() -> *mut CGraph> = lib.get(b"create_graph").unwrap();
        let c_add: Symbol<unsafe extern "C" fn(*mut CGraph, *const c_char) -> *mut CNode> = lib.get(b"add_node").unwrap();
        let c_free: Symbol<unsafe extern "C" fn(*mut CGraph)> = lib.get(b"free_graph").unwrap();

        let g = c_create();
        assert!(!g.is_null());
        assert_eq!((*g).node_count, 0);

        let n1 = c_add(g, c_str("Boston").as_ptr());
        assert!(!n1.is_null());
        assert_eq!((*g).node_count, 1);
        assert_eq!(read_c_city_name(n1), "Boston");
        assert_eq!((*n1).ref_count, 1);

        let _n2 = c_add(g, c_str("NYC").as_ptr());
        assert_eq!((*g).node_count, 2);

        let dup = c_add(g, c_str("Boston").as_ptr());
        assert!(dup.is_null());
        assert_eq!((*g).node_count, 2);

        c_free(g);
    }

    // Rust side
    let mut g = dag_city_route_manager::rs_create_graph().unwrap();
    assert_eq!(g.nodes.len(), 0);
    let n1 = dag_city_route_manager::rs_add_node(&mut g, "Boston").unwrap();
    assert_eq!(g.nodes.len(), 1);
    assert_eq!(g.nodes[n1].city_name, "Boston");
    assert_eq!(g.nodes[n1].ref_count, 1);
    dag_city_route_manager::rs_add_node(&mut g, "NYC").unwrap();
    assert_eq!(g.nodes.len(), 2);
    assert!(dag_city_route_manager::rs_add_node(&mut g, "Boston").is_none());
    assert_eq!(g.nodes.len(), 2);
    dag_city_route_manager::rs_free_graph(&mut g);
}

#[test]
fn test_add_edge() {
    let lib = unsafe { Library::new(c_lib_path()).unwrap() };
    unsafe {
        let c_create: Symbol<unsafe extern "C" fn() -> *mut CGraph> = lib.get(b"create_graph").unwrap();
        let c_add: Symbol<unsafe extern "C" fn(*mut CGraph, *const c_char) -> *mut CNode> = lib.get(b"add_node").unwrap();
        let c_edge: Symbol<unsafe extern "C" fn(*mut CNode, *mut CNode, c_int) -> c_int> = lib.get(b"add_edge").unwrap();
        let c_free: Symbol<unsafe extern "C" fn(*mut CGraph)> = lib.get(b"free_graph").unwrap();

        let g = c_create();
        let n1 = c_add(g, c_str("A").as_ptr());
        let n2 = c_add(g, c_str("B").as_ptr());

        assert_eq!(c_edge(n1, n2, 10), 0);
        assert_eq!((*n1).edge_count, 1);
        assert_eq!((*n1).edges[0].distance, 10);
        assert_eq!(c_edge(n1, n2, 20), -1); // duplicate
        assert_eq!(c_edge(n2, n1, -5), -1); // negative

        c_free(g);
    }

    let mut g = dag_city_route_manager::rs_create_graph().unwrap();
    let a = dag_city_route_manager::rs_add_node(&mut g, "A").unwrap();
    let b = dag_city_route_manager::rs_add_node(&mut g, "B").unwrap();
    assert_eq!(dag_city_route_manager::rs_add_edge(&mut g, a, b, 10), 0);
    assert_eq!(g.nodes[a].edges.len(), 1);
    assert_eq!(g.nodes[a].edges[0].distance, 10);
    assert_eq!(dag_city_route_manager::rs_add_edge(&mut g, a, b, 20), -1);
    assert_eq!(dag_city_route_manager::rs_add_edge(&mut g, b, a, -5), -1);
    dag_city_route_manager::rs_free_graph(&mut g);
}

#[test]
fn test_get_node_by_name() {
    let lib = unsafe { Library::new(c_lib_path()).unwrap() };
    unsafe {
        let c_create: Symbol<unsafe extern "C" fn() -> *mut CGraph> = lib.get(b"create_graph").unwrap();
        let c_add: Symbol<unsafe extern "C" fn(*mut CGraph, *const c_char) -> *mut CNode> = lib.get(b"add_node").unwrap();
        let c_get: Symbol<unsafe extern "C" fn(*mut CGraph, *const c_char) -> *mut CNode> = lib.get(b"get_node_by_name").unwrap();
        let c_free: Symbol<unsafe extern "C" fn(*mut CGraph)> = lib.get(b"free_graph").unwrap();

        let g = c_create();
        let n1 = c_add(g, c_str("Boston").as_ptr());
        c_add(g, c_str("NYC").as_ptr());
        assert_eq!(c_get(g, c_str("Boston").as_ptr()), n1);
        assert!(c_get(g, c_str("Chicago").as_ptr()).is_null());
        c_free(g);
    }

    let mut g = dag_city_route_manager::rs_create_graph().unwrap();
    dag_city_route_manager::rs_add_node(&mut g, "Boston").unwrap();
    dag_city_route_manager::rs_add_node(&mut g, "NYC").unwrap();
    assert_eq!(dag_city_route_manager::rs_get_node_by_name(&g, "Boston"), Some(0));
    assert_eq!(dag_city_route_manager::rs_get_node_by_name(&g, "Chicago"), None);
    dag_city_route_manager::rs_free_graph(&mut g);
}

#[test]
fn test_delete_node_ref_count() {
    let lib = unsafe { Library::new(c_lib_path()).unwrap() };
    unsafe {
        let c_create: Symbol<unsafe extern "C" fn() -> *mut CGraph> = lib.get(b"create_graph").unwrap();
        let c_add: Symbol<unsafe extern "C" fn(*mut CGraph, *const c_char) -> *mut CNode> = lib.get(b"add_node").unwrap();
        let c_del: Symbol<unsafe extern "C" fn(*mut CNode)> = lib.get(b"delete_node").unwrap();
        let c_free: Symbol<unsafe extern "C" fn(*mut CGraph)> = lib.get(b"free_graph").unwrap();

        let g = c_create();
        let n1 = c_add(g, c_str("A").as_ptr());
        assert_eq!((*n1).ref_count, 1);
        (*n1).ref_count = 3;
        c_del(n1);
        assert_eq!((*n1).ref_count, 2);
        c_del(n1);
        assert_eq!((*n1).ref_count, 1);
        c_free(g);
    }

    let mut g = dag_city_route_manager::rs_create_graph().unwrap();
    let a = dag_city_route_manager::rs_add_node(&mut g, "A").unwrap();
    g.nodes[a].ref_count = 3;
    dag_city_route_manager::rs_delete_node(&mut g, a);
    assert_eq!(g.nodes[a].ref_count, 2);
    dag_city_route_manager::rs_delete_node(&mut g, a);
    assert_eq!(g.nodes[a].ref_count, 1);
    dag_city_route_manager::rs_free_graph(&mut g);
}

#[test]
fn test_shallow_copy_ref_counts() {
    let lib = unsafe { Library::new(c_lib_path()).unwrap() };
    unsafe {
        let c_create: Symbol<unsafe extern "C" fn() -> *mut CGraph> = lib.get(b"create_graph").unwrap();
        let c_add: Symbol<unsafe extern "C" fn(*mut CGraph, *const c_char) -> *mut CNode> = lib.get(b"add_node").unwrap();
        let c_edge: Symbol<unsafe extern "C" fn(*mut CNode, *mut CNode, c_int) -> c_int> = lib.get(b"add_edge").unwrap();
        let c_copy: Symbol<unsafe extern "C" fn(*mut CNode) -> *mut CNode> = lib.get(b"shallow_copy").unwrap();
        let c_free: Symbol<unsafe extern "C" fn(*mut CGraph)> = lib.get(b"free_graph").unwrap();

        let g = c_create();
        let a = c_add(g, c_str("A").as_ptr());
        let b = c_add(g, c_str("B").as_ptr());
        let c = c_add(g, c_str("C").as_ptr());
        c_edge(a, b, 5);
        c_edge(b, c, 10);

        assert_eq!((*a).ref_count, 1);
        let copy = c_copy(a);
        assert_eq!(copy, a);
        assert_eq!((*a).ref_count, 2);
        assert_eq!((*b).ref_count, 2);
        assert_eq!((*c).ref_count, 2);
        c_free(g);
    }

    let mut g = dag_city_route_manager::rs_create_graph().unwrap();
    let a = dag_city_route_manager::rs_add_node(&mut g, "A").unwrap();
    let b = dag_city_route_manager::rs_add_node(&mut g, "B").unwrap();
    let c = dag_city_route_manager::rs_add_node(&mut g, "C").unwrap();
    dag_city_route_manager::rs_add_edge(&mut g, a, b, 5);
    dag_city_route_manager::rs_add_edge(&mut g, b, c, 10);
    let copy = dag_city_route_manager::rs_shallow_copy(&mut g, a).unwrap();
    assert_eq!(copy, a);
    assert_eq!(g.nodes[a].ref_count, 2);
    assert_eq!(g.nodes[b].ref_count, 2);
    assert_eq!(g.nodes[c].ref_count, 2);
    dag_city_route_manager::rs_free_graph(&mut g);
}

#[test]
fn test_find_shortest_path() {
    let lib = unsafe { Library::new(c_lib_path()).unwrap() };

    let c_path_names: Vec<String>;
    unsafe {
        let c_create: Symbol<unsafe extern "C" fn() -> *mut CGraph> = lib.get(b"create_graph").unwrap();
        let c_add: Symbol<unsafe extern "C" fn(*mut CGraph, *const c_char) -> *mut CNode> = lib.get(b"add_node").unwrap();
        let c_edge: Symbol<unsafe extern "C" fn(*mut CNode, *mut CNode, c_int) -> c_int> = lib.get(b"add_edge").unwrap();
        let c_path: Symbol<unsafe extern "C" fn(*mut CNode, *mut CNode, *mut c_int) -> *mut *mut CNode> = lib.get(b"find_shortest_path").unwrap();
        let c_free: Symbol<unsafe extern "C" fn(*mut CGraph)> = lib.get(b"free_graph").unwrap();

        let g = c_create();
        let a = c_add(g, c_str("A").as_ptr());
        let b = c_add(g, c_str("B").as_ptr());
        let c = c_add(g, c_str("C").as_ptr());
        let d = c_add(g, c_str("D").as_ptr());
        c_edge(a, b, 10);
        c_edge(a, c, 3);
        c_edge(c, b, 2);
        c_edge(b, d, 5);
        c_edge(c, d, 15);

        let mut path_len: c_int = 0;
        let path = c_path(a, d, &mut path_len);
        assert!(!path.is_null());
        assert_eq!(path_len, 4);
        c_path_names = (0..path_len as usize)
            .map(|i| read_c_city_name(*path.add(i)))
            .collect();
        libc::free(path as *mut c_void);
        c_free(g);
    }

    let mut g = dag_city_route_manager::rs_create_graph().unwrap();
    let a = dag_city_route_manager::rs_add_node(&mut g, "A").unwrap();
    let b = dag_city_route_manager::rs_add_node(&mut g, "B").unwrap();
    let c = dag_city_route_manager::rs_add_node(&mut g, "C").unwrap();
    let d = dag_city_route_manager::rs_add_node(&mut g, "D").unwrap();
    dag_city_route_manager::rs_add_edge(&mut g, a, b, 10);
    dag_city_route_manager::rs_add_edge(&mut g, a, c, 3);
    dag_city_route_manager::rs_add_edge(&mut g, c, b, 2);
    dag_city_route_manager::rs_add_edge(&mut g, b, d, 5);
    dag_city_route_manager::rs_add_edge(&mut g, c, d, 15);

    let rust_path = dag_city_route_manager::rs_find_shortest_path(&g, a, d).unwrap();
    let rust_path_names: Vec<String> = rust_path.iter().map(|&idx| g.nodes[idx].city_name.clone()).collect();
    assert_eq!(c_path_names, rust_path_names, "Shortest path mismatch");
    dag_city_route_manager::rs_free_graph(&mut g);
}

#[test]
fn test_find_shortest_path_no_path() {
    let lib = unsafe { Library::new(c_lib_path()).unwrap() };
    unsafe {
        let c_create: Symbol<unsafe extern "C" fn() -> *mut CGraph> = lib.get(b"create_graph").unwrap();
        let c_add: Symbol<unsafe extern "C" fn(*mut CGraph, *const c_char) -> *mut CNode> = lib.get(b"add_node").unwrap();
        let c_path: Symbol<unsafe extern "C" fn(*mut CNode, *mut CNode, *mut c_int) -> *mut *mut CNode> = lib.get(b"find_shortest_path").unwrap();
        let c_free: Symbol<unsafe extern "C" fn(*mut CGraph)> = lib.get(b"free_graph").unwrap();

        let g = c_create();
        let a = c_add(g, c_str("A").as_ptr());
        let b = c_add(g, c_str("B").as_ptr());
        let mut path_len: c_int = 0;
        let path = c_path(a, b, &mut path_len);
        assert!(path.is_null());
        assert_eq!(path_len, 0);
        c_free(g);
    }

    let mut g = dag_city_route_manager::rs_create_graph().unwrap();
    let a = dag_city_route_manager::rs_add_node(&mut g, "A").unwrap();
    let b = dag_city_route_manager::rs_add_node(&mut g, "B").unwrap();
    assert!(dag_city_route_manager::rs_find_shortest_path(&g, a, b).is_none());
    dag_city_route_manager::rs_free_graph(&mut g);
}

#[test]
fn test_print_node_output() {
    let lib = unsafe { Library::new(c_lib_path()).unwrap() };
    let c_output: String;
    unsafe {
        let c_create: Symbol<unsafe extern "C" fn() -> *mut CGraph> = lib.get(b"create_graph").unwrap();
        let c_add: Symbol<unsafe extern "C" fn(*mut CGraph, *const c_char) -> *mut CNode> = lib.get(b"add_node").unwrap();
        let c_edge: Symbol<unsafe extern "C" fn(*mut CNode, *mut CNode, c_int) -> c_int> = lib.get(b"add_edge").unwrap();
        let c_print: Symbol<unsafe extern "C" fn(*mut CNode)> = lib.get(b"print_node").unwrap();
        let c_free: Symbol<unsafe extern "C" fn(*mut CGraph)> = lib.get(b"free_graph").unwrap();

        let g = c_create();
        let a = c_add(g, c_str("Boston").as_ptr());
        let b = c_add(g, c_str("NYC").as_ptr());
        c_edge(a, b, 200);
        c_output = capture_stdout(|| { c_print(a); });
        c_free(g);
    }

    let mut g = dag_city_route_manager::rs_create_graph().unwrap();
    let a = dag_city_route_manager::rs_add_node(&mut g, "Boston").unwrap();
    let b = dag_city_route_manager::rs_add_node(&mut g, "NYC").unwrap();
    dag_city_route_manager::rs_add_edge(&mut g, a, b, 200);
    let mut rust_output = Vec::new();
    dag_city_route_manager::rs_print_node_to(&g, a, &mut rust_output);
    let rust_output = String::from_utf8(rust_output).unwrap();
    assert_eq!(c_output, rust_output, "print_node output mismatch");
    dag_city_route_manager::rs_free_graph(&mut g);
}

#[test]
fn test_print_graph_output() {
    let lib = unsafe { Library::new(c_lib_path()).unwrap() };
    let c_output: String;
    unsafe {
        let c_create: Symbol<unsafe extern "C" fn() -> *mut CGraph> = lib.get(b"create_graph").unwrap();
        let c_add: Symbol<unsafe extern "C" fn(*mut CGraph, *const c_char) -> *mut CNode> = lib.get(b"add_node").unwrap();
        let c_edge: Symbol<unsafe extern "C" fn(*mut CNode, *mut CNode, c_int) -> c_int> = lib.get(b"add_edge").unwrap();
        let c_print: Symbol<unsafe extern "C" fn(*mut CGraph)> = lib.get(b"print_graph").unwrap();
        let c_free: Symbol<unsafe extern "C" fn(*mut CGraph)> = lib.get(b"free_graph").unwrap();

        let g = c_create();
        let a = c_add(g, c_str("Boston").as_ptr());
        let b = c_add(g, c_str("NYC").as_ptr());
        let c = c_add(g, c_str("Chicago").as_ptr());
        c_edge(a, b, 200);
        c_edge(b, c, 150);
        c_edge(a, c, 300);
        c_output = capture_stdout(|| { c_print(g); });
        c_free(g);
    }

    let mut g = dag_city_route_manager::rs_create_graph().unwrap();
    let a = dag_city_route_manager::rs_add_node(&mut g, "Boston").unwrap();
    let b = dag_city_route_manager::rs_add_node(&mut g, "NYC").unwrap();
    let c = dag_city_route_manager::rs_add_node(&mut g, "Chicago").unwrap();
    dag_city_route_manager::rs_add_edge(&mut g, a, b, 200);
    dag_city_route_manager::rs_add_edge(&mut g, b, c, 150);
    dag_city_route_manager::rs_add_edge(&mut g, a, c, 300);
    let mut rust_output = Vec::new();
    dag_city_route_manager::rs_print_graph_to(&g, &mut rust_output);
    let rust_output = String::from_utf8(rust_output).unwrap();
    assert_eq!(c_output, rust_output, "print_graph output mismatch");
    dag_city_route_manager::rs_free_graph(&mut g);
}

#[test]
fn test_free_graph_decrements_ref_counts() {
    let mut g = dag_city_route_manager::rs_create_graph().unwrap();
    dag_city_route_manager::rs_add_node(&mut g, "A").unwrap();
    dag_city_route_manager::rs_add_node(&mut g, "B").unwrap();
    assert_eq!(g.nodes[0].ref_count, 1);
    assert_eq!(g.nodes[1].ref_count, 1);
    dag_city_route_manager::rs_free_graph(&mut g);
    assert_eq!(g.nodes[0].ref_count, 0);
    assert_eq!(g.nodes[1].ref_count, 0);
}

#[test]
fn test_binary_output_comparison() {
    use std::process::Command;
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let c_binary = manifest_dir.join("c_src").join("build").join("driver");
    let rust_binary = manifest_dir.join("target").join("debug").join("driver");

    Command::new("cargo")
        .args(["build", "--bin", "driver"])
        .current_dir(&manifest_dir)
        .status()
        .expect("Failed to build Rust binary");

    let test_input = "1\nBoston\n1\nNYC\n1\nChicago\n2\nBoston\nNYC\n200\n2\nNYC\nChicago\n150\n3\n5\nBoston\nChicago\n8\n";

    let c_out = Command::new(&c_binary)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            use std::io::Write;
            child.stdin.as_mut().unwrap().write_all(test_input.as_bytes()).unwrap();
            child.wait_with_output()
        })
        .expect("Failed to run C binary");

    let rust_out = Command::new(&rust_binary)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            use std::io::Write;
            child.stdin.as_mut().unwrap().write_all(test_input.as_bytes()).unwrap();
            child.wait_with_output()
        })
        .expect("Failed to run Rust binary");

    let c_stdout = String::from_utf8_lossy(&c_out.stdout);
    let rust_stdout = String::from_utf8_lossy(&rust_out.stdout);
    assert_eq!(c_stdout, rust_stdout, "Binary stdout mismatch!\n--- C ---\n{}\n--- Rust ---\n{}", c_stdout, rust_stdout);
}
