//! Shared differential-testing harness.
//!
//! Loads BOTH shared objects with `libloading` and drives them through their
//! exported symbols only — the Rust crate is never linked or called directly, so
//! the `#[no_mangle]` / `extern "C"` wrappers are part of what is under test.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::c_char;
use std::ffi::c_int;
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, OnceLock};

pub const MAX_NODES: usize = 50;
pub const TREE_NODE_SIZE: usize = 52;

/// `#[repr(C)]` mirror of the C `TreeNode`, used only to inspect raw memory.
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct TreeNode {
    pub id: c_int,
    pub value: c_int,
    pub parent_id: c_int,
    pub left_child_id: c_int,
    pub right_child_id: c_int,
    pub label: [c_char; 32],
}

impl TreeNode {
    pub fn zeroed() -> Self {
        TreeNode {
            id: 0,
            value: 0,
            parent_id: 0,
            left_child_id: 0,
            right_child_id: 0,
            label: [0; 32],
        }
    }
}

pub type Op4 = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;
pub type FindFn = unsafe extern "C" fn(c_int) -> *mut TreeNode;
pub type AddFn = unsafe extern "C" fn(c_int, c_int, c_int, *const c_char) -> c_int;
pub type SumFn = unsafe extern "C" fn(c_int) -> c_int;
pub type ParseFn = unsafe extern "C" fn(*const c_char) -> c_int;
pub type GetFuncFn = unsafe extern "C" fn(c_int) -> Op4;

/// One loaded library plus resolved symbol addresses.
pub struct Lib {
    pub name: &'static str,
    lib: Library,
}

impl Lib {
    fn open(name: &'static str, path: PathBuf) -> Lib {
        let lib = unsafe { Library::new(&path) }
            .unwrap_or_else(|e| panic!("failed to dlopen {} ({}): {e}", name, path.display()));
        Lib { name, lib }
    }

    fn f<T>(&self, sym: &str) -> Symbol<'_, T> {
        unsafe { self.lib.get(sym.as_bytes()) }
            .unwrap_or_else(|e| panic!("{}: missing symbol `{sym}`: {e}", self.name))
    }

    pub fn add_op(&self, a: c_int, b: c_int, u1: c_int, u2: c_int) -> c_int {
        unsafe { (self.f::<Op4>("add_op"))(a, b, u1, u2) }
    }
    pub fn multiply_op(&self, a: c_int, b: c_int, u1: c_int, u2: c_int) -> c_int {
        unsafe { (self.f::<Op4>("multiply_op"))(a, b, u1, u2) }
    }
    pub fn subtract_op(&self, a: c_int, b: c_int, u1: c_int, u2: c_int) -> c_int {
        unsafe { (self.f::<Op4>("subtract_op"))(a, b, u1, u2) }
    }
    pub fn divide_op(&self, a: c_int, b: c_int, u1: c_int, u2: c_int) -> c_int {
        unsafe { (self.f::<Op4>("divide_op"))(a, b, u1, u2) }
    }
    pub fn modulo_op(&self, a: c_int, b: c_int, u1: c_int, u2: c_int) -> c_int {
        unsafe { (self.f::<Op4>("modulo_op"))(a, b, u1, u2) }
    }

    /// Returns the found node's index within `node_table`, or `None` for NULL.
    /// Raw pointers differ between the two `.so`s, so the *index* is the
    /// comparable observation. Also asserts the pointer is either NULL or
    /// exactly on a `node_table` slot boundary.
    pub fn find_node_by_id(&self, id: c_int) -> Option<usize> {
        let p = unsafe { (self.f::<FindFn>("find_node_by_id"))(id) };
        if p.is_null() {
            return None;
        }
        let base = self.node_table_ptr() as usize;
        let off = (p as usize)
            .checked_sub(base)
            .unwrap_or_else(|| panic!("{}: find_node_by_id returned ptr below node_table", self.name));
        assert_eq!(
            off % TREE_NODE_SIZE,
            0,
            "{}: find_node_by_id returned misaligned ptr (offset {off})",
            self.name
        );
        Some(off / TREE_NODE_SIZE)
    }

    pub fn add_tree_node(&self, id: c_int, value: c_int, parent_id: c_int, label: &[u8]) -> c_int {
        assert_eq!(label.last(), Some(&0u8), "label must be NUL-terminated");
        unsafe { (self.f::<AddFn>("add_tree_node"))(id, value, parent_id, label.as_ptr().cast()) }
    }

    pub fn calculate_tree_sum(&self, node_id: c_int) -> c_int {
        unsafe { (self.f::<SumFn>("calculate_tree_sum"))(node_id) }
    }

    /// `label` may be a non-NUL-terminated slice; used only where the C would
    /// still stop inside 31 bytes.
    pub fn parse_operation(&self, s: &[u8]) -> c_int {
        assert_eq!(s.last(), Some(&0u8), "string must be NUL-terminated");
        unsafe { (self.f::<ParseFn>("parse_operation"))(s.as_ptr().cast()) }
    }

    pub fn parse_operation_null(&self) -> c_int {
        unsafe { (self.f::<ParseFn>("parse_operation"))(std::ptr::null()) }
    }

    /// Returns the raw dispatch pointer. Addresses are not comparable across
    /// `.so`s, so callers must probe it via [`Lib::call_op`].
    pub fn get_operation_func(&self, op: c_int) -> Op4 {
        unsafe { (self.f::<GetFuncFn>("get_operation_func"))(op) }
    }

    pub fn call_op(&self, f: Op4, a: c_int, b: c_int, u1: c_int, u2: c_int) -> c_int {
        unsafe { f(a, b, u1, u2) }
    }

    pub fn inreftree(&self, p1: c_int, p2: c_int, p3: c_int, p4: c_int) -> c_int {
        unsafe { (self.f::<Op4>("inreftree"))(p1, p2, p3, p4) }
    }

    // ---- exported data objects -------------------------------------------

    pub fn node_count_ptr(&self) -> *mut c_int {
        *self.f::<*mut c_int>("node_count")
    }
    pub fn node_table_ptr(&self) -> *mut TreeNode {
        *self.f::<*mut TreeNode>("node_table")
    }

    pub fn node_count(&self) -> c_int {
        unsafe { *self.node_count_ptr() }
    }
    pub fn set_node_count(&self, v: c_int) {
        unsafe { *self.node_count_ptr() = v }
    }

    /// The full 2600-byte `node_table` image, for byte-for-byte comparison.
    pub fn table_bytes(&self) -> Vec<u8> {
        unsafe {
            std::slice::from_raw_parts(
                self.node_table_ptr().cast::<u8>(),
                MAX_NODES * TREE_NODE_SIZE,
            )
            .to_vec()
        }
    }

    pub fn node(&self, i: usize) -> TreeNode {
        assert!(i < MAX_NODES);
        unsafe { *self.node_table_ptr().add(i) }
    }

    pub fn set_node(&self, i: usize, n: TreeNode) {
        assert!(i < MAX_NODES);
        unsafe { *self.node_table_ptr().add(i) = n }
    }

    /// Zero the whole table and `node_count`, so both libraries start from an
    /// identical, known state.
    pub fn reset(&self) {
        unsafe {
            std::ptr::write_bytes(
                self.node_table_ptr().cast::<u8>(),
                0,
                MAX_NODES * TREE_NODE_SIZE,
            );
        }
        self.set_node_count(0);
    }
}

/// The two libraries under differential test.
///
/// `dlopen` of the same path returns the same mapping process-wide, so both
/// libraries' `node_table` / `node_count` are shared by every test in the
/// binary. Holding a process-wide lock for the lifetime of a `Pair` serialises
/// the state-mutating tests instead of letting them race.
pub struct Pair {
    pub c: Lib,
    pub rs: Lib,
    _guard: MutexGuard<'static, ()>,
}

static STATE_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

fn workdir() -> PathBuf {
    // CARGO_MANIFEST_DIR = <work>/translation
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ has a parent")
        .to_path_buf()
}

fn find_c_so() -> PathBuf {
    let dir = workdir().join("c_src/build");
    let mut hits: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| {
            panic!(
                "cannot read {} ({e}); build the C library first:\n  cd c_src && mkdir -p build && cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
                dir.display()
            )
        })
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            p.extension().and_then(|s| s.to_str()) == Some("so")
                && p.file_name()
                    .and_then(|s| s.to_str())
                    .is_some_and(|s| s.starts_with("lib"))
        })
        .collect();
    hits.sort();
    assert_eq!(
        hits.len(),
        1,
        "expected exactly one C .so in {}, found {hits:?}",
        dir.display()
    );
    hits.pop().unwrap()
}

fn find_rust_so() -> PathBuf {
    let base = workdir().join("translation/target");
    for profile in ["release", "debug"] {
        let p = base.join(profile).join("libinreftree_lib.so");
        if p.exists() {
            return p;
        }
    }
    panic!(
        "libinreftree_lib.so not found under {}; run `cargo build --release` first",
        base.display()
    );
}

impl Pair {
    pub fn load() -> Pair {
        let guard = STATE_LOCK
            .get_or_init(|| Mutex::new(()))
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let p = Pair {
            c: Lib::open("C", find_c_so()),
            rs: Lib::open("Rust", find_rust_so()),
            _guard: guard,
        };
        p.reset_both();
        p
    }

    pub fn reset_both(&self) {
        self.c.reset();
        self.rs.reset();
    }

    /// Assert the two libraries' mutable state is byte-identical.
    pub fn assert_state_eq(&self, ctx: &str) {
        assert_eq!(
            self.c.node_count(),
            self.rs.node_count(),
            "{ctx}: node_count diverged (C={}, Rust={})",
            self.c.node_count(),
            self.rs.node_count()
        );
        let cb = self.c.table_bytes();
        let rb = self.rs.table_bytes();
        if cb != rb {
            let first = cb
                .iter()
                .zip(rb.iter())
                .position(|(a, b)| a != b)
                .expect("lengths equal");
            panic!(
                "{ctx}: node_table diverged at byte {first} (slot {}, offset {}): C=0x{:02x} Rust=0x{:02x}",
                first / TREE_NODE_SIZE,
                first % TREE_NODE_SIZE,
                cb[first],
                rb[first]
            );
        }
    }

    /// Compare a scalar result from both libraries.
    pub fn eq<T: PartialEq + std::fmt::Debug>(&self, ctx: &str, c: T, rs: T) {
        assert_eq!(c, rs, "{ctx}: C returned {c:?}, Rust returned {rs:?}");
    }
}

/// Deterministic xorshift64* PRNG — fixed seed, reproducible across runs.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(if seed == 0 { 0x9E3779B97F4A7C15 } else { seed })
    }
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545F4914F6CDD1D)
    }
    pub fn i32(&mut self) -> c_int {
        self.next_u64() as u32 as i32
    }
    /// Small magnitude, so sums stay in range and hit many distinct residues.
    pub fn small(&mut self) -> c_int {
        (self.next_u64() % 4001) as i32 - 2000
    }
    pub fn range(&mut self, lo: i64, hi: i64) -> i64 {
        assert!(hi > lo);
        lo + (self.next_u64() % ((hi - lo) as u64)) as i64
    }
    pub fn usize(&mut self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
    }
    pub fn byte(&mut self) -> u8 {
        self.next_u64() as u8
    }
    /// A byte that is never NUL and never one of `+ * - / %`.
    pub fn plain_byte(&mut self) -> u8 {
        loop {
            let b = self.byte();
            if b != 0 && !matches!(b, b'+' | b'*' | b'-' | b'/' | b'%') {
                return b;
            }
        }
    }
    /// Interesting `i32` corner values mixed with uniform draws.
    pub fn interesting_i32(&mut self) -> c_int {
        const CORNERS: [i32; 12] = [
            0, 1, -1, 2, -2, 3, -3, 4, -4, i32::MIN, i32::MAX, i32::MIN + 1,
        ];
        match self.next_u64() % 3 {
            0 => CORNERS[self.usize(CORNERS.len())],
            1 => self.small(),
            _ => self.i32(),
        }
    }
}

/// A NUL-terminated byte vector from a byte slice.
pub fn cstr(bytes: &[u8]) -> Vec<u8> {
    let mut v = bytes.to_vec();
    v.push(0);
    v
}
