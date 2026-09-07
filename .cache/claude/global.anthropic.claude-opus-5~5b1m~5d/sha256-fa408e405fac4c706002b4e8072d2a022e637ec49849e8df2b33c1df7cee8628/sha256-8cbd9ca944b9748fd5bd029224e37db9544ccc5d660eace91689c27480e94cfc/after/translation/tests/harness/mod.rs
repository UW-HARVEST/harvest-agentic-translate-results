//! Shared differential-test harness.
//!
//! BOTH libraries are loaded through `libloading` and driven only through their
//! exported symbols — the Rust crate is never linked or called directly, so the
//! `#[no_mangle] extern "C"` wrappers are part of what is under test.

#![allow(dead_code)]

pub mod crash_host;

use libloading::{Library, Symbol};
use std::ffi::c_char;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, OnceLock};

pub const MAX_NODES: usize = 50;

pub const OP_ADD: i32 = 1;
pub const OP_MULTIPLY: i32 = 2;
pub const OP_SUBTRACT: i32 = 3;
pub const OP_DIVIDE: i32 = 4;
pub const OP_MODULO: i32 = 5;

#[repr(C)]
#[derive(Copy, Clone, PartialEq, Eq)]
pub struct TreeNode {
    pub id: i32,
    pub value: i32,
    pub parent_id: i32,
    pub left_child_id: i32,
    pub right_child_id: i32,
    pub label: [u8; 32],
}

impl Default for TreeNode {
    fn default() -> Self {
        TreeNode {
            id: 0,
            value: 0,
            parent_id: 0,
            left_child_id: 0,
            right_child_id: 0,
            label: [0u8; 32],
        }
    }
}

impl std::fmt::Debug for TreeNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "TreeNode{{id:{},value:{},parent:{},l:{},r:{},label:{:?}}}",
            self.id,
            self.value,
            self.parent_id,
            self.left_child_id,
            self.right_child_id,
            self.label
        )
    }
}

pub type Fn4 = unsafe extern "C" fn(i32, i32, i32, i32) -> i32;

// ---------------------------------------------------------------------------
// Locating the two shared objects
// ---------------------------------------------------------------------------

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// The C `.so`. `CMakeLists.txt` names the project after the *parent directory*
/// of `c_src`, so the file name is not fixed — glob the build dir instead.
pub fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("C_SO_PATH") {
        return PathBuf::from(p);
    }
    let build = crate_root().join("../c_src/build");
    let mut found: Vec<PathBuf> = std::fs::read_dir(&build)
        .unwrap_or_else(|e| panic!("cannot read {}: {e} — build the C lib first", build.display()))
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.extension().map(|x| x == "so").unwrap_or(false)
                && p.file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n.starts_with("lib"))
                    .unwrap_or(false)
        })
        .collect();
    found.sort();
    assert_eq!(
        found.len(),
        1,
        "expected exactly one lib*.so in {}, got {:?}",
        build.display(),
        found
    );
    found.pop().unwrap()
}

/// The Rust `cdylib`. Prefer the profile the tests were built with.
pub fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_SO_PATH") {
        return PathBuf::from(p);
    }
    let name = "libinreftree_lib.so";
    // Walk up from the test executable: target/<profile>/deps/<test> -> target/<profile>
    if let Ok(exe) = std::env::current_exe() {
        if let Some(profile_dir) = exe.parent().and_then(|p| p.parent()) {
            let cand = profile_dir.join(name);
            if cand.exists() {
                return cand;
            }
        }
    }
    for p in ["target/release", "target/debug"] {
        let cand = crate_root().join(p).join(name);
        if cand.exists() {
            return cand;
        }
    }
    panic!("{name} not found; run `cargo build --release` first");
}

// ---------------------------------------------------------------------------
// Loaded library handle
// ---------------------------------------------------------------------------

pub struct Lib {
    pub name: &'static str,
    _lib: Library,
    pub add_op: Fn4,
    pub multiply_op: Fn4,
    pub subtract_op: Fn4,
    pub divide_op: Fn4,
    pub modulo_op: Fn4,
    pub find_node_by_id: unsafe extern "C" fn(i32) -> *mut TreeNode,
    pub add_tree_node: unsafe extern "C" fn(i32, i32, i32, *const c_char) -> i32,
    pub calculate_tree_sum: unsafe extern "C" fn(i32) -> i32,
    pub parse_operation: unsafe extern "C" fn(*const c_char) -> i32,
    pub get_operation_func: unsafe extern "C" fn(i32) -> Fn4,
    pub inreftree: Fn4,
    pub node_table: *mut TreeNode,
    pub node_count: *mut i32,
}

// The handles are only ever touched while holding `state_lock()`.
unsafe impl Send for Lib {}
unsafe impl Sync for Lib {}

unsafe fn sym<T: Copy>(lib: &Library, n: &str) -> T {
    let s: Symbol<T> = lib
        .get(n.as_bytes())
        .unwrap_or_else(|e| panic!("missing symbol `{n}`: {e}"));
    *s
}

impl Lib {
    pub fn open(name: &'static str, path: &Path) -> Lib {
        unsafe {
            let lib = Library::new(path)
                .unwrap_or_else(|e| panic!("dlopen({}) failed: {e}", path.display()));
            Lib {
                name,
                add_op: sym(&lib, "add_op"),
                multiply_op: sym(&lib, "multiply_op"),
                subtract_op: sym(&lib, "subtract_op"),
                divide_op: sym(&lib, "divide_op"),
                modulo_op: sym(&lib, "modulo_op"),
                find_node_by_id: sym(&lib, "find_node_by_id"),
                add_tree_node: sym(&lib, "add_tree_node"),
                calculate_tree_sum: sym(&lib, "calculate_tree_sum"),
                parse_operation: sym(&lib, "parse_operation"),
                get_operation_func: sym(&lib, "get_operation_func"),
                inreftree: sym(&lib, "inreftree"),
                node_table: sym::<*mut TreeNode>(&lib, "node_table"),
                node_count: sym::<*mut i32>(&lib, "node_count"),
                _lib: lib,
            }
        }
    }

    // --- global state accessors -------------------------------------------

    pub fn count(&self) -> i32 {
        unsafe { *self.node_count }
    }
    pub fn set_count(&self, v: i32) {
        unsafe { *self.node_count = v }
    }
    /// Whole 2600-byte `node_table` as raw bytes.
    pub fn table_bytes(&self) -> Vec<u8> {
        unsafe {
            std::slice::from_raw_parts(
                self.node_table as *const u8,
                MAX_NODES * std::mem::size_of::<TreeNode>(),
            )
            .to_vec()
        }
    }
    pub fn table(&self) -> Vec<TreeNode> {
        unsafe { std::slice::from_raw_parts(self.node_table, MAX_NODES).to_vec() }
    }
    pub fn set_table(&self, nodes: &[TreeNode]) {
        assert!(nodes.len() <= MAX_NODES);
        unsafe {
            for (i, n) in nodes.iter().enumerate() {
                *self.node_table.add(i) = *n;
            }
        }
    }
    pub fn zero_state(&self) {
        unsafe {
            std::ptr::write_bytes(self.node_table as *mut u8, 0, MAX_NODES * 52);
            *self.node_count = 0;
        }
    }
    /// `find_node_by_id` result normalised to an index into *this* library's
    /// own `node_table` (or `None` for NULL), so the two are comparable.
    pub fn find_index(&self, id: i32) -> Option<isize> {
        unsafe {
            let p = (self.find_node_by_id)(id);
            if p.is_null() {
                return None;
            }
            let off = (p as isize) - (self.node_table as isize);
            assert_eq!(off % 52, 0, "{}: unaligned node pointer {off}", self.name);
            Some(off / 52)
        }
    }
    /// Which `*_op` symbol of *this* library the returned pointer is.
    pub fn op_func_name(&self, op: i32) -> &'static str {
        let f = unsafe { (self.get_operation_func)(op) } as usize;
        for (n, g) in [
            ("add_op", self.add_op),
            ("multiply_op", self.multiply_op),
            ("subtract_op", self.subtract_op),
            ("divide_op", self.divide_op),
            ("modulo_op", self.modulo_op),
        ] {
            if g as usize == f {
                return n;
            }
        }
        panic!("{}: get_operation_func({op}) returned unknown pointer {f:#x}", self.name)
    }
}

// ---------------------------------------------------------------------------
// Singletons + serialisation (both libs hold mutable process-wide globals)
// ---------------------------------------------------------------------------

pub struct Pair {
    pub c: Lib,
    pub r: Lib,
}

static PAIR: OnceLock<Pair> = OnceLock::new();
static LOCK: OnceLock<Mutex<()>> = OnceLock::new();

/// `cargo test --test <t>` does NOT rebuild the `cdylib` (an integration test
/// cannot link a cdylib, so it is not a dependency of the test target). A stale
/// `.so` would silently make every differential test vacuous, so refuse to run
/// unless the `.so` is newer than every Rust source file.
fn assert_so_fresh(so: &Path) {
    let so_m = match std::fs::metadata(so).and_then(|m| m.modified()) {
        Ok(m) => m,
        Err(_) => return,
    };
    let src = crate_root().join("src");
    let mut newest = None;
    let mut stack = vec![src];
    while let Some(d) = stack.pop() {
        if let Ok(rd) = std::fs::read_dir(&d) {
            for e in rd.flatten() {
                let p = e.path();
                if p.is_dir() {
                    stack.push(p);
                } else if p.extension().map(|x| x == "rs").unwrap_or(false) {
                    if let Ok(m) = e.metadata().and_then(|m| m.modified()) {
                        if newest.map(|n| m > n).unwrap_or(true) {
                            newest = Some(m);
                        }
                    }
                }
            }
        }
    }
    if let Some(n) = newest {
        assert!(
            so_m >= n,
            "STALE {}: it is older than translation/src/*.rs.\n\
             Run `cargo build --release` (or ./run_all.sh) before `cargo test` —\n\
             otherwise the differential tests compare against an old library.",
            so.display()
        );
    }
}

pub fn pair() -> &'static Pair {
    PAIR.get_or_init(|| {
        let rso = rust_so_path();
        assert_so_fresh(&rso);
        Pair {
            c: Lib::open("C", &c_so_path()),
            r: Lib::open("Rust", &rso),
        }
    })
}

/// Serialises tests: `node_table` / `node_count` are process-wide.
pub fn state_lock() -> MutexGuard<'static, ()> {
    match LOCK.get_or_init(|| Mutex::new(())).lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    }
}

/// Grab the pair, take the lock, and reset both libraries' globals.
pub fn fresh() -> (&'static Pair, MutexGuard<'static, ()>) {
    let g = state_lock();
    let p = pair();
    p.c.zero_state();
    p.r.zero_state();
    (p, g)
}

/// Assert both libraries' `node_count` + full `node_table` bytes are identical.
#[track_caller]
pub fn assert_state_eq(p: &Pair, ctx: &str) {
    assert_eq!(p.c.count(), p.r.count(), "node_count mismatch @ {ctx}");
    let (cb, rb) = (p.c.table_bytes(), p.r.table_bytes());
    if cb != rb {
        let i = cb.iter().zip(&rb).position(|(a, b)| a != b).unwrap();
        panic!(
            "node_table byte mismatch @ {ctx}: first diff at byte {i} (node {} field-off {}): C={} Rust={}\nC node    = {:?}\nRust node = {:?}",
            i / 52,
            i % 52,
            cb[i],
            rb[i],
            p.c.table()[i / 52],
            p.r.table()[i / 52],
        );
    }
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (SplitMix64) — fixed seeds, reproducible runs
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed)
    }
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    pub fn i32(&mut self) -> i32 {
        self.next_u64() as u32 as i32
    }
    /// Biased toward interesting values: corners, small ints, full range.
    pub fn spicy_i32(&mut self) -> i32 {
        const CORNERS: [i32; 12] = [
            0, 1, -1, 2, -2, 3, -3, 4, -4, i32::MIN, i32::MAX, i32::MIN + 1,
        ];
        match self.next_u64() % 4 {
            0 => CORNERS[(self.next_u64() % CORNERS.len() as u64) as usize],
            1 => (self.next_u64() % 41) as i32 - 20,
            2 => self.i32() >> (self.next_u64() % 31) as u32,
            _ => self.i32(),
        }
    }
    pub fn below(&mut self, n: u64) -> u64 {
        self.next_u64() % n
    }
}
