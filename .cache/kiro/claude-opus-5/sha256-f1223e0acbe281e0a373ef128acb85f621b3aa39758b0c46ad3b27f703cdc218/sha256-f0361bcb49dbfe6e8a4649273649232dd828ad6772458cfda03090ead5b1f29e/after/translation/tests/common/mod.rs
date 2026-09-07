// Shared differential-test harness.
//
// Both the C `.so` and the Rust `.so` are loaded through `libloading` and driven
// ONLY through their exported C symbols -- no Rust function is ever called
// directly, so the `#[unsafe(no_mangle)] extern "C"` wrappers are under test too.
//
// The library keeps process-wide `static` state (`node_storage` / `node_count`).
// `dlopen` de-duplicates by inode, so to obtain a *pristine* (`node_count == 0`)
// instance for each test we copy each `.so` to a unique temporary path first.
// That gives every test its own independent copy of the statics on both sides.

#![allow(dead_code)]

use std::ffi::{c_char, c_double, c_int};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

pub const MAX_NODES: usize = 100;
pub const MAX_NAME_LEN: usize = 50;

/// Mirror of the C `Node` struct. Only used to *read* memory handed back by
/// `find_node_by_id`; layout parity is itself asserted by `C38`.
#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct Node {
    pub id: c_int,
    pub parent_id: c_int,
    pub name: [c_char; MAX_NAME_LEN],
    pub value: c_double,
    pub active: c_int,
}

/// A plain-data snapshot of a `Node`, comparable across the two libraries.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct NodeSnap {
    pub id: i32,
    pub parent_id: i32,
    pub name: Vec<u8>, // all 50 bytes, verbatim
    pub value_bits: u64,
    pub active: i32,
}

type FnAddNode = unsafe extern "C" fn(c_int, c_int, *const c_char, c_double) -> c_int;
type FnFindNode = unsafe extern "C" fn(c_int) -> *mut Node;
type FnChildren = unsafe extern "C" fn(c_int) -> c_int;
type FnSubtree = unsafe extern "C" fn(c_int) -> c_double;
type FnProcessString = unsafe extern "C" fn(*mut c_char) -> c_int;
type FnSafeD2I = unsafe extern "C" fn(c_double) -> c_int;
type FnMaxnmin = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

pub struct Lib {
    pub tag: &'static str,
    _lib: libloading::Library,
    pub add_node: FnAddNode,
    pub find_node_by_id: FnFindNode,
    pub get_children_count: FnChildren,
    pub calculate_subtree_sum: FnSubtree,
    pub process_string: FnProcessString,
    pub safe_double_to_int: FnSafeD2I,
    pub maxnmin: FnMaxnmin,
}

impl Lib {
    fn load(tag: &'static str, path: &Path) -> Lib {
        unsafe {
            let lib = libloading::Library::new(path)
                .unwrap_or_else(|e| panic!("dlopen {} ({}) failed: {e}", path.display(), tag));
            macro_rules! sym {
                ($t:ty, $n:literal) => {
                    *lib.get::<$t>($n).unwrap_or_else(|e| {
                        panic!("{} missing symbol {:?}: {e}", tag, String::from_utf8_lossy($n))
                    })
                };
            }
            Lib {
                tag,
                add_node: sym!(FnAddNode, b"add_node\0"),
                find_node_by_id: sym!(FnFindNode, b"find_node_by_id\0"),
                get_children_count: sym!(FnChildren, b"get_children_count\0"),
                calculate_subtree_sum: sym!(FnSubtree, b"calculate_subtree_sum\0"),
                process_string: sym!(FnProcessString, b"process_string\0"),
                safe_double_to_int: sym!(FnSafeD2I, b"safe_double_to_int\0"),
                maxnmin: sym!(FnMaxnmin, b"maxnmin\0"),
                _lib: lib,
            }
        }
    }

    // ---- thin safe wrappers -------------------------------------------------

    pub fn add_node(&self, id: i32, parent_id: i32, name: &[u8], value: f64) -> i32 {
        let mut buf = name.to_vec();
        buf.push(0);
        unsafe { (self.add_node)(id, parent_id, buf.as_ptr() as *const c_char, value) }
    }

    /// Raw `Node*`; compare only NULL-ness and *relative* offsets across libs.
    pub fn find_raw(&self, id: i32) -> *mut Node {
        unsafe { (self.find_node_by_id)(id) }
    }

    pub fn find_snap(&self, id: i32) -> Option<NodeSnap> {
        let p = self.find_raw(id);
        if p.is_null() {
            return None;
        }
        Some(unsafe { snap(p) })
    }

    pub fn children(&self, parent_id: i32) -> i32 {
        unsafe { (self.get_children_count)(parent_id) }
    }

    /// Bit pattern of the returned double, so NaN payloads and -0.0 are compared
    /// exactly rather than by `==`.
    pub fn subtree_bits(&self, node_id: i32) -> u64 {
        unsafe { (self.calculate_subtree_sum)(node_id) }.to_bits()
    }

    pub fn process_string(&self, s: &[u8]) -> i32 {
        let mut buf = s.to_vec();
        buf.push(0);
        unsafe { (self.process_string)(buf.as_mut_ptr() as *mut c_char) }
    }

    /// Run `process_string` on a `name` array *in place* inside the library's own
    /// storage, exactly as `maxnmin` does.
    pub fn process_name_in_place(&self, id: i32) -> Option<i32> {
        let p = self.find_raw(id);
        if p.is_null() {
            return None;
        }
        unsafe { Some((self.process_string)((*p).name.as_mut_ptr())) }
    }

    pub fn safe_d2i(&self, d: f64) -> i32 {
        unsafe { (self.safe_double_to_int)(d) }
    }

    pub fn maxnmin(&self, a: i32, b: i32, c: i32, d: i32) -> i32 {
        unsafe { (self.maxnmin)(a, b, c, d) }
    }

    // ---- in-place mutation through the returned `Node*` ---------------------
    // This is the library's only public write path into stored nodes: C hands
    // back a non-const `Node*`. Real consumers use it, so the tests do too.

    pub fn set_active(&self, id: i32, active: i32) -> bool {
        let p = self.find_raw(id);
        if p.is_null() {
            return false;
        }
        unsafe { (*p).active = active };
        true
    }

    pub fn set_parent(&self, id: i32, parent_id: i32) -> bool {
        let p = self.find_raw(id);
        if p.is_null() {
            return false;
        }
        unsafe { (*p).parent_id = parent_id };
        true
    }

    pub fn set_value(&self, id: i32, value: f64) -> bool {
        let p = self.find_raw(id);
        if p.is_null() {
            return false;
        }
        unsafe { (*p).value = value };
        true
    }
}

pub unsafe fn snap(p: *const Node) -> NodeSnap {
    unsafe {
        let n = &*p;
        NodeSnap {
            id: n.id,
            parent_id: n.parent_id,
            name: n.name.iter().map(|&c| c as u8).collect(),
            value_bits: n.value.to_bits(),
            active: n.active,
        }
    }
}

// ---------------------------------------------------------------------------
// locating and privately copying the two shared objects
// ---------------------------------------------------------------------------

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

pub fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("HARVEST_C_SO") {
        return PathBuf::from(p);
    }
    let build = crate_root().join("../c_src/build");
    let mut found: Vec<PathBuf> = std::fs::read_dir(&build)
        .unwrap_or_else(|e| {
            panic!(
                "cannot read {} ({e}); build the C library first:\n  \
                 cd c_src && mkdir -p build && cd build && \
                 cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
                build.display()
            )
        })
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            p.file_name()
                .and_then(|s| s.to_str())
                .map(|s| s.starts_with("lib") && s.ends_with(".so"))
                .unwrap_or(false)
        })
        .collect();
    found.sort();
    assert_eq!(
        found.len(),
        1,
        "expected exactly one lib*.so in {}, found {:?}",
        build.display(),
        found
    );
    found.pop().unwrap()
}

pub fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("HARVEST_RUST_SO") {
        return PathBuf::from(p);
    }
    let root = crate_root();
    // Prefer the release artifact: that is the shipping `.so`, and it is the one
    // built with `panic = "abort"`.
    for prof in ["release", "debug"] {
        let p = root.join("target").join(prof).join("libmaxnmin_lib.so");
        if p.exists() {
            return p;
        }
    }
    panic!("libmaxnmin_lib.so not found; run `cargo build --release` first");
}

static UNIQ: AtomicU64 = AtomicU64::new(0);

fn private_copy(src: &Path, tag: &str) -> (PathBuf, ()) {
    let n = UNIQ.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!("harvest-diff-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let dst = dir.join(format!("{tag}-{n}-lib.so"));
    std::fs::copy(src, &dst)
        .unwrap_or_else(|e| panic!("copy {} -> {}: {e}", src.display(), dst.display()));
    (dst, ())
}

/// A matched pair of freshly-loaded libraries, each with pristine statics.
pub struct Pair {
    pub c: Lib,
    pub r: Lib,
}

/// Load an independent, pristine (`node_count == 0`) instance of both libraries.
pub fn fresh_pair() -> Pair {
    let (cp, _) = private_copy(&c_so_path(), "c");
    let (rp, _) = private_copy(&rust_so_path(), "rust");
    let c = Lib::load("C", &cp);
    let r = Lib::load("RUST", &rp);
    // The mappings survive unlinking on Linux; keep /tmp tidy.
    let _ = std::fs::remove_file(&cp);
    let _ = std::fs::remove_file(&rp);
    Pair { c, r }
}

/// NOTE: there is deliberately no "shared" loader. `dlopen` de-duplicates by
/// inode, so two tests asking for the same `.so` path would receive the SAME
/// instance -- and therefore the same `node_count` / `node_storage` globals.
/// Since cargo runs test functions in parallel threads, that produces a data
/// race on the library's own state and manufactures phantom "divergences".
/// Every test must take its own `fresh_pair()`.

// ---------------------------------------------------------------------------
// deterministic PRNG (xorshift64*) -- fixed seed => reproducible runs
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
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    pub fn next_i32(&mut self) -> i32 {
        self.next_u32() as i32
    }
    /// Uniform in `0..n`.
    pub fn below(&mut self, n: u64) -> u64 {
        assert!(n > 0);
        self.next_u64() % n
    }
    pub fn range_i32(&mut self, lo: i32, hi: i32) -> i32 {
        let span = (hi as i64 - lo as i64 + 1) as u64;
        (lo as i64 + self.below(span) as i64) as i32
    }
    /// Arbitrary `f64`, any bit pattern (subnormals, NaN, inf included).
    pub fn any_f64(&mut self) -> f64 {
        f64::from_bits(self.next_u64())
    }
    /// A "reasonable" double: moderate magnitude with a fraction, either sign.
    pub fn tame_f64(&mut self) -> f64 {
        let m = (self.next_u64() % 2_000_000) as f64 / 1000.0;
        if self.next_u64() & 1 == 0 { m } else { -m }
    }
    /// A double near the `int` conversion boundaries.
    pub fn boundary_f64(&mut self) -> f64 {
        let base: f64 = match self.next_u64() % 4 {
            0 => i32::MAX as f64,
            1 => i32::MIN as f64,
            2 => 0.0,
            _ => 2147483648.0,
        };
        let jitter = match self.next_u64() % 6 {
            0 => 0.0,
            1 => 0.5,
            2 => -0.5,
            3 => 1.0,
            4 => -1.0,
            _ => (self.next_u64() % 8) as f64 * 0.25,
        };
        let d = base + jitter;
        if self.next_u64() & 1 == 0 { d } else { -d }
    }
    /// A double from a mixed pool, weighted toward interesting cases.
    pub fn mixed_f64(&mut self) -> f64 {
        match self.next_u64() % 10 {
            0 => f64::NAN,
            1 => f64::INFINITY,
            2 => f64::NEG_INFINITY,
            3 => 0.0,
            4 => -0.0,
            5 => self.boundary_f64(),
            6 => self.any_f64(),
            7 => 1e300 * if self.next_u64() & 1 == 0 { 1.0 } else { -1.0 },
            _ => self.tame_f64(),
        }
    }
    pub fn ascii_name(&mut self, len: usize) -> Vec<u8> {
        (0..len)
            .map(|_| b'a' + (self.next_u64() % 26) as u8)
            .collect()
    }
    /// Bytes spanning `0x01..=0xFF` (no interior NUL): exercises signed `char`.
    pub fn full_byte_name(&mut self, len: usize) -> Vec<u8> {
        (0..len)
            .map(|_| 1u8 + (self.next_u64() % 255) as u8)
            .collect()
    }
}

// ---------------------------------------------------------------------------
// assertion helpers
// ---------------------------------------------------------------------------

#[track_caller]
pub fn eq_i32(row: &str, ctx: impl std::fmt::Debug, c: i32, r: i32) {
    assert_eq!(
        c, r,
        "[{row}] C/Rust divergence for {ctx:?}: C returned {c}, Rust returned {r}"
    );
}

#[track_caller]
pub fn eq_bits(row: &str, ctx: impl std::fmt::Debug, c: u64, r: u64) {
    assert_eq!(
        c,
        r,
        "[{row}] C/Rust divergence for {ctx:?}: C returned {} (bits {:#018x}), \
         Rust returned {} (bits {:#018x})",
        f64::from_bits(c),
        c,
        f64::from_bits(r),
        r
    );
}

#[track_caller]
pub fn eq_node(row: &str, ctx: impl std::fmt::Debug, c: &Option<NodeSnap>, r: &Option<NodeSnap>) {
    assert_eq!(
        c.is_some(),
        r.is_some(),
        "[{row}] NULL-ness divergence for {ctx:?}: C found={}, Rust found={}",
        c.is_some(),
        r.is_some()
    );
    assert_eq!(c, r, "[{row}] node-content divergence for {ctx:?}");
}
