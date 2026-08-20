//! Shared harness for the differential tests.
//!
//! Both implementations are loaded as shared objects with `libloading` and
//! driven through their exported C symbols only:
//!
//! * `build_c/libdag_c.so`        — built from the unmodified `c_src/src/lib.c`
//! * `target/debug/libdag_rs.so`  — the Rust `cdylib`
//!
//! Everything observable is recorded into a text log (return values with
//! pointers canonicalised to creation indices, plus the full `node_t` /
//! `graph_t` memory contents) and the captured `stdout` / `stderr` bytes, so
//! the two runs can be compared byte for byte.

#![allow(dead_code)]

use libloading::Library;
use std::io::Read;
use std::os::raw::{c_char, c_int, c_void};
use std::os::unix::io::AsRawFd;
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, OnceLock};

pub const MAX_CITY_NAME: usize = 64;
pub const MAX_EDGES: usize = 10;
pub const MAX_NODES: usize = 100;

// ---------------------------------------------------------------------------
// C data layout (dag_lib.h)
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Clone, Copy)]
pub struct EdgeT {
    pub destination: *mut NodeT,
    pub distance: c_int,
}

#[repr(C)]
pub struct NodeT {
    pub city_name: [c_char; MAX_CITY_NAME],
    pub ref_count: c_int,
    pub edges: [EdgeT; MAX_EDGES],
    pub edge_count: c_int,
}

#[repr(C)]
pub struct GraphT {
    pub nodes: [*mut NodeT; MAX_NODES],
    pub node_count: c_int,
}

extern "C" {
    fn free(ptr: *mut c_void);
    fn fflush(stream: *mut c_void) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(old: c_int, new: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
}

// ---------------------------------------------------------------------------
// Loading the two shared objects
// ---------------------------------------------------------------------------

type FnCreateGraph = unsafe extern "C" fn() -> *mut GraphT;
type FnAddNode = unsafe extern "C" fn(*mut GraphT, *const c_char) -> *mut NodeT;
type FnAddEdge = unsafe extern "C" fn(*mut NodeT, *mut NodeT, c_int) -> c_int;
type FnDeleteNode = unsafe extern "C" fn(*mut NodeT);
type FnShallowCopy = unsafe extern "C" fn(*mut NodeT) -> *mut NodeT;
type FnFindShortestPath =
    unsafe extern "C" fn(*mut NodeT, *mut NodeT, *mut c_int) -> *mut *mut NodeT;
type FnFreeGraph = unsafe extern "C" fn(*mut GraphT);
type FnGetNodeByName = unsafe extern "C" fn(*mut GraphT, *const c_char) -> *mut NodeT;
type FnPrintNode = unsafe extern "C" fn(*mut NodeT);
type FnPrintGraph = unsafe extern "C" fn(*mut GraphT);

pub struct Api {
    pub name: &'static str,
    pub create_graph: FnCreateGraph,
    pub add_node: FnAddNode,
    pub add_edge: FnAddEdge,
    pub delete_node: FnDeleteNode,
    pub shallow_copy: FnShallowCopy,
    pub find_shortest_path: FnFindShortestPath,
    pub free_graph: FnFreeGraph,
    pub get_node_by_name: FnGetNodeByName,
    pub print_node: FnPrintNode,
    pub print_graph: FnPrintGraph,
}

unsafe impl Send for Api {}
unsafe impl Sync for Api {}

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

pub fn c_so_path() -> PathBuf {
    crate_root().join("build_c/libdag_c.so")
}

pub fn rust_so_path() -> PathBuf {
    // `cargo test` puts the test binary in <target>/debug/deps/…, so derive the
    // artifact directory from the current executable instead of hardcoding it.
    let exe = std::env::current_exe().expect("current_exe");
    let deps = exe.parent().expect("deps dir");
    let debug = deps.parent().expect("target/debug");
    let direct = debug.join("libdag_rs.so");
    if direct.exists() {
        return direct;
    }
    crate_root().join("target/debug/libdag_rs.so")
}

/// Builds `build_c/libdag_c.so` from the unmodified C sources if it is missing.
fn ensure_c_so() {
    let so = c_so_path();
    if so.exists() {
        return;
    }
    let root = crate_root();
    std::fs::create_dir_all(root.join("build_c")).ok();
    let status = std::process::Command::new("gcc")
        .current_dir(&root)
        .args([
            "-shared",
            "-fPIC",
            "-O2",
            "-I",
            "c_src/include",
            "-o",
            "build_c/libdag_c.so",
            "c_src/src/lib.c",
        ])
        .status()
        .expect("failed to run gcc");
    assert!(status.success(), "gcc failed to build build_c/libdag_c.so");
}

/// `cargo test` builds the test binaries and the `rlib`, but it does **not**
/// refresh the `cdylib` artifact, so a plain `cargo test` would happily load a
/// stale `libdag_rs.so`. Rebuild it explicitly before dlopen'ing it.
fn ensure_rust_so() {
    let built = std::process::Command::new(std::env::var("CARGO").unwrap_or("cargo".into()))
        .current_dir(crate_root())
        .args(["build", "--offline", "--quiet", "--lib"])
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    let path = rust_so_path();
    assert!(
        built || path.exists(),
        "could not build {} - run `cargo build --offline` first",
        path.display()
    );
}

fn load(path: &PathBuf, name: &'static str) -> Api {
    assert!(
        path.exists(),
        "{} does not exist - run `cargo build --offline` first",
        path.display()
    );
    // Leaked on purpose: the library must stay mapped for the whole test run.
    let lib: &'static Library =
        Box::leak(Box::new(unsafe { Library::new(path) }.unwrap_or_else(|e| {
            panic!("dlopen({}) failed: {e}", path.display());
        })));
    unsafe {
        macro_rules! sym {
            ($t:ty, $n:literal) => {{
                let s = lib
                    .get::<$t>(concat!($n, "\0").as_bytes())
                    .unwrap_or_else(|e| panic!("{} misses symbol {}: {e}", name, $n));
                *s
            }};
        }
        Api {
            name,
            create_graph: sym!(FnCreateGraph, "create_graph"),
            add_node: sym!(FnAddNode, "add_node"),
            add_edge: sym!(FnAddEdge, "add_edge"),
            delete_node: sym!(FnDeleteNode, "delete_node"),
            shallow_copy: sym!(FnShallowCopy, "shallow_copy"),
            find_shortest_path: sym!(FnFindShortestPath, "find_shortest_path"),
            free_graph: sym!(FnFreeGraph, "free_graph"),
            get_node_by_name: sym!(FnGetNodeByName, "get_node_by_name"),
            print_node: sym!(FnPrintNode, "print_node"),
            print_graph: sym!(FnPrintGraph, "print_graph"),
        }
    }
}

pub fn c_api() -> &'static Api {
    static API: OnceLock<Api> = OnceLock::new();
    API.get_or_init(|| {
        ensure_c_so();
        load(&c_so_path(), "C")
    })
}

pub fn rust_api() -> &'static Api {
    static API: OnceLock<Api> = OnceLock::new();
    API.get_or_init(|| {
        ensure_rust_so();
        load(&rust_so_path(), "Rust")
    })
}

/// Serialises everything that redirects the process' file descriptors.
pub fn serial() -> MutexGuard<'static, ()> {
    static LOCK: Mutex<()> = Mutex::new(());
    match LOCK.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    }
}

// ---------------------------------------------------------------------------
// stdout / stderr capture
// ---------------------------------------------------------------------------

pub struct Captured {
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

fn read_all(path: &PathBuf) -> Vec<u8> {
    let mut buf = Vec::new();
    if let Ok(mut f) = std::fs::File::open(path) {
        let _ = f.read_to_end(&mut buf);
    }
    buf
}

/// Runs `f` with file descriptors 1 and 2 redirected into temporary files and
/// returns whatever the callee wrote (after `fflush(NULL)`, so glibc's fully
/// buffered stdout is included).
pub fn capture<T>(f: impl FnOnce() -> T) -> (T, Captured) {
    static SEQ: Mutex<u64> = Mutex::new(0);
    let n = {
        let mut g = SEQ.lock().unwrap_or_else(|p| p.into_inner());
        *g += 1;
        *g
    };
    let dir = std::env::temp_dir();
    let out_path = dir.join(format!("dagdiff-{}-{}.out", std::process::id(), n));
    let err_path = dir.join(format!("dagdiff-{}-{}.err", std::process::id(), n));

    let out_file = std::fs::File::create(&out_path).expect("create stdout capture");
    let err_file = std::fs::File::create(&err_path).expect("create stderr capture");

    let result;
    unsafe {
        // Push out anything that is still pending for the real streams.
        fflush(std::ptr::null_mut());
        let saved_out = dup(1);
        let saved_err = dup(2);
        assert!(saved_out >= 0 && saved_err >= 0, "dup failed");
        dup2(out_file.as_raw_fd(), 1);
        dup2(err_file.as_raw_fd(), 2);

        result = f();

        fflush(std::ptr::null_mut());
        dup2(saved_out, 1);
        dup2(saved_err, 2);
        close(saved_out);
        close(saved_err);
    }
    drop(out_file);
    drop(err_file);

    let captured = Captured {
        stdout: read_all(&out_path),
        stderr: read_all(&err_path),
    };
    let _ = std::fs::remove_file(&out_path);
    let _ = std::fs::remove_file(&err_path);
    (result, captured)
}

// ---------------------------------------------------------------------------
// Operation script
// ---------------------------------------------------------------------------

/// One call into the library. Node/graph operands are creation indices, which
/// the runner resolves to the pointers the library handed out.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Op {
    CreateGraph,
    /// `add_node(graph[g], name)`
    AddNode(usize, Vec<u8>),
    /// `add_node(NULL, name)`
    AddNodeNullGraph(Vec<u8>),
    /// `add_node(graph[g], NULL)`
    AddNodeNullName(usize),
    /// `add_node(NULL, NULL)`
    AddNodeNullBoth,
    /// `add_edge(node[from], node[to], distance)`
    AddEdge(usize, usize, i32),
    /// `add_edge(NULL, node[to], distance)`
    AddEdgeNullFrom(usize, i32),
    /// `add_edge(node[from], NULL, distance)`
    AddEdgeNullTo(usize, i32),
    /// `add_edge(NULL, NULL, distance)`
    AddEdgeNullBoth(i32),
    DeleteNode(usize),
    DeleteNodeNull,
    ShallowCopy(usize),
    ShallowCopyNull,
    FindShortestPath(usize, usize),
    FindShortestPathNullStart(usize),
    FindShortestPathNullEnd(usize),
    /// `find_shortest_path(node[a], node[b], NULL)`
    FindShortestPathNullLen(usize, usize),
    GetNodeByName(usize, Vec<u8>),
    GetNodeByNameNullGraph(Vec<u8>),
    GetNodeByNameNullName(usize),
    PrintNode(usize),
    PrintNodeNull,
    PrintGraph(usize),
    PrintGraphNull,
    FreeGraph(usize),
    FreeGraphNull,
    /// Dump every graph and every live node (memory comparison).
    DumpAll,
    /// Dump one node.
    DumpNode(usize),
}

pub struct Runner<'a> {
    api: &'a Api,
    graphs: Vec<*mut GraphT>,
    nodes: Vec<*mut NodeT>,
    log: Vec<String>,
}

fn quote(bytes: &[u8]) -> String {
    let mut s = String::from("\"");
    for &b in bytes {
        match b {
            b'"' => s.push_str("\\\""),
            b'\\' => s.push_str("\\\\"),
            0x20..=0x7e => s.push(b as char),
            _ => s.push_str(&format!("\\x{:02x}", b)),
        }
    }
    s.push('"');
    s
}

impl<'a> Runner<'a> {
    pub fn new(api: &'a Api) -> Runner<'a> {
        Runner {
            api,
            graphs: Vec::new(),
            nodes: Vec::new(),
            log: Vec::new(),
        }
    }

    /// Canonical name of a node pointer: the *most recent* creation index with
    /// that address (an address can be recycled by `malloc` after a `free`).
    fn canon(&self, p: *const NodeT) -> String {
        if p.is_null() {
            return "NULL".to_string();
        }
        for (i, &q) in self.nodes.iter().enumerate().rev() {
            if q as *const NodeT == p {
                return format!("n{i}");
            }
        }
        "n?".to_string()
    }

    fn node_ptr(&self, id: usize) -> *mut NodeT {
        self.nodes[id]
    }

    fn graph_ptr(&self, id: usize) -> *mut GraphT {
        self.graphs[id]
    }

    unsafe fn dump_node(&self, p: *mut NodeT) -> String {
        if p.is_null() {
            return "NULL".to_string();
        }
        let n = &*p;
        let name: Vec<u8> = n.city_name.iter().map(|&c| c as u8).collect();
        let mut s = format!(
            "{{name_bytes={} ref_count={} edge_count={} edges=[",
            quote(&name),
            n.ref_count,
            n.edge_count
        );
        // Only the first `edge_count` entries were ever written; the rest is
        // uninitialised `malloc` memory in the C.
        let live = n.edge_count.clamp(0, MAX_EDGES as c_int) as usize;
        for i in 0..live {
            if i > 0 {
                s.push(' ');
            }
            s.push_str(&format!(
                "{}:{}",
                self.canon(n.edges[i].destination),
                n.edges[i].distance
            ));
        }
        s.push_str("]}");
        s
    }

    unsafe fn dump_graph(&self, p: *mut GraphT) -> String {
        if p.is_null() {
            return "NULL".to_string();
        }
        let g = &*p;
        let mut s = format!("{{node_count={} nodes=[", g.node_count);
        let live = g.node_count.clamp(0, MAX_NODES as c_int) as usize;
        for i in 0..live {
            if i > 0 {
                s.push(' ');
            }
            s.push_str(&self.canon(g.nodes[i]));
        }
        s.push_str("] tail=[");
        for i in live..MAX_NODES {
            if i > live {
                s.push(' ');
            }
            s.push_str(if g.nodes[i].is_null() { "NULL" } else { "set" });
        }
        s.push_str("]}");
        s
    }

    pub fn run(&mut self, ops: &[Op]) {
        for op in ops {
            unsafe { self.run_one(op) };
        }
    }

    unsafe fn run_one(&mut self, op: &Op) {
        match op {
            Op::CreateGraph => {
                let g = (self.api.create_graph)();
                let id = self.graphs.len();
                self.graphs.push(g);
                self.log.push(format!(
                    "create_graph -> g{id} null={} {}",
                    g.is_null(),
                    self.dump_graph(g)
                ));
            }
            Op::AddNode(g, name) => {
                let cname = cstring(name);
                let p = (self.api.add_node)(self.graph_ptr(*g), cname.as_ptr() as *const c_char);
                if !p.is_null() {
                    self.nodes.push(p);
                }
                self.log.push(format!(
                    "add_node(g{g}, {}) -> {} {}",
                    quote(name),
                    self.canon(p),
                    self.dump_node(p)
                ));
            }
            Op::AddNodeNullGraph(name) => {
                let cname = cstring(name);
                let p = (self.api.add_node)(
                    std::ptr::null_mut(),
                    cname.as_ptr() as *const c_char,
                );
                self.log
                    .push(format!("add_node(NULL, {}) -> {}", quote(name), self.canon(p)));
            }
            Op::AddNodeNullName(g) => {
                let p = (self.api.add_node)(self.graph_ptr(*g), std::ptr::null());
                self.log
                    .push(format!("add_node(g{g}, NULL) -> {}", self.canon(p)));
            }
            Op::AddNodeNullBoth => {
                let p = (self.api.add_node)(std::ptr::null_mut(), std::ptr::null());
                self.log
                    .push(format!("add_node(NULL, NULL) -> {}", self.canon(p)));
            }
            Op::AddEdge(from, to, d) => {
                let r = (self.api.add_edge)(self.node_ptr(*from), self.node_ptr(*to), *d);
                self.log.push(format!(
                    "add_edge(n{from}, n{to}, {d}) -> {r} from={}",
                    self.dump_node(self.node_ptr(*from))
                ));
            }
            Op::AddEdgeNullFrom(to, d) => {
                let r = (self.api.add_edge)(std::ptr::null_mut(), self.node_ptr(*to), *d);
                self.log.push(format!("add_edge(NULL, n{to}, {d}) -> {r}"));
            }
            Op::AddEdgeNullTo(from, d) => {
                let r = (self.api.add_edge)(self.node_ptr(*from), std::ptr::null_mut(), *d);
                self.log.push(format!(
                    "add_edge(n{from}, NULL, {d}) -> {r} from={}",
                    self.dump_node(self.node_ptr(*from))
                ));
            }
            Op::AddEdgeNullBoth(d) => {
                let r = (self.api.add_edge)(std::ptr::null_mut(), std::ptr::null_mut(), *d);
                self.log.push(format!("add_edge(NULL, NULL, {d}) -> {r}"));
            }
            Op::DeleteNode(id) => {
                (self.api.delete_node)(self.node_ptr(*id));
                self.log.push(format!("delete_node(n{id})"));
            }
            Op::DeleteNodeNull => {
                (self.api.delete_node)(std::ptr::null_mut());
                self.log.push("delete_node(NULL)".to_string());
            }
            Op::ShallowCopy(id) => {
                let p = (self.api.shallow_copy)(self.node_ptr(*id));
                self.log.push(format!(
                    "shallow_copy(n{id}) -> {} {}",
                    self.canon(p),
                    self.dump_node(p)
                ));
            }
            Op::ShallowCopyNull => {
                let p = (self.api.shallow_copy)(std::ptr::null_mut());
                self.log
                    .push(format!("shallow_copy(NULL) -> {}", self.canon(p)));
            }
            Op::FindShortestPath(a, b) => {
                let mut len: c_int = -12345;
                let res = (self.api.find_shortest_path)(
                    self.node_ptr(*a),
                    self.node_ptr(*b),
                    &mut len,
                );
                let mut entries = String::new();
                if !res.is_null() {
                    for i in 0..len.max(0) as usize {
                        if i > 0 {
                            entries.push(' ');
                        }
                        entries.push_str(&self.canon(*res.add(i)));
                    }
                }
                self.log.push(format!(
                    "find_shortest_path(n{a}, n{b}) -> null={} len={len} path=[{entries}]",
                    res.is_null()
                ));
                if !res.is_null() {
                    free(res as *mut c_void);
                }
            }
            Op::FindShortestPathNullStart(b) => {
                let mut len: c_int = -12345;
                let res = (self.api.find_shortest_path)(
                    std::ptr::null_mut(),
                    self.node_ptr(*b),
                    &mut len,
                );
                self.log.push(format!(
                    "find_shortest_path(NULL, n{b}) -> null={} len={len}",
                    res.is_null()
                ));
                if !res.is_null() {
                    free(res as *mut c_void);
                }
            }
            Op::FindShortestPathNullEnd(a) => {
                let mut len: c_int = -12345;
                let res = (self.api.find_shortest_path)(
                    self.node_ptr(*a),
                    std::ptr::null_mut(),
                    &mut len,
                );
                self.log.push(format!(
                    "find_shortest_path(n{a}, NULL) -> null={} len={len}",
                    res.is_null()
                ));
                if !res.is_null() {
                    free(res as *mut c_void);
                }
            }
            Op::FindShortestPathNullLen(a, b) => {
                let res = (self.api.find_shortest_path)(
                    self.node_ptr(*a),
                    self.node_ptr(*b),
                    std::ptr::null_mut(),
                );
                self.log.push(format!(
                    "find_shortest_path(n{a}, n{b}, NULL) -> null={}",
                    res.is_null()
                ));
                if !res.is_null() {
                    free(res as *mut c_void);
                }
            }
            Op::GetNodeByName(g, name) => {
                let cname = cstring(name);
                let p = (self.api.get_node_by_name)(
                    self.graph_ptr(*g),
                    cname.as_ptr() as *const c_char,
                );
                self.log.push(format!(
                    "get_node_by_name(g{g}, {}) -> {}",
                    quote(name),
                    self.canon(p)
                ));
            }
            Op::GetNodeByNameNullGraph(name) => {
                let cname = cstring(name);
                let p = (self.api.get_node_by_name)(
                    std::ptr::null_mut(),
                    cname.as_ptr() as *const c_char,
                );
                self.log.push(format!(
                    "get_node_by_name(NULL, {}) -> {}",
                    quote(name),
                    self.canon(p)
                ));
            }
            Op::GetNodeByNameNullName(g) => {
                let p = (self.api.get_node_by_name)(self.graph_ptr(*g), std::ptr::null());
                self.log
                    .push(format!("get_node_by_name(g{g}, NULL) -> {}", self.canon(p)));
            }
            Op::PrintNode(id) => {
                (self.api.print_node)(self.node_ptr(*id));
                self.log.push(format!("print_node(n{id})"));
            }
            Op::PrintNodeNull => {
                (self.api.print_node)(std::ptr::null_mut());
                self.log.push("print_node(NULL)".to_string());
            }
            Op::PrintGraph(g) => {
                (self.api.print_graph)(self.graph_ptr(*g));
                self.log.push(format!("print_graph(g{g})"));
            }
            Op::PrintGraphNull => {
                (self.api.print_graph)(std::ptr::null_mut());
                self.log.push("print_graph(NULL)".to_string());
            }
            Op::FreeGraph(g) => {
                (self.api.free_graph)(self.graph_ptr(*g));
                self.log.push(format!("free_graph(g{g})"));
            }
            Op::FreeGraphNull => {
                (self.api.free_graph)(std::ptr::null_mut());
                self.log.push("free_graph(NULL)".to_string());
            }
            Op::DumpAll => {
                let graphs: Vec<*mut GraphT> = self.graphs.clone();
                for (i, g) in graphs.iter().enumerate() {
                    let d = self.dump_graph(*g);
                    self.log.push(format!("  g{i} = {d}"));
                }
                let nodes: Vec<*mut NodeT> = self.nodes.clone();
                for (i, n) in nodes.iter().enumerate() {
                    let d = self.dump_node(*n);
                    self.log.push(format!("  n{i} = {d}"));
                }
            }
            Op::DumpNode(id) => {
                let d = self.dump_node(self.node_ptr(*id));
                self.log.push(format!("  n{id} = {d}"));
            }
        }
    }

    pub fn into_log(self) -> Vec<String> {
        self.log
    }
}

fn cstring(bytes: &[u8]) -> Vec<u8> {
    let mut v = bytes.to_vec();
    v.push(0);
    v
}

// ---------------------------------------------------------------------------
// Differential driver
// ---------------------------------------------------------------------------

fn run_with(api: &Api, ops: &[Op]) -> (Vec<String>, Captured) {
    let (log, captured) = capture(|| {
        let mut r = Runner::new(api);
        r.run(ops);
        r.into_log()
    });
    (log, captured)
}

fn show(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

/// Runs `ops` against both shared objects and asserts that the logs, stdout and
/// stderr match byte for byte.
pub fn assert_same(label: &str, ops: &[Op]) {
    let _guard = serial();
    let (c_log, c_out) = run_with(c_api(), ops);
    let (r_log, r_out) = run_with(rust_api(), ops);

    // `DAG_DIFF_DUMP=<substring>` prints what the C actually produced, so that a
    // row of ERRORS.md / CONFIGS.md can be checked for really triggering the
    // branch it claims to trigger.
    if let Ok(want) = std::env::var("DAG_DIFF_DUMP") {
        if label.contains(&want) {
            eprintln!("### {label}\n--- log ---");
            for l in &c_log {
                eprintln!("{l}");
            }
            eprintln!("--- stdout ---\n{}", show(&c_out.stdout));
            eprintln!("--- stderr ---\n{}", show(&c_out.stderr));
        }
    }

    if c_log != r_log {
        let mut msg = format!("[{label}] call/memory log differs\n");
        let n = c_log.len().max(r_log.len());
        for i in 0..n {
            let a = c_log.get(i).map(String::as_str).unwrap_or("<missing>");
            let b = r_log.get(i).map(String::as_str).unwrap_or("<missing>");
            if a != b {
                msg.push_str(&format!("  op {i}:\n    C   : {a}\n    Rust: {b}\n"));
                if msg.len() > 4000 {
                    msg.push_str("  ...\n");
                    break;
                }
            }
        }
        msg.push_str(&format!("  ops: {:?}\n", &ops[..ops.len().min(40)]));
        panic!("{msg}");
    }
    assert_eq!(
        show(&c_out.stdout),
        show(&r_out.stdout),
        "[{label}] stdout differs"
    );
    assert_eq!(
        c_out.stdout, r_out.stdout,
        "[{label}] stdout differs (bytes)"
    );
    assert_eq!(
        show(&c_out.stderr),
        show(&r_out.stderr),
        "[{label}] stderr differs"
    );
    assert_eq!(
        c_out.stderr, r_out.stderr,
        "[{label}] stderr differs (bytes)"
    );
}

// ---------------------------------------------------------------------------
// Minimal serial test harness
// ---------------------------------------------------------------------------

/// Runs the given cases one after another in a single thread.
///
/// The differential tests redirect the process' file descriptors 1 and 2, which
/// is only safe while nothing else writes to them - including libtest's own
/// progress output. The affected test targets therefore use `harness = false`
/// and this runner.
pub fn run_suite(cases: &[(&str, fn())]) {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let filters: Vec<String> = args
        .iter()
        .filter(|a| !a.starts_with('-'))
        .cloned()
        .collect();
    let list = args.iter().any(|a| a == "--list");

    if list {
        for (name, _) in cases {
            println!("{name}: test");
        }
        println!("\n{} tests, 0 benchmarks", cases.len());
        return;
    }

    let selected: Vec<&(&str, fn())> = cases
        .iter()
        .filter(|(name, _)| filters.is_empty() || filters.iter().any(|f| name.contains(f)))
        .collect();

    println!("\nrunning {} tests (serial harness)", selected.len());
    let mut failed: Vec<String> = Vec::new();
    for (name, f) in &selected {
        print!("test {name} ... ");
        let _ = std::io::Write::flush(&mut std::io::stdout());
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(*f));
        match outcome {
            Ok(()) => println!("ok"),
            Err(_) => {
                println!("FAILED");
                failed.push((*name).to_string());
            }
        }
    }
    println!();
    if failed.is_empty() {
        println!(
            "test result: ok. {} passed; 0 failed; {} filtered out",
            selected.len(),
            cases.len() - selected.len()
        );
    } else {
        println!(
            "test result: FAILED. {} passed; {} failed",
            selected.len() - failed.len(),
            failed.len()
        );
        for name in &failed {
            println!("  failed: {name}");
        }
        std::process::exit(1);
    }
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (xorshift64*)
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed | 1)
    }

    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    /// Uniform in `0..n` (`n > 0`).
    pub fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
    }

    pub fn range_i32(&mut self, lo: i32, hi: i32) -> i32 {
        let span = (hi as i64 - lo as i64 + 1) as u64;
        (lo as i64 + (self.next_u64() % span) as i64) as i32
    }

    pub fn bool(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }

    /// A random city name: mostly printable, sometimes with high bytes, never
    /// containing a NUL (which C strings cannot carry).
    pub fn name(&mut self, max_len: usize) -> Vec<u8> {
        let len = self.below(max_len + 1);
        let mut v = Vec::with_capacity(len);
        for _ in 0..len {
            let b = match self.below(10) {
                0 => self.range_i32(0x80, 0xff) as u8,
                1 => b' ',
                2 => b'-',
                3..=5 => b'a' + self.below(26) as u8,
                6..=7 => b'A' + self.below(26) as u8,
                _ => b'0' + self.below(10) as u8,
            };
            v.push(if b == 0 { b'x' } else { b });
        }
        v
    }
}
