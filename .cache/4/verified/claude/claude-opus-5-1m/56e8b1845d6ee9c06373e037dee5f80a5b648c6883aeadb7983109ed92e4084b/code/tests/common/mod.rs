//! Shared differential-test harness.
//!
//! Loads BOTH shared libraries through `libloading` and calls everything through
//! `dlsym`-resolved symbols. The Rust crate is never linked directly, so the
//! `#[no_mangle]` / `extern "C"` export wrappers are part of what is under test.
//!
//!   * C    `.so`: `$C_SO`    or `c_src/build/libtranslated_rust.so`
//!   * Rust `.so`: `$RUST_SO` or `target/{release,debug}/libinreftree_lib.so`
//!
//! Both libraries own mutable global state (`node_table`, `node_count`), and
//! `cargo test` runs test functions on multiple threads inside ONE process
//! (where `dlopen` is reference-counted, i.e. the state really is shared). Every
//! test therefore goes through [`with_libs`], which takes a global lock and
//! zeroes both libraries' state first, making tests order- and thread-independent.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, c_void};
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, OnceLock};

// ---------------------------------------------------------------------------
// Types mirroring the C ABI
// ---------------------------------------------------------------------------

/// `sizeof(TreeNode)` == 5*4 + 32 == 52 (no padding: 4-byte alignment, and the
/// trailing `char[32]` keeps the total a multiple of 4).
pub const TREE_NODE_SIZE: usize = 52;
pub const MAX_NODES: usize = 50;
/// `sizeof(node_table)` == 50 * 52 == 2600 == 0xa28 (matches `nm -D -S`).
pub const NODE_TABLE_BYTES: usize = MAX_NODES * TREE_NODE_SIZE;

pub const OP_ADD: c_int = 1;
pub const OP_MULTIPLY: c_int = 2;
pub const OP_SUBTRACT: c_int = 3;
pub const OP_DIVIDE: c_int = 4;
pub const OP_MODULO: c_int = 5;

pub type OpFn = extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

/// Decoded view of one `TreeNode` row, for readable assertion messages.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeView {
    pub id: i32,
    pub value: i32,
    pub parent_id: i32,
    pub left_child_id: i32,
    pub right_child_id: i32,
    pub label: [u8; 32],
}

impl NodeView {
    fn from_bytes(b: &[u8]) -> Self {
        let g = |o: usize| i32::from_ne_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]]);
        let mut label = [0u8; 32];
        label.copy_from_slice(&b[20..52]);
        NodeView {
            id: g(0),
            value: g(4),
            parent_id: g(8),
            left_child_id: g(12),
            right_child_id: g(16),
            label,
        }
    }
}

// ---------------------------------------------------------------------------
// One loaded library
// ---------------------------------------------------------------------------

pub struct Lib {
    pub name: &'static str,
    pub path: PathBuf,
    // Raw fn pointers are copied out of the `Symbol`s, so `Lib` is not
    // self-referential; `_lib` only keeps the mapping alive.
    _lib: Library,
    pub add_op: OpFn,
    pub multiply_op: OpFn,
    pub subtract_op: OpFn,
    pub divide_op: OpFn,
    pub modulo_op: OpFn,
    pub find_node_by_id: extern "C" fn(c_int) -> *mut c_void,
    pub add_tree_node: extern "C" fn(c_int, c_int, c_int, *const c_char) -> c_int,
    pub calculate_tree_sum: extern "C" fn(c_int) -> c_int,
    pub parse_operation: extern "C" fn(*const c_char) -> c_int,
    // Declared as a raw pointer return (not `Option<fn>`) so a NULL return would
    // be observable rather than UB.
    pub get_operation_func: extern "C" fn(c_int) -> *const c_void,
    pub inreftree: extern "C" fn(c_int, c_int, c_int, c_int) -> c_int,
    pub node_table: *mut u8,
    pub node_count: *mut c_int,
}

macro_rules! fun {
    ($lib:expr, $n:literal) => {{
        let s: Symbol<_> = unsafe { $lib.get(concat!($n, "\0").as_bytes()) }
            .unwrap_or_else(|e| panic!("dlsym {} failed: {e}", $n));
        *s
    }};
}

macro_rules! data {
    ($lib:expr, $n:literal, $t:ty) => {{
        let s: Symbol<$t> = unsafe { $lib.get(concat!($n, "\0").as_bytes()) }
            .unwrap_or_else(|e| panic!("dlsym {} failed: {e}", $n));
        *s
    }};
}

impl Lib {
    fn open(name: &'static str, path: PathBuf) -> Lib {
        let lib = unsafe { Library::new(&path) }
            .unwrap_or_else(|e| panic!("dlopen {} ({}) failed: {e}", name, path.display()));
        Lib {
            name,
            add_op: fun!(lib, "add_op"),
            multiply_op: fun!(lib, "multiply_op"),
            subtract_op: fun!(lib, "subtract_op"),
            divide_op: fun!(lib, "divide_op"),
            modulo_op: fun!(lib, "modulo_op"),
            find_node_by_id: fun!(lib, "find_node_by_id"),
            add_tree_node: fun!(lib, "add_tree_node"),
            calculate_tree_sum: fun!(lib, "calculate_tree_sum"),
            parse_operation: fun!(lib, "parse_operation"),
            get_operation_func: fun!(lib, "get_operation_func"),
            inreftree: fun!(lib, "inreftree"),
            node_table: data!(lib, "node_table", *mut u8),
            node_count: data!(lib, "node_count", *mut c_int),
            path,
            _lib: lib,
        }
    }

    /// Address of an exported function, for comparing against the pointer
    /// returned by `get_operation_func`.
    pub fn op_addr(&self, which: c_int) -> usize {
        let f: OpFn = match which {
            OP_ADD => self.add_op,
            OP_MULTIPLY => self.multiply_op,
            OP_SUBTRACT => self.subtract_op,
            OP_DIVIDE => self.divide_op,
            OP_MODULO => self.modulo_op,
            _ => panic!("bad op {which}"),
        };
        f as usize
    }

    // -- global state access -------------------------------------------------

    pub fn get_count(&self) -> c_int {
        unsafe { *self.node_count }
    }

    pub fn set_count(&self, v: c_int) {
        unsafe { *self.node_count = v }
    }

    /// Full byte image of the exported `node_table` object (all 2600 bytes).
    pub fn table_image(&self) -> Vec<u8> {
        unsafe { std::slice::from_raw_parts(self.node_table, NODE_TABLE_BYTES).to_vec() }
    }

    pub fn set_table_image(&self, bytes: &[u8]) {
        assert_eq!(bytes.len(), NODE_TABLE_BYTES);
        unsafe { std::ptr::copy_nonoverlapping(bytes.as_ptr(), self.node_table, NODE_TABLE_BYTES) }
    }

    pub fn node(&self, idx: usize) -> NodeView {
        assert!(idx < MAX_NODES);
        let img = self.table_image();
        NodeView::from_bytes(&img[idx * TREE_NODE_SIZE..(idx + 1) * TREE_NODE_SIZE])
    }

    /// Write one row directly through the exported `node_table` pointer.
    pub fn set_node(&self, idx: usize, n: &NodeView) {
        assert!(idx < MAX_NODES);
        let mut buf = [0u8; TREE_NODE_SIZE];
        buf[0..4].copy_from_slice(&n.id.to_ne_bytes());
        buf[4..8].copy_from_slice(&n.value.to_ne_bytes());
        buf[8..12].copy_from_slice(&n.parent_id.to_ne_bytes());
        buf[12..16].copy_from_slice(&n.left_child_id.to_ne_bytes());
        buf[16..20].copy_from_slice(&n.right_child_id.to_ne_bytes());
        buf[20..52].copy_from_slice(&n.label);
        unsafe {
            std::ptr::copy_nonoverlapping(
                buf.as_ptr(),
                self.node_table.add(idx * TREE_NODE_SIZE),
                TREE_NODE_SIZE,
            )
        }
    }

    pub fn reset(&self) {
        self.set_count(0);
        unsafe { std::ptr::write_bytes(self.node_table, 0, NODE_TABLE_BYTES) };
    }

    /// `find_node_by_id`, normalised to a table index so the two libraries'
    /// distinct base addresses are comparable.
    pub fn find_index(&self, id: c_int) -> Option<isize> {
        let p = (self.find_node_by_id)(id) as *const u8;
        if p.is_null() {
            return None;
        }
        let off = unsafe { p.offset_from(self.node_table as *const u8) };
        assert_eq!(
            off % TREE_NODE_SIZE as isize,
            0,
            "{}: find_node_by_id returned a misaligned pointer (offset {off})",
            self.name
        );
        Some(off / TREE_NODE_SIZE as isize)
    }

    pub fn add_node(&self, id: c_int, value: c_int, parent_id: c_int, label: &[u8]) -> c_int {
        let mut z: Vec<u8> = label.to_vec();
        z.push(0);
        (self.add_tree_node)(id, value, parent_id, z.as_ptr() as *const c_char)
    }

    pub fn parse_op(&self, s: &[u8]) -> c_int {
        let mut z: Vec<u8> = s.to_vec();
        z.push(0);
        (self.parse_operation)(z.as_ptr() as *const c_char)
    }

    pub fn parse_op_null(&self) -> c_int {
        (self.parse_operation)(std::ptr::null())
    }
}

// ---------------------------------------------------------------------------
// Global, serialised access to the pair of libraries
// ---------------------------------------------------------------------------

pub struct Pair {
    pub c: Lib,
    pub rust: Lib,
}

// The raw `node_table` / `node_count` pointers make `Lib` non-`Send`. All access
// goes through the single global `Mutex<Pair>` in `with_libs`, so the pair is
// only ever touched by one thread at a time.
unsafe impl Send for Lib {}
unsafe impl Sync for Lib {}
unsafe impl Send for Pair {}
unsafe impl Sync for Pair {}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("C_SO") {
        return PathBuf::from(p);
    }
    let p = manifest_dir().join("c_src/build/libtranslated_rust.so");
    assert!(
        p.exists(),
        "C shared library not found at {}.\nBuild it first:\n  cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        p.display()
    );
    p
}

fn mtime(p: &std::path::Path) -> Option<std::time::SystemTime> {
    std::fs::metadata(p).ok()?.modified().ok()
}

/// The profile this test binary was built with, inferred from its own path
/// (`target/<profile>/deps/<test>-<hash>`), so the test always loads the `.so`
/// that matches how the tests themselves were compiled.
fn current_profile() -> String {
    let exe = std::env::current_exe().unwrap_or_default();
    exe.parent()
        .and_then(|p| p.parent())
        .and_then(|p| p.file_name())
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "debug".to_string())
}

/// `cargo test` does NOT rebuild a `cdylib`-only lib target: integration tests
/// cannot link it, so cargo sees no dependency edge and skips it. Without this
/// step the tests would silently load a STALE `.so` and pass no matter what
/// `src/lib.rs` says. So build it explicitly here, then hard-fail if the artifact
/// is still older than the sources.
fn ensure_rust_so_fresh(so: &std::path::Path) {
    let md = manifest_dir();
    if std::env::var("NO_AUTO_BUILD").is_err() {
        let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
        let mut cmd = std::process::Command::new(cargo);
        cmd.arg("build").arg("--lib").current_dir(&md);
        if current_profile() == "release" {
            cmd.arg("--release");
        }
        // Do not inherit the parent cargo's target-dir juggling env vars.
        cmd.env_remove("CARGO_MAKEFLAGS");
        match cmd.output() {
            Ok(o) if o.status.success() => {}
            Ok(o) => eprintln!(
                "warning: auto `cargo build --lib` failed:\n{}",
                String::from_utf8_lossy(&o.stderr)
            ),
            Err(e) => eprintln!("warning: could not run cargo to rebuild the cdylib: {e}"),
        }
    }

    assert!(
        so.exists(),
        "Rust cdylib not found at {}.\nBuild it first:  cargo build --release",
        so.display()
    );

    // Staleness guard: the .so must be at least as new as every input to it.
    let so_t = mtime(so).expect("stat the Rust .so");
    for src in ["src/lib.rs", "Cargo.toml"] {
        let p = md.join(src);
        if let Some(t) = mtime(&p) {
            assert!(
                so_t >= t,
                "STALE ARTIFACT: {} is older than {}.\n\
                 `cargo test` does not rebuild a cdylib-only lib target, so the tests would \
                 have verified an out-of-date library.\nRun:  cargo build{} --lib",
                so.display(),
                p.display(),
                if current_profile() == "release" { " --release" } else { "" }
            );
        }
    }
}

fn rust_so_path() -> PathBuf {
    let p = match std::env::var("RUST_SO") {
        Ok(p) => PathBuf::from(p),
        Err(_) => manifest_dir()
            .join("target")
            .join(current_profile())
            .join("libinreftree_lib.so"),
    };
    ensure_rust_so_fresh(&p);
    p
}

static PAIR: OnceLock<Mutex<Pair>> = OnceLock::new();

fn pair() -> &'static Mutex<Pair> {
    PAIR.get_or_init(|| {
        let p = Pair {
            c: Lib::open("C", c_so_path()),
            rust: Lib::open("Rust", rust_so_path()),
        };
        // Sanity: the two libraries must be distinct objects with distinct
        // state, otherwise the whole comparison would be vacuous.
        assert_ne!(
            p.c.node_table as usize, p.rust.node_table as usize,
            "C and Rust node_table resolved to the same address - the libraries \
             are not isolated, so every comparison would be trivially true"
        );
        assert_ne!(p.c.inreftree as usize, p.rust.inreftree as usize);
        Mutex::new(p)
    })
}

/// Run `f` with both libraries, serialised against all other tests, with both
/// libraries' global state zeroed first.
pub fn with_libs<R>(f: impl FnOnce(&Pair) -> R) -> R {
    let g: MutexGuard<'_, Pair> = pair().lock().unwrap_or_else(|e| e.into_inner());
    g.c.reset();
    g.rust.reset();
    f(&g)
}

// ---------------------------------------------------------------------------
// Assertion helpers
// ---------------------------------------------------------------------------

/// Assert the two libraries' full observable state (`node_count` + the entire
/// 2600-byte `node_table` image) is byte-identical.
#[track_caller]
pub fn assert_state_eq(p: &Pair, ctx: &str) {
    assert_eq!(
        p.c.get_count(),
        p.rust.get_count(),
        "node_count mismatch [{ctx}]"
    );
    let (a, b) = (p.c.table_image(), p.rust.table_image());
    if a != b {
        let i = a.iter().zip(&b).position(|(x, y)| x != y).unwrap();
        let row = i / TREE_NODE_SIZE;
        panic!(
            "node_table mismatch [{ctx}] at byte {i} (row {row}, offset {}):\n  C   = {:?}\n  Rust= {:?}",
            i % TREE_NODE_SIZE,
            p.c.node(row),
            p.rust.node(row),
        );
    }
}

#[track_caller]
pub fn assert_ret_eq(cv: c_int, rv: c_int, ctx: &str) {
    assert_eq!(cv, rv, "return value mismatch [{ctx}]: C={cv} Rust={rv}");
}

// ---------------------------------------------------------------------------
// Deterministic RNG (SplitMix64) - fixed seed, reproducible
// ---------------------------------------------------------------------------

pub const SEED: u64 = 0x9E37_79B9_7F4A_7C15;

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
    /// Small-magnitude value, so sums stay in range and hit every `% 4` class.
    pub fn small(&mut self) -> i32 {
        (self.next_u64() % 201) as i32 - 100
    }
    pub fn below(&mut self, n: u64) -> u64 {
        self.next_u64() % n
    }
    /// Mix of extreme and random values - the interesting distribution for
    /// overflow-sensitive arithmetic.
    pub fn spicy_i32(&mut self) -> i32 {
        match self.below(10) {
            0 => i32::MIN,
            1 => i32::MAX,
            2 => i32::MIN + 1,
            3 => i32::MAX - 1,
            4 => 0,
            5 => -1,
            6 => 1,
            7 => self.small(),
            _ => self.i32(),
        }
    }
}

/// The interesting scalar boundary values.
pub const EDGE: [i32; 11] = [
    0,
    1,
    -1,
    2,
    -2,
    3,
    -3,
    i32::MIN,
    i32::MIN + 1,
    i32::MAX,
    i32::MAX - 1,
];
