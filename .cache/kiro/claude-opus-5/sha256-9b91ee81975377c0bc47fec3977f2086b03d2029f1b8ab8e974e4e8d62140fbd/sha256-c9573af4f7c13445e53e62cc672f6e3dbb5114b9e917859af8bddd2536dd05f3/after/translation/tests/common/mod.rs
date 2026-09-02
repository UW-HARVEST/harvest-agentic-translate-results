//! Shared differential-test harness.
//!
//! Loads BOTH shared objects with `libloading` and exposes one `Lib` handle per
//! implementation. The Rust side is ALWAYS reached through
//! `target/release/libinreftree_lib.so`, never by calling crate functions
//! directly, so the `#[no_mangle]` export wrappers are part of what is tested.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::os::raw::{c_char, c_int};
use std::path::PathBuf;

pub const MAX_NODES: usize = 50;
/// `5 * sizeof(int) + 32` — verified against the C `.so`'s `node_table` span.
pub const TREE_NODE_SIZE: usize = 52;
pub const NODE_TABLE_BYTES: usize = MAX_NODES * TREE_NODE_SIZE;

#[repr(C)]
#[derive(Copy, Clone, PartialEq, Eq)]
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
        write!(
            f,
            "TreeNode {{ id: {}, value: {}, parent: {}, l: {}, r: {}, label: {:?} }}",
            self.id, self.value, self.parent_id, self.left_child_id, self.right_child_id, raw
        )
    }
}

pub type OpFn = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

// ---------------------------------------------------------------------------
// Locating the two shared objects
// ---------------------------------------------------------------------------

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

pub fn c_so_path() -> PathBuf {
    let build = crate_root().join("../c_src/build");
    let mut found = None;
    if let Ok(rd) = std::fs::read_dir(&build) {
        for e in rd.flatten() {
            let p = e.path();
            let name = p.file_name().unwrap_or_default().to_string_lossy().to_string();
            if name.starts_with("lib") && name.ends_with(".so") {
                found = Some(p);
                break;
            }
        }
    }
    found.unwrap_or_else(|| {
        panic!(
            "C shared library not found in {}. Build it with:\n  cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            build.display()
        )
    })
}

pub fn rust_so_path() -> PathBuf {
    // Allow pinning a specific artifact (used to verify the debug-profile
    // cdylib as well as the release one).
    if let Ok(p) = std::env::var("TRANSLATION_SO") {
        let p = PathBuf::from(p);
        assert!(p.exists(), "TRANSLATION_SO={} does not exist", p.display());
        return p;
    }
    // `cargo test` puts the test binary in target/<profile>/deps; the cdylib
    // sits in target/<profile>. Try the release build first (that is what the
    // task builds), then fall back to whatever profile is in use.
    let root = crate_root();
    for prof in ["release", "debug"] {
        let p = root.join("target").join(prof).join("libinreftree_lib.so");
        if p.exists() {
            return p;
        }
    }
    panic!("Rust cdylib not found. Build it with: cd translation && cargo build --release");
}

// ---------------------------------------------------------------------------
// Library handle
// ---------------------------------------------------------------------------

pub struct Lib {
    pub name: &'static str,
    lib: Library,
}

impl Lib {
    pub fn open(name: &'static str, path: &std::path::Path) -> Lib {
        // RTLD_LOCAL (libloading's default) keeps the two libraries' identically
        // named symbols from colliding, so each handle really resolves into its
        // own .so.
        let lib = unsafe { Library::new(path) }
            .unwrap_or_else(|e| panic!("failed to dlopen {} ({}): {e}", name, path.display()));
        Lib { name, lib }
    }

    fn sym<T>(&self, s: &str) -> Symbol<'_, T> {
        unsafe { self.lib.get(s.as_bytes()) }
            .unwrap_or_else(|e| panic!("{}: missing symbol `{s}`: {e}", self.name))
    }

    // -- the five arithmetic ops ------------------------------------------

    pub fn add_op(&self, a: c_int, b: c_int, u1: c_int, u2: c_int) -> c_int {
        let f: Symbol<OpFn> = self.sym("add_op");
        unsafe { f(a, b, u1, u2) }
    }
    pub fn multiply_op(&self, a: c_int, b: c_int, u1: c_int, u2: c_int) -> c_int {
        let f: Symbol<OpFn> = self.sym("multiply_op");
        unsafe { f(a, b, u1, u2) }
    }
    pub fn subtract_op(&self, a: c_int, b: c_int, u1: c_int, u2: c_int) -> c_int {
        let f: Symbol<OpFn> = self.sym("subtract_op");
        unsafe { f(a, b, u1, u2) }
    }
    pub fn divide_op(&self, a: c_int, b: c_int, u1: c_int, u2: c_int) -> c_int {
        let f: Symbol<OpFn> = self.sym("divide_op");
        unsafe { f(a, b, u1, u2) }
    }
    pub fn modulo_op(&self, a: c_int, b: c_int, u1: c_int, u2: c_int) -> c_int {
        let f: Symbol<OpFn> = self.sym("modulo_op");
        unsafe { f(a, b, u1, u2) }
    }

    /// Dispatch by name so the fuzz driver can pick an op at random.
    pub fn op_by_name(&self, n: &str, a: c_int, b: c_int) -> c_int {
        match n {
            "add_op" => self.add_op(a, b, 0, 0),
            "multiply_op" => self.multiply_op(a, b, 0, 0),
            "subtract_op" => self.subtract_op(a, b, 0, 0),
            "divide_op" => self.divide_op(a, b, 0, 0),
            "modulo_op" => self.modulo_op(a, b, 0, 0),
            other => panic!("unknown op {other}"),
        }
    }

    // -- tree table --------------------------------------------------------

    /// Returns the matched node's *index* into `node_table`, or `None` for NULL.
    ///
    /// Raw pointers cannot be compared across libraries (different load
    /// addresses), so the pointer is normalised to an offset from the library's
    /// own `node_table` base. That still detects "returned the wrong entry".
    pub fn find_node_by_id(&self, id: c_int) -> Option<isize> {
        let f: Symbol<unsafe extern "C" fn(c_int) -> *mut u8> = self.sym("find_node_by_id");
        let p = unsafe { f(id) };
        if p.is_null() {
            None
        } else {
            let base = self.node_table_ptr();
            let delta = (p as isize) - (base as isize);
            assert_eq!(
                delta % TREE_NODE_SIZE as isize,
                0,
                "{}: find_node_by_id returned a misaligned pointer (delta {delta})",
                self.name
            );
            Some(delta / TREE_NODE_SIZE as isize)
        }
    }

    pub fn add_tree_node(&self, id: c_int, value: c_int, parent_id: c_int, label: &[u8]) -> c_int {
        assert_eq!(label.last(), Some(&0), "label must be NUL-terminated");
        let f: Symbol<unsafe extern "C" fn(c_int, c_int, c_int, *const c_char) -> c_int> =
            self.sym("add_tree_node");
        unsafe { f(id, value, parent_id, label.as_ptr().cast()) }
    }

    /// `add_tree_node` with an explicitly NULL label (error row 10).
    pub fn add_tree_node_null_label(&self, id: c_int, value: c_int, parent_id: c_int) -> c_int {
        let f: Symbol<unsafe extern "C" fn(c_int, c_int, c_int, *const c_char) -> c_int> =
            self.sym("add_tree_node");
        unsafe { f(id, value, parent_id, std::ptr::null()) }
    }

    pub fn calculate_tree_sum(&self, node_id: c_int) -> c_int {
        let f: Symbol<unsafe extern "C" fn(c_int) -> c_int> = self.sym("calculate_tree_sum");
        unsafe { f(node_id) }
    }

    // -- operation dispatch ------------------------------------------------

    pub fn parse_operation(&self, s: &[u8]) -> c_int {
        assert_eq!(s.last(), Some(&0), "string must be NUL-terminated");
        let f: Symbol<unsafe extern "C" fn(*const c_char) -> c_int> = self.sym("parse_operation");
        unsafe { f(s.as_ptr().cast()) }
    }

    pub fn parse_operation_null(&self) -> c_int {
        let f: Symbol<unsafe extern "C" fn(*const c_char) -> c_int> = self.sym("parse_operation");
        unsafe { f(std::ptr::null()) }
    }

    /// `get_operation_func` returns a function pointer whose *address* is
    /// library-specific. It is identified behaviourally instead: called with a
    /// discriminating operand pair, the five ops give five distinct answers.
    pub fn get_operation_func_probe(&self, op: c_int, a: c_int, b: c_int) -> c_int {
        let f: Symbol<unsafe extern "C" fn(c_int) -> OpFn> = self.sym("get_operation_func");
        unsafe {
            let g = f(op);
            g(a, b, 0, 0)
        }
    }

    /// Which of the five exported `*_op` symbols in THIS library the returned
    /// pointer equals. Compares real addresses, but only within one `.so`.
    pub fn get_operation_func_identity(&self, op: c_int) -> &'static str {
        let f: Symbol<unsafe extern "C" fn(c_int) -> OpFn> = self.sym("get_operation_func");
        let got = unsafe { f(op) } as usize;
        for name in ["add_op", "multiply_op", "subtract_op", "divide_op", "modulo_op"] {
            let s: Symbol<OpFn> = self.sym(name);
            if (*s as usize) == got {
                return match name {
                    "add_op" => "add_op",
                    "multiply_op" => "multiply_op",
                    "subtract_op" => "subtract_op",
                    "divide_op" => "divide_op",
                    _ => "modulo_op",
                };
            }
        }
        "<unknown>"
    }

    // -- entry point -------------------------------------------------------

    pub fn inreftree(&self, p1: c_int, p2: c_int, p3: c_int, p4: c_int) -> c_int {
        let f: Symbol<unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int> =
            self.sym("inreftree");
        unsafe { f(p1, p2, p3, p4) }
    }

    // -- exported mutable state -------------------------------------------

    pub fn node_count_ptr(&self) -> *mut c_int {
        let s: Symbol<*mut c_int> = self.sym("node_count");
        *s
    }

    pub fn node_table_ptr(&self) -> *mut u8 {
        let s: Symbol<*mut u8> = self.sym("node_table");
        *s
    }

    pub fn get_node_count(&self) -> c_int {
        unsafe { std::ptr::read_volatile(self.node_count_ptr()) }
    }

    pub fn set_node_count(&self, v: c_int) {
        unsafe { std::ptr::write_volatile(self.node_count_ptr(), v) }
    }

    /// Full 2600-byte image of `node_table`, including padding and stale bytes.
    pub fn node_table_image(&self) -> Vec<u8> {
        unsafe { std::slice::from_raw_parts(self.node_table_ptr(), NODE_TABLE_BYTES).to_vec() }
    }

    pub fn set_node_table_image(&self, bytes: &[u8]) {
        assert_eq!(bytes.len(), NODE_TABLE_BYTES);
        unsafe {
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), self.node_table_ptr(), NODE_TABLE_BYTES)
        }
    }

    pub fn node(&self, i: usize) -> TreeNode {
        assert!(i < MAX_NODES);
        unsafe { std::ptr::read_unaligned(self.node_table_ptr().add(i * TREE_NODE_SIZE).cast()) }
    }

    pub fn set_node(&self, i: usize, n: &TreeNode) {
        assert!(i < MAX_NODES);
        unsafe {
            std::ptr::write_unaligned(self.node_table_ptr().add(i * TREE_NODE_SIZE).cast(), *n)
        }
    }

    // -- pre-resolved raw pointers (for use across fork(), where calling into
    //    the dynamic loader is not safe) -----------------------------------

    pub fn raw_op(&self, name: &str) -> OpFn {
        let s: Symbol<OpFn> = self.sym(name);
        *s
    }

    pub fn raw_add_tree_node(
        &self,
    ) -> unsafe extern "C" fn(c_int, c_int, c_int, *const c_char) -> c_int {
        let s: Symbol<unsafe extern "C" fn(c_int, c_int, c_int, *const c_char) -> c_int> =
            self.sym("add_tree_node");
        *s
    }

    /// Mirrors `calculate_tree_sum`'s traversal over a snapshot of this
    /// library's table, with a step budget.
    ///
    /// A `left_child_id` / `right_child_id` that resolves back to an ancestor
    /// makes the C function recurse until the stack is exhausted — and the Rust
    /// translation does exactly the same, so such an input is a property of the
    /// data, not a divergence, and cannot be compared in-process. Tests use this
    /// to skip those inputs.
    pub fn sum_terminates(&self, id: c_int) -> bool {
        let count = (self.get_node_count().max(0) as usize).min(MAX_NODES);
        let nodes: Vec<TreeNode> = (0..count).map(|i| self.node(i)).collect();
        fn walk(nodes: &[TreeNode], id: c_int, budget: &mut u32) -> bool {
            if *budget == 0 {
                return false;
            }
            *budget -= 1;
            let Some(n) = nodes.iter().find(|n| n.id == id) else {
                return true;
            };
            if n.left_child_id != -1 && !walk(nodes, n.left_child_id, budget) {
                return false;
            }
            if n.right_child_id != -1 && !walk(nodes, n.right_child_id, budget) {
                return false;
            }
            true
        }
        let mut budget = 2_000u32;
        walk(&nodes, id, &mut budget)
    }

    /// Zero the whole table and reset the count — the deterministic start state
    /// both libraries must be put in before a comparison.
    pub fn reset(&self) {
        self.set_node_table_image(&[0u8; NODE_TABLE_BYTES]);
        self.set_node_count(0);
    }
}

// ---------------------------------------------------------------------------
// The pair under test
// ---------------------------------------------------------------------------

struct Libs {
    c: Lib,
    r: Lib,
}

// `libloading::Library` is Send + Sync; the shared mutable state inside the
// loaded objects is what needs protecting, and `TEST_LOCK` does that.
static LIBS: std::sync::OnceLock<Libs> = std::sync::OnceLock::new();

/// `dlopen`ing the same path twice returns the SAME mapping, so both libraries'
/// `node_table` / `node_count` are process-global. `cargo test` runs tests on
/// several threads, so every test must hold this lock for its whole body or the
/// tests would corrupt each other's state (and produce bogus "divergences").
static TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

pub struct Pair {
    _guard: std::sync::MutexGuard<'static, ()>,
    pub c: &'static Lib,
    pub r: &'static Lib,
}

impl Pair {
    pub fn open() -> Pair {
        // Poisoning is expected: a failing differential test panics while
        // holding the lock. The state is re-established by `reset_both`, so the
        // lock is recovered rather than propagated.
        let guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let libs = LIBS.get_or_init(|| Libs {
            c: Lib::open("C", &c_so_path()),
            r: Lib::open("Rust", &rust_so_path()),
        });
        Pair {
            _guard: guard,
            c: &libs.c,
            r: &libs.r,
        }
    }

    /// Put both libraries in the same zeroed start state.
    pub fn reset_both(&self) {
        self.c.reset();
        self.r.reset();
    }

    /// Fill both `node_table`s with the same NON-ZERO pattern and set the same
    /// `node_count`.
    ///
    /// Starting from zeroed memory hides any field the implementation forgets to
    /// write (a zero left behind looks the same as a zero written). Poisoning
    /// first makes every missing store visible in the image comparison.
    pub fn poison_both(&self, rng: &mut Rng, count: c_int) {
        let mut img = vec![0u8; NODE_TABLE_BYTES];
        for b in img.iter_mut() {
            // never 0, so any byte the library fails to write stays detectable
            *b = (rng.next_u32() as u8) | 0x81;
        }
        self.c.set_node_table_image(&img);
        self.r.set_node_table_image(&img);
        self.c.set_node_count(count);
        self.r.set_node_count(count);
        self.assert_state_eq("poison_both");
    }

    /// Assert `node_count` and the whole `node_table` image agree.
    pub fn assert_state_eq(&self, ctx: &str) {
        assert_eq!(
            self.c.get_node_count(),
            self.r.get_node_count(),
            "{ctx}: node_count diverged"
        );
        let cc = self.c.node_table_image();
        let rr = self.r.node_table_image();
        if cc != rr {
            for i in 0..MAX_NODES {
                let a = &cc[i * TREE_NODE_SIZE..(i + 1) * TREE_NODE_SIZE];
                let b = &rr[i * TREE_NODE_SIZE..(i + 1) * TREE_NODE_SIZE];
                if a != b {
                    panic!(
                        "{ctx}: node_table[{i}] diverged\n  C   : {a:?}\n  Rust: {b:?}\n  C node   = {:?}\n  R node   = {:?}",
                        self.c.node(i),
                        self.r.node(i)
                    );
                }
            }
            panic!("{ctx}: node_table images differ but no single entry did (impossible)");
        }
    }
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (fixed seed -> reproducible property-style testing)
// ---------------------------------------------------------------------------

pub const SEED: u64 = 0x5EED_1234_ABCD_F00D;

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(if seed == 0 { 1 } else { seed })
    }
    pub fn next_u64(&mut self) -> u64 {
        // xorshift64*
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    pub fn next_i32(&mut self) -> c_int {
        self.next_u32() as c_int
    }
    /// Biased toward small magnitudes and boundary values, which is where the
    /// interesting branches live, while still covering the full range.
    pub fn next_i32_mixed(&mut self) -> c_int {
        match self.next_u32() % 8 {
            0 => 0,
            1 => i32::MIN,
            2 => i32::MAX,
            3 => -1,
            4 => (self.next_u32() % 17) as i32 - 8,
            5 => (self.next_u32() % 2001) as i32 - 1000,
            _ => self.next_i32(),
        }
    }
    pub fn below(&mut self, n: u32) -> u32 {
        if n == 0 { 0 } else { self.next_u32() % n }
    }
}

/// Boundary grid used by the `*_op` rows.
pub const EDGE_I32: [c_int; 13] = [
    i32::MIN,
    i32::MIN + 1,
    -65536,
    -257,
    -2,
    -1,
    0,
    1,
    2,
    257,
    65536,
    i32::MAX - 1,
    i32::MAX,
];

/// Helper: NUL-terminate a byte slice.
pub fn cstr(s: &[u8]) -> Vec<u8> {
    let mut v = s.to_vec();
    v.push(0);
    v
}
