// Differential-test harness.
//
// Loads BOTH shared objects through `libloading` and calls every function
// through its exported C symbol, so the `#[unsafe(no_mangle)] extern "C"`
// wrappers of the Rust crate are part of what is under test. Nothing is ever
// called directly against the Rust crate's Rust-level API.
//
// Each `Pair::fresh()` copies both `.so` files to a unique temporary path
// before `dlopen`, which gives every test-case a *pristine* copy of the
// libraries' `static` state (`node_count == 0`, `node_storage` all zero) --
// `dlopen` of a distinct file yields a distinct mapping with its own BSS.

#![allow(dead_code)]

use std::ffi::c_char;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

pub const MAX_NODES: usize = 100;
pub const MAX_NAME_LEN: usize = 50;
pub const NODE_SIZE: usize = 80;

/// Mirrors the (file-private) C `Node` struct.
/// Verified against gcc: `sizeof == 80`, `alignof == 8`,
/// offsets `id=0 parent_id=4 name=8 value=64 active=72`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Node {
    pub id: i32,
    pub parent_id: i32,
    pub name: [c_char; MAX_NAME_LEN],
    pub value: f64,
    pub active: i32,
}

/// Field-wise snapshot of a `Node`, in a form that is cheap to compare and
/// print. `value` is kept as raw bits so NaN payloads/signs are compared too.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct NodeSnap {
    pub id: i32,
    pub parent_id: i32,
    pub name: [u8; MAX_NAME_LEN],
    pub value_bits: u64,
    pub active: i32,
}

impl NodeSnap {
    pub unsafe fn read(p: *const Node) -> NodeSnap {
        let n = unsafe { &*p };
        let mut name = [0u8; MAX_NAME_LEN];
        for i in 0..MAX_NAME_LEN {
            name[i] = n.name[i] as u8;
        }
        NodeSnap {
            id: n.id,
            parent_id: n.parent_id,
            name,
            value_bits: n.value.to_bits(),
            active: n.active,
        }
    }
}

pub type FnAddNode = unsafe extern "C" fn(i32, i32, *const c_char, f64) -> i32;
pub type FnFindNode = unsafe extern "C" fn(i32) -> *mut Node;
pub type FnChildren = unsafe extern "C" fn(i32) -> i32;
pub type FnSubtree = unsafe extern "C" fn(i32) -> f64;
pub type FnProcessString = unsafe extern "C" fn(*mut c_char) -> i32;
pub type FnSafeD2I = unsafe extern "C" fn(f64) -> i32;
pub type FnMaxnmin = unsafe extern "C" fn(i32, i32, i32, i32) -> i32;

/// One loaded shared object with all 7 exported entry points resolved.
pub struct Lib {
    pub tag: &'static str,
    pub add_node: FnAddNode,
    pub find_node_by_id: FnFindNode,
    pub get_children_count: FnChildren,
    pub calculate_subtree_sum: FnSubtree,
    pub process_string: FnProcessString,
    pub safe_double_to_int: FnSafeD2I,
    pub maxnmin: FnMaxnmin,
    lib: Option<libloading::Library>,
    tmp: PathBuf,
}

impl Lib {
    fn load(tag: &'static str, original: &Path, tmp: PathBuf) -> Lib {
        fs::copy(original, &tmp)
            .unwrap_or_else(|e| panic!("copy {} -> {}: {e}", original.display(), tmp.display()));
        let lib = unsafe { libloading::Library::new(&tmp) }
            .unwrap_or_else(|e| panic!("dlopen {}: {e}", tmp.display()));
        macro_rules! sym {
            ($name:literal, $ty:ty) => {{
                let s: libloading::Symbol<$ty> = unsafe { lib.get($name) }
                    .unwrap_or_else(|e| panic!("{tag}: missing symbol {}: {e}", $name.escape_ascii()));
                *s
            }};
        }
        let add_node = sym!(b"add_node\0", FnAddNode);
        let find_node_by_id = sym!(b"find_node_by_id\0", FnFindNode);
        let get_children_count = sym!(b"get_children_count\0", FnChildren);
        let calculate_subtree_sum = sym!(b"calculate_subtree_sum\0", FnSubtree);
        let process_string = sym!(b"process_string\0", FnProcessString);
        let safe_double_to_int = sym!(b"safe_double_to_int\0", FnSafeD2I);
        let maxnmin = sym!(b"maxnmin\0", FnMaxnmin);
        Lib {
            tag,
            add_node,
            find_node_by_id,
            get_children_count,
            calculate_subtree_sum,
            process_string,
            safe_double_to_int,
            maxnmin,
            lib: Some(lib),
            tmp,
        }
    }
}

impl Drop for Lib {
    fn drop(&mut self) {
        drop(self.lib.take());
        let _ = fs::remove_file(&self.tmp);
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

pub fn find_c_so() -> PathBuf {
    if let Ok(p) = std::env::var("HARVEST_C_SO") {
        return PathBuf::from(p);
    }
    let build = manifest_dir().join("../c_src/build");
    let mut found: Vec<PathBuf> = Vec::new();
    if let Ok(rd) = fs::read_dir(&build) {
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().map(|x| x == "so").unwrap_or(false) {
                found.push(p);
            }
        }
    }
    found.sort();
    match found.len() {
        0 => panic!(
            "no C .so found in {} -- build it with:\n  cd c_src && mkdir -p build && cd build \
             && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            build.display()
        ),
        _ => found.remove(0),
    }
}

pub fn find_rust_so() -> PathBuf {
    if let Ok(p) = std::env::var("HARVEST_RUST_SO") {
        return PathBuf::from(p);
    }
    let base = manifest_dir().join("target");
    for prof in ["release", "debug"] {
        let p = base.join(prof).join("libmaxnmin_lib.so");
        if p.exists() {
            return p;
        }
    }
    panic!(
        "no Rust cdylib found under {} -- build it with `cargo build --release`",
        base.display()
    );
}

fn tmp_root() -> PathBuf {
    let d = std::env::temp_dir().join(format!("harvest_diff_{}", std::process::id()));
    fs::create_dir_all(&d).unwrap_or_else(|e| panic!("mkdir {}: {e}", d.display()));
    d
}

static COUNTER: AtomicU64 = AtomicU64::new(0);

/// A freshly loaded (C, Rust) library pair with pristine static state.
pub struct Pair {
    pub c: Lib,
    pub rust: Lib,
}

impl Pair {
    pub fn fresh() -> Pair {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let root = tmp_root();
        let c = Lib::load(
            "C",
            &find_c_so(),
            root.join(format!("c_{n}_{:?}.so", std::thread::current().id())),
        );
        let rust = Lib::load(
            "Rust",
            &find_rust_so(),
            root.join(format!("r_{n}_{:?}.so", std::thread::current().id())),
        );
        Pair { c, rust }
    }
}

// ---------------------------------------------------------------------------
// deterministic RNG (xorshift64* -- fixed seed per row, reproducible)
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed ^ 0x9E37_79B9_7F4A_7C15)
    }
    pub fn next_u64(&mut self) -> u64 {
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
    pub fn next_i32(&mut self) -> i32 {
        self.next_u32() as i32
    }
    /// uniform in `0..n`
    pub fn below(&mut self, n: u64) -> u64 {
        assert!(n > 0);
        self.next_u64() % n
    }
    pub fn range_i32(&mut self, lo: i32, hi: i32) -> i32 {
        // inclusive lo..=hi
        let span = (hi as i64 - lo as i64 + 1) as u64;
        (lo as i64 + self.below(span) as i64) as i32
    }
    /// raw bit pattern reinterpreted as f64 (any class incl. weird NaNs)
    pub fn next_f64_bits(&mut self) -> f64 {
        f64::from_bits(self.next_u64())
    }
    /// uniform in [0,1)
    pub fn unit(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }
    pub fn nonzero_byte(&mut self) -> u8 {
        let b = (self.next_u64() >> 24) as u8;
        if b == 0 { 1 } else { b }
    }
    pub fn bool(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }
}

// ---------------------------------------------------------------------------
// comparison helpers
// ---------------------------------------------------------------------------

#[track_caller]
pub fn eq_i32(row: &str, ctx: impl std::fmt::Display, c: i32, r: i32) {
    assert_eq!(c, r, "[{row}] int mismatch: C={c} Rust={r} :: {ctx}");
}

#[track_caller]
pub fn eq_f64(row: &str, ctx: impl std::fmt::Display, c: f64, r: f64) {
    assert_eq!(
        c.to_bits(),
        r.to_bits(),
        "[{row}] double mismatch: C={c:?} (0x{:016x}) Rust={r:?} (0x{:016x}) :: {ctx}",
        c.to_bits(),
        r.to_bits()
    );
}

#[track_caller]
pub fn eq_nullness(row: &str, ctx: impl std::fmt::Display, c: *const Node, r: *const Node) {
    assert_eq!(
        c.is_null(),
        r.is_null(),
        "[{row}] pointer nullness mismatch: C_null={} Rust_null={} :: {ctx}",
        c.is_null(),
        r.is_null()
    );
}

/// Compares two `Node *` results: same nullness and, when non-null, identical
/// contents. `Node *` values themselves live in different address spaces, so
/// identity is compared through *deltas* (see `slot_delta`).
#[track_caller]
pub fn eq_node_ptr(row: &str, ctx: impl std::fmt::Display, c: *mut Node, r: *mut Node) {
    eq_nullness(row, &ctx, c, r);
    if !c.is_null() {
        let cs = unsafe { NodeSnap::read(c) };
        let rs = unsafe { NodeSnap::read(r) };
        assert_eq!(
            cs, rs,
            "[{row}] Node contents mismatch:\n  C   ={cs:?}\n  Rust={rs:?}\n  :: {ctx}"
        );
    }
}

/// Slot index of `p` relative to `base`, in `Node` units. Both pointers must
/// come from the *same* library. Returns `None` if either is null.
pub fn slot_delta(base: *const Node, p: *const Node) -> Option<isize> {
    if base.is_null() || p.is_null() {
        return None;
    }
    let d = (p as isize) - (base as isize);
    assert_eq!(d % NODE_SIZE as isize, 0, "pointer delta not a Node multiple");
    Some(d / NODE_SIZE as isize)
}

/// NUL-terminated byte buffer usable as `char *` / `const char *`.
pub struct CBuf(Vec<u8>);

impl CBuf {
    pub fn new(bytes: &[u8]) -> CBuf {
        let mut v = bytes.to_vec();
        v.push(0);
        CBuf(v)
    }
    pub fn from_str(s: &str) -> CBuf {
        CBuf::new(s.as_bytes())
    }
    /// Buffer with no terminator appended (caller guarantees one is present).
    pub fn raw(bytes: &[u8]) -> CBuf {
        CBuf(bytes.to_vec())
    }
    pub fn ptr(&self) -> *const c_char {
        self.0.as_ptr() as *const c_char
    }
    pub fn ptr_mut(&mut self) -> *mut c_char {
        self.0.as_mut_ptr() as *mut c_char
    }
}

/// The six nodes `maxnmin` always (re)seeds, in insertion order.
pub const BUILTINS: [(i32, i32, &str, f64); 6] = [
    (1, -1, "root", 10.5),
    (2, 1, "child1", 20.7),
    (3, 1, "child2", 15.3),
    (4, 2, "grandchild1", 5.9),
    (5, 2, "grandchild2", 8.2),
    (6, 3, "grandchild3", 12.4),
];

/// Applies the same `add_node` call to both libraries and asserts the returned
/// index matches.
#[track_caller]
pub fn both_add(
    p: &Pair,
    row: &str,
    id: i32,
    parent_id: i32,
    name: &[u8],
    value: f64,
) -> i32 {
    let buf = CBuf::new(name);
    let rc = unsafe { (p.c.add_node)(id, parent_id, buf.ptr(), value) };
    let rr = unsafe { (p.rust.add_node)(id, parent_id, buf.ptr(), value) };
    eq_i32(
        row,
        format!(
            "add_node(id={id}, parent={parent_id}, name={:?}, value_bits=0x{:016x})",
            String::from_utf8_lossy(name),
            value.to_bits()
        ),
        rc,
        rr,
    );
    rc
}

/// `find_node_by_id` on both libraries; returns the two pointers.
#[track_caller]
pub fn both_find(p: &Pair, row: &str, id: i32) -> (*mut Node, *mut Node) {
    let fc = unsafe { (p.c.find_node_by_id)(id) };
    let fr = unsafe { (p.rust.find_node_by_id)(id) };
    eq_node_ptr(row, format!("find_node_by_id({id})"), fc, fr);
    (fc, fr)
}

/// `get_children_count` on both libraries.
#[track_caller]
pub fn both_children(p: &Pair, row: &str, id: i32) -> i32 {
    let cc = unsafe { (p.c.get_children_count)(id) };
    let cr = unsafe { (p.rust.get_children_count)(id) };
    eq_i32(row, format!("get_children_count({id})"), cc, cr);
    cc
}

/// `calculate_subtree_sum` on both libraries.
///
/// NOTE: the C recursion (line 91) has no visited set, so this must only be
/// called on acyclic child relations -- see `ERRORS.md` row E39.
#[track_caller]
pub fn both_subtree(p: &Pair, row: &str, id: i32) -> f64 {
    let sc = unsafe { (p.c.calculate_subtree_sum)(id) };
    let sr = unsafe { (p.rust.calculate_subtree_sum)(id) };
    eq_f64(row, format!("calculate_subtree_sum({id})"), sc, sr);
    sc
}

/// `find_node_by_id` + `get_children_count` (safe on any state, incl. cyclic).
#[track_caller]
pub fn both_query_nosum(p: &Pair, row: &str, id: i32) {
    both_find(p, row, id);
    both_children(p, row, id);
}

/// Runs all three read-only query entry points for `id` on both libraries and
/// asserts every result matches. Requires an acyclic child relation.
#[track_caller]
pub fn both_query(p: &Pair, row: &str, id: i32) {
    both_find(p, row, id);
    both_children(p, row, id);
    both_subtree(p, row, id);
}

/// Applies the same in-place mutation to the `Node` that `find_node_by_id(id)`
/// returns in each library (the C API hands out a mutable `Node *`, so this is
/// legitimate consumer behaviour). Asserts the lookups agreed first.
#[track_caller]
pub fn both_mutate(p: &Pair, row: &str, id: i32, f: &dyn Fn(*mut Node)) -> bool {
    let (fc, fr) = both_find(p, row, id);
    if fc.is_null() {
        return false;
    }
    f(fc);
    f(fr);
    true
}

/// Slot index (relative to `anchor_id`'s slot) of `id`'s slot, compared between
/// the two libraries. This is how `Node *` identity is compared across the two
/// distinct address spaces.
#[track_caller]
pub fn both_delta(p: &Pair, row: &str, anchor_id: i32, id: i32) {
    let (ac, ar) = both_find(p, row, anchor_id);
    let (fc, fr) = both_find(p, row, id);
    let dc = slot_delta(ac, fc);
    let dr = slot_delta(ar, fr);
    assert_eq!(
        dc, dr,
        "[{row}] slot delta mismatch for id={id} (anchor={anchor_id}): C={dc:?} Rust={dr:?}"
    );
}

/// `safe_double_to_int` on both libraries.
#[track_caller]
pub fn both_d2i(p: &Pair, row: &str, d: f64) {
    let c = unsafe { (p.c.safe_double_to_int)(d) };
    let r = unsafe { (p.rust.safe_double_to_int)(d) };
    eq_i32(
        row,
        format!("safe_double_to_int({d:?} bits=0x{:016x})", d.to_bits()),
        c,
        r,
    );
}

/// `process_string` on both libraries (same buffer contents, separate copies so
/// neither library can observe the other's writes -- the C signature is
/// non-const `char *`).
#[track_caller]
pub fn both_process(p: &Pair, row: &str, bytes: &[u8]) {
    let mut b1 = CBuf::raw(bytes.to_vec().as_slice());
    let mut b2 = CBuf::raw(bytes.to_vec().as_slice());
    let c = unsafe { (p.c.process_string)(b1.ptr_mut()) };
    let r = unsafe { (p.rust.process_string)(b2.ptr_mut()) };
    eq_i32(
        row,
        format!("process_string(len={}, head={:?})", bytes.len(), &bytes[..bytes.len().min(16)]),
        c,
        r,
    );
}

/// `maxnmin` on both libraries.
#[track_caller]
pub fn both_maxnmin(p: &Pair, row: &str, a: i32, b: i32, c_: i32, d: i32) -> i32 {
    let rc = unsafe { (p.c.maxnmin)(a, b, c_, d) };
    let rr = unsafe { (p.rust.maxnmin)(a, b, c_, d) };
    eq_i32(row, format!("maxnmin({a}, {b}, {c_}, {d})"), rc, rr);
    rc
}
