//! Shared harness: loads the C reference `.so` and every Rust `.so` build
//! artifact via `libloading`, so all calls cross a real FFI boundary and
//! exercise the `#[no_mangle]` export wrappers.
#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::{c_char, c_double, c_int};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, OnceLock};

pub const MAX_NODES: usize = 100;
pub const MAX_NAME_LEN: usize = 50;

/// Mirror of the C `Node` struct, used only to read back memory handed out by
/// `find_node_by_id` from either library.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Node {
    pub id: c_int,
    pub parent_id: c_int,
    pub name: [c_char; MAX_NAME_LEN],
    pub value: c_double,
    pub active: c_int,
}

/// Comparable snapshot of a `Node` (raw bit pattern for the double so that
/// NaN payloads and -0.0 are compared byte-for-byte).
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct NodeSnapshot {
    pub id: c_int,
    pub parent_id: c_int,
    pub name: Vec<u8>,
    pub value_bits: u64,
    pub active: c_int,
}

pub type AddNodeFn = unsafe extern "C" fn(c_int, c_int, *const c_char, c_double) -> c_int;
pub type FindNodeFn = unsafe extern "C" fn(c_int) -> *mut Node;
pub type ChildrenCountFn = unsafe extern "C" fn(c_int) -> c_int;
pub type SubtreeSumFn = unsafe extern "C" fn(c_int) -> c_double;
pub type ProcessStringFn = unsafe extern "C" fn(*mut c_char) -> c_int;
pub type SafeD2IFn = unsafe extern "C" fn(c_double) -> c_int;
pub type MaxnminFn = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

/// One loaded implementation. `_lib` must outlive the function pointers.
pub struct Api {
    pub label: String,
    pub add_node: AddNodeFn,
    pub find_node_by_id: FindNodeFn,
    pub get_children_count: ChildrenCountFn,
    pub calculate_subtree_sum: SubtreeSumFn,
    pub process_string: ProcessStringFn,
    pub safe_double_to_int: SafeD2IFn,
    pub maxnmin: MaxnminFn,
    _lib: Library,
}

unsafe fn get<T: Copy>(lib: &Library, name: &[u8]) -> T {
    let sym: Symbol<T> = lib
        .get(name)
        .unwrap_or_else(|e| panic!("missing symbol {}: {e}", String::from_utf8_lossy(name)));
    *sym
}

impl Api {
    fn load(path: &Path, label: String) -> Api {
        // RTLD_LOCAL (libloading's default) keeps the two libraries' identical
        // symbol names from colliding in the global namespace.
        let lib = unsafe { Library::new(path) }
            .unwrap_or_else(|e| panic!("failed to dlopen {}: {e}", path.display()));
        unsafe {
            Api {
                add_node: get(&lib, b"add_node\0"),
                find_node_by_id: get(&lib, b"find_node_by_id\0"),
                get_children_count: get(&lib, b"get_children_count\0"),
                calculate_subtree_sum: get(&lib, b"calculate_subtree_sum\0"),
                process_string: get(&lib, b"process_string\0"),
                safe_double_to_int: get(&lib, b"safe_double_to_int\0"),
                maxnmin: get(&lib, b"maxnmin\0"),
                label,
                _lib: lib,
            }
        }
    }

    /// Reset both implementations to a known state. There is no dedicated
    /// reset export, but `maxnmin` zeroes `node_count` and repopulates the six
    /// fixed nodes, which is deterministic and identical in both libraries.
    pub fn reset(&self) {
        unsafe { (self.maxnmin)(0, 0, 0, 0) };
    }

    pub fn snapshot(&self, p: *const Node) -> Option<NodeSnapshot> {
        if p.is_null() {
            return None;
        }
        unsafe {
            let n = &*p;
            Some(NodeSnapshot {
                id: n.id,
                parent_id: n.parent_id,
                name: n.name.iter().map(|&c| c as u8).collect(),
                value_bits: n.value.to_bits(),
                active: n.active,
            })
        }
    }
}

pub struct Impls {
    pub c: Api,
    pub rust: Vec<Api>,
}

fn find_so(dir: &Path) -> Option<PathBuf> {
    let mut hits: Vec<PathBuf> = std::fs::read_dir(dir)
        .ok()?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.is_file()
                && p.extension().map(|e| e == "so").unwrap_or(false)
                && p.file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n.starts_with("lib"))
                    .unwrap_or(false)
        })
        .collect();
    hits.sort();
    hits.into_iter().next()
}

fn load_all() -> Impls {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = manifest.parent().expect("workspace root").to_path_buf();

    let c_dir = root.join("c_src").join("build");
    let c_path = find_so(&c_dir).unwrap_or_else(|| {
        panic!(
            "no C .so found in {} -- build it with cmake first",
            c_dir.display()
        )
    });
    let c = Api::load(&c_path, format!("C:{}", c_path.display()));

    // Test every Rust cdylib artifact that exists (debug catches arithmetic
    // overflow, release is the shipped configuration).
    let mut rust = Vec::new();
    for profile in ["debug", "release"] {
        let p = manifest.join("target").join(profile).join("libmaxnmin_lib.so");
        if p.is_file() {
            rust.push(Api::load(&p, format!("rust-{profile}")));
        }
    }
    assert!(
        !rust.is_empty(),
        "no Rust cdylib found under {}/target/{{debug,release}}",
        manifest.display()
    );

    Impls { c, rust }
}

static IMPLS: OnceLock<Mutex<Impls>> = OnceLock::new();

/// Serialises access: both libraries hold mutable file-scope state, and cargo
/// runs tests concurrently in one process.
pub fn impls() -> MutexGuard<'static, Impls> {
    IMPLS
        .get_or_init(|| Mutex::new(load_all()))
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}

/// Reset every implementation to the post-`maxnmin(0,0,0,0)` baseline.
pub fn reset_all(i: &Impls) {
    i.c.reset();
    for r in &i.rust {
        r.reset();
    }
}
