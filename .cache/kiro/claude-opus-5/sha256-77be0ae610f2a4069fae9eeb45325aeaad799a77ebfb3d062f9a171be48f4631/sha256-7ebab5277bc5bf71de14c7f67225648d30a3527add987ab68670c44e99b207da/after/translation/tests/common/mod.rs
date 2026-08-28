//! Shared harness: locates and loads both the C and the Rust shared objects.
//!
//! Every call in these tests goes through `libloading`, i.e. through the
//! `#[no_mangle]` exported symbols, exactly as an external C caller would.

#![allow(dead_code)]

use std::ffi::{c_char, c_int};
use std::path::{Path, PathBuf};

pub const MAX_NODES: usize = 50;

/// Mirror of the C `TreeNode` layout, used to read each library's
/// `node_table` global through the FFI boundary.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct TreeNode {
    pub id: c_int,
    pub value: c_int,
    pub parent_id: c_int,
    pub left_child_id: c_int,
    pub right_child_id: c_int,
    pub label: [c_char; 32],
}

impl std::fmt::Debug for TreeNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let raw: Vec<u8> = self.label.iter().map(|&b| b as u8).collect();
        f.debug_struct("TreeNode")
            .field("id", &self.id)
            .field("value", &self.value)
            .field("parent_id", &self.parent_id)
            .field("left_child_id", &self.left_child_id)
            .field("right_child_id", &self.right_child_id)
            .field("label_bytes", &raw)
            .finish()
    }
}

pub type OpFn = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

/// One loaded implementation (either the C `.so` or the Rust `.so`).
pub struct Lib {
    pub name: &'static str,
    lib: libloading::Library,
}

impl Lib {
    fn open(name: &'static str, path: &Path) -> Lib {
        let lib = unsafe { libloading::Library::new(path) }
            .unwrap_or_else(|e| panic!("failed to load {} from {}: {e}", name, path.display()));
        Lib { name, lib }
    }

    fn sym<T>(&self, symbol: &str) -> libloading::Symbol<'_, T> {
        unsafe { self.lib.get(symbol.as_bytes()) }
            .unwrap_or_else(|e| panic!("{} is missing symbol `{}`: {e}", self.name, symbol))
    }

    // --- arithmetic ops ---------------------------------------------------

    pub fn op_ptr(&self, symbol: &str) -> *const () {
        let f: libloading::Symbol<OpFn> = self.sym(symbol);
        (unsafe { f.into_raw() }).into_raw() as *const ()
    }

    pub fn call_op(&self, symbol: &str, a: c_int, b: c_int, u1: c_int, u2: c_int) -> c_int {
        let f: libloading::Symbol<OpFn> = self.sym(symbol);
        unsafe { f(a, b, u1, u2) }
    }

    // --- tree table ------------------------------------------------------

    pub fn find_node_by_id(&self, id: c_int) -> *mut TreeNode {
        let f: libloading::Symbol<unsafe extern "C" fn(c_int) -> *mut TreeNode> =
            self.sym("find_node_by_id");
        unsafe { f(id) }
    }

    pub fn add_tree_node(
        &self,
        id: c_int,
        value: c_int,
        parent_id: c_int,
        label: &[u8],
    ) -> c_int {
        assert_eq!(
            label.last().copied(),
            Some(0),
            "label must be NUL terminated"
        );
        let f: libloading::Symbol<
            unsafe extern "C" fn(c_int, c_int, c_int, *const c_char) -> c_int,
        > = self.sym("add_tree_node");
        unsafe { f(id, value, parent_id, label.as_ptr() as *const c_char) }
    }

    pub fn calculate_tree_sum(&self, node_id: c_int) -> c_int {
        let f: libloading::Symbol<unsafe extern "C" fn(c_int) -> c_int> =
            self.sym("calculate_tree_sum");
        unsafe { f(node_id) }
    }

    pub fn parse_operation(&self, s: Option<&[u8]>) -> c_int {
        let f: libloading::Symbol<unsafe extern "C" fn(*const c_char) -> c_int> =
            self.sym("parse_operation");
        match s {
            None => unsafe { f(std::ptr::null()) },
            Some(bytes) => {
                assert_eq!(bytes.last().copied(), Some(0), "must be NUL terminated");
                unsafe { f(bytes.as_ptr() as *const c_char) }
            }
        }
    }

    pub fn get_operation_func(&self, op: c_int) -> *const () {
        let f: libloading::Symbol<unsafe extern "C" fn(c_int) -> *const ()> =
            self.sym("get_operation_func");
        unsafe { f(op) }
    }

    pub fn inreftree(&self, a: c_int, b: c_int, c: c_int, d: c_int) -> c_int {
        let f: libloading::Symbol<unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int> =
            self.sym("inreftree");
        unsafe { f(a, b, c, d) }
    }

    // --- globals ---------------------------------------------------------

    pub fn node_count(&self) -> c_int {
        let p: libloading::Symbol<*mut c_int> = self.sym("node_count");
        unsafe { **p }
    }

    pub fn set_node_count(&self, v: c_int) {
        let p: libloading::Symbol<*mut c_int> = self.sym("node_count");
        unsafe { **p = v }
    }

    pub fn node_table_ptr(&self) -> *mut TreeNode {
        let p: libloading::Symbol<*mut TreeNode> = self.sym("node_table");
        *p
    }

    /// Snapshot of the whole 50-entry table, byte for byte.
    pub fn node_table(&self) -> Vec<TreeNode> {
        let base = self.node_table_ptr();
        (0..MAX_NODES)
            .map(|i| unsafe { std::ptr::read(base.add(i)) })
            .collect()
    }

    /// Raw bytes of the whole table, for byte-identical comparison.
    pub fn node_table_bytes(&self) -> Vec<u8> {
        let base = self.node_table_ptr() as *const u8;
        let len = MAX_NODES * std::mem::size_of::<TreeNode>();
        unsafe { std::slice::from_raw_parts(base, len) }.to_vec()
    }

    /// Zero the whole table and the counter, so both libraries start from an
    /// identical state before a scenario.
    pub fn reset(&self) {
        let base = self.node_table_ptr() as *mut u8;
        let len = MAX_NODES * std::mem::size_of::<TreeNode>();
        unsafe { std::ptr::write_bytes(base, 0, len) };
        self.set_node_count(0);
    }
}

fn workspace_root() -> PathBuf {
    // translation/ -> parent is the working directory holding c_src/
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest.parent().expect("manifest has a parent").to_path_buf()
}

fn find_c_so() -> PathBuf {
    if let Ok(p) = std::env::var("C_SO_PATH") {
        return PathBuf::from(p);
    }
    let build = workspace_root().join("c_src/build");
    let mut candidates: Vec<PathBuf> = std::fs::read_dir(&build)
        .unwrap_or_else(|e| panic!("cannot read {}: {e} (build the C library first)", build.display()))
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.extension().and_then(|s| s.to_str()) == Some("so")
                && p.file_name()
                    .and_then(|s| s.to_str())
                    .is_some_and(|s| s.starts_with("lib"))
        })
        .collect();
    candidates.sort();
    candidates
        .pop()
        .unwrap_or_else(|| panic!("no .so found in {}", build.display()))
}

fn find_rust_so() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_SO_PATH") {
        return PathBuf::from(p);
    }
    // The integration-test binary lives in target/<profile>/deps/, so the
    // cdylib built by the same `cargo test` invocation is one level up.
    let exe = std::env::current_exe().expect("current_exe");
    let deps = exe.parent().expect("deps dir");
    let profile_dir = deps.parent().expect("profile dir");
    for dir in [profile_dir, deps] {
        let p = dir.join("libinreftree_lib.so");
        if p.exists() {
            return p;
        }
    }
    panic!(
        "libinreftree_lib.so not found near {}; run `cargo build` first",
        profile_dir.display()
    );
}

/// Path to the C shared object under test.
pub fn c_so_path() -> PathBuf {
    find_c_so()
}

/// Path to the Rust shared object under test.
pub fn rust_so_path() -> PathBuf {
    find_rust_so()
}

/// Both implementations, loaded side by side.
///
/// The two shared objects each have exactly one copy of `node_table` /
/// `node_count` per process, so tests that touch those globals must not run
/// concurrently. Holding this guard serialises them.
pub struct Pair {
    pub c: &'static Lib,
    pub rs: &'static Lib,
    _guard: std::sync::MutexGuard<'static, ()>,
}

static LIBS: std::sync::OnceLock<(Lib, Lib)> = std::sync::OnceLock::new();
static STATE_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

pub fn load() -> Pair {
    // A panicking test poisons the mutex; the globals are re-zeroed by every
    // scenario anyway, so recover rather than cascade failures.
    let guard = STATE_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let (c, rs) = LIBS.get_or_init(|| {
        (
            Lib::open("C", &find_c_so()),
            Lib::open("Rust", &find_rust_so()),
        )
    });
    Pair {
        c,
        rs,
        _guard: guard,
    }
}

/// A spread of interesting `int` values, kept small enough that the cross
/// products below stay fast.
pub fn interesting_ints() -> Vec<c_int> {
    vec![
        0, 1, -1, 2, -2, 3, -3, 4, -4, 5, -5, 7, -7, 8, 16, 31, 32, 33, 63, 64, 100, -100, 255,
        256, 1000, -1000, 65535, 65536, 1_000_000, -1_000_000, 0x3FFF_FFFF, -0x3FFF_FFFF,
        c_int::MAX, c_int::MIN, c_int::MAX - 1, c_int::MIN + 1,
    ]
}
