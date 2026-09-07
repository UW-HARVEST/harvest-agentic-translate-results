// Differential-test harness.
//
// Loads BOTH the C shared library and the Rust cdylib through `libloading` and
// exposes identical typed wrappers around their exported symbols, so every
// assertion in the test suite crosses the real FFI boundary in both directions.
//
// Neither library exports its `static` node table, and there is no public reset
// entry point, so a "pristine" library state can only be obtained from a fresh
// mapping. `Harness::fresh()` therefore copies both `.so` files to unique
// temporary paths and `dlopen`s the copies: each harness instance owns a private
// copy of `node_storage` / `node_count`, which also makes the tests safe to run
// in parallel.

#![allow(dead_code)]

use std::ffi::c_char;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

pub const MAX_NODES: usize = 100;
pub const MAX_NAME_LEN: usize = 50;

/// Byte-identical layout to the C `Node` (x86-64 SysV: 80 bytes, offsets
/// 0/4/8/64/72).
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Node {
    pub id: i32,
    pub parent_id: i32,
    pub name: [c_char; MAX_NAME_LEN],
    pub value: f64,
    pub active: i32,
}

/// Comparable, printable snapshot of a `Node` as observed through a returned
/// pointer. `value` is captured as raw bits so NaN payloads are compared too.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct NodeSnap {
    pub id: i32,
    pub parent_id: i32,
    pub name: [u8; MAX_NAME_LEN],
    pub value_bits: u64,
    pub active: i32,
}

impl NodeSnap {
    unsafe fn read(p: *const Node) -> NodeSnap {
        let n = unsafe { *p };
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

type FnAddNode = unsafe extern "C" fn(i32, i32, *const c_char, f64) -> i32;
type FnFindNode = unsafe extern "C" fn(i32) -> *mut Node;
type FnChildren = unsafe extern "C" fn(i32) -> i32;
type FnSubtree = unsafe extern "C" fn(i32) -> f64;
type FnProcess = unsafe extern "C" fn(*mut c_char) -> i32;
type FnSdti = unsafe extern "C" fn(f64) -> i32;
type FnMaxnmin = unsafe extern "C" fn(i32, i32, i32, i32) -> i32;

/// One loaded library (C or Rust) with all seven exports resolved.
pub struct Lib {
    _lib: libloading::Library,
    tmp_path: PathBuf,
    pub tag: &'static str,
    pub add_node: FnAddNode,
    pub find_node_by_id: FnFindNode,
    pub get_children_count: FnChildren,
    pub calculate_subtree_sum: FnSubtree,
    pub process_string: FnProcess,
    pub safe_double_to_int: FnSdti,
    pub maxnmin: FnMaxnmin,
}

impl Drop for Lib {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.tmp_path);
    }
}

impl Lib {
    fn load(orig: &Path, tag: &'static str) -> Lib {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!("diffso-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("create temp dir");
        let tmp_path = dir.join(format!("lib{}-{}-{}.so", tag, std::process::id(), n));
        std::fs::copy(orig, &tmp_path)
            .unwrap_or_else(|e| panic!("copy {} -> {}: {e}", orig.display(), tmp_path.display()));

        unsafe {
            let lib = libloading::Library::new(&tmp_path)
                .unwrap_or_else(|e| panic!("dlopen {}: {e}", tmp_path.display()));
            macro_rules! sym {
                ($t:ty, $name:literal) => {{
                    let s: libloading::Symbol<$t> = lib
                        .get($name)
                        .unwrap_or_else(|e| {
                            panic!("{} missing {}: {e}", tag, stringify!($name))
                        });
                    *s
                }};
            }
            let add_node = sym!(FnAddNode, b"add_node\0");
            let find_node_by_id = sym!(FnFindNode, b"find_node_by_id\0");
            let get_children_count = sym!(FnChildren, b"get_children_count\0");
            let calculate_subtree_sum = sym!(FnSubtree, b"calculate_subtree_sum\0");
            let process_string = sym!(FnProcess, b"process_string\0");
            let safe_double_to_int = sym!(FnSdti, b"safe_double_to_int\0");
            let maxnmin = sym!(FnMaxnmin, b"maxnmin\0");
            Lib {
                _lib: lib,
                tmp_path,
                tag,
                add_node,
                find_node_by_id,
                get_children_count,
                calculate_subtree_sum,
                process_string,
                safe_double_to_int,
                maxnmin,
            }
        }
    }

    // ---- typed, safe-ish wrappers -----------------------------------------

    pub fn add(&self, id: i32, parent_id: i32, name: &[u8], value: f64) -> i32 {
        let mut buf: Vec<u8> = name.to_vec();
        buf.push(0);
        unsafe { (self.add_node)(id, parent_id, buf.as_ptr() as *const c_char, value) }
    }

    /// Raw `add_node` with a caller-provided NUL-terminated buffer (allows
    /// embedded NULs / exact byte control).
    pub fn add_raw(&self, id: i32, parent_id: i32, name_with_nul: &[u8], value: f64) -> i32 {
        assert!(name_with_nul.contains(&0), "buffer must be NUL terminated");
        unsafe {
            (self.add_node)(
                id,
                parent_id,
                name_with_nul.as_ptr() as *const c_char,
                value,
            )
        }
    }

    pub fn find_snap(&self, id: i32) -> Option<NodeSnap> {
        let p = unsafe { (self.find_node_by_id)(id) };
        if p.is_null() {
            None
        } else {
            Some(unsafe { NodeSnap::read(p) })
        }
    }

    pub fn find_ptr(&self, id: i32) -> *mut Node {
        unsafe { (self.find_node_by_id)(id) }
    }

    /// Index of the node returned for `id`, relative to the node returned for
    /// `base_id`. Lets pointer identity be compared between libraries whose
    /// storage lives at different addresses.
    pub fn find_rel_index(&self, id: i32, base_id: i32) -> Option<isize> {
        let base = self.find_ptr(base_id);
        assert!(!base.is_null(), "{}: base id {} not found", self.tag, base_id);
        let p = self.find_ptr(id);
        if p.is_null() {
            return None;
        }
        let diff = (p as isize) - (base as isize);
        assert_eq!(
            diff % (std::mem::size_of::<Node>() as isize),
            0,
            "{}: pointer not node-aligned",
            self.tag
        );
        Some(diff / std::mem::size_of::<Node>() as isize)
    }

    /// Index of the node returned for `id`, relative to an explicit base
    /// pointer obtained from the *same* library.
    pub fn rel_to(&self, id: i32, base: *mut Node) -> Option<isize> {
        let p = self.find_ptr(id);
        if p.is_null() {
            return None;
        }
        let diff = (p as isize) - (base as isize);
        assert_eq!(
            diff % (std::mem::size_of::<Node>() as isize),
            0,
            "{}: pointer not node-aligned",
            self.tag
        );
        Some(diff / std::mem::size_of::<Node>() as isize)
    }

    pub fn children(&self, parent_id: i32) -> i32 {
        unsafe { (self.get_children_count)(parent_id) }
    }

    pub fn subtree_bits(&self, node_id: i32) -> u64 {
        unsafe { (self.calculate_subtree_sum)(node_id) }.to_bits()
    }

    pub fn process(&self, bytes_with_nul: &mut Vec<u8>) -> i32 {
        assert!(bytes_with_nul.contains(&0), "buffer must be NUL terminated");
        unsafe { (self.process_string)(bytes_with_nul.as_mut_ptr() as *mut c_char) }
    }

    pub fn process_str(&self, s: &[u8]) -> i32 {
        let mut buf: Vec<u8> = s.to_vec();
        buf.push(0);
        self.process(&mut buf)
    }

    pub fn sdti(&self, d: f64) -> i32 {
        unsafe { (self.safe_double_to_int)(d) }
    }

    pub fn run(&self, a: i32, b: i32, c: i32, d: i32) -> i32 {
        unsafe { (self.maxnmin)(a, b, c, d) }
    }
}

// ---------------------------------------------------------------------------
// Library discovery
// ---------------------------------------------------------------------------

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("manifest dir has a parent")
        .to_path_buf()
}

pub fn c_so_path() -> PathBuf {
    let build = workspace_root().join("c_src/build");
    let mut found: Vec<PathBuf> = Vec::new();
    if let Ok(rd) = std::fs::read_dir(&build) {
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().map(|x| x == "so").unwrap_or(false) {
                found.push(p);
            }
        }
    }
    found.sort();
    assert!(
        !found.is_empty(),
        "no C .so under {}. Build it with:\n  cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        build.display()
    );
    found.remove(0)
}

pub fn rust_so_path() -> PathBuf {
    // `DIFF_RUST_SO` lets the same suite be run against a specific build of the
    // cdylib (release *and* debug are both verified; see run_all.sh).
    if let Ok(p) = std::env::var("DIFF_RUST_SO") {
        let p = PathBuf::from(p);
        assert!(p.exists(), "DIFF_RUST_SO={} does not exist", p.display());
        return p;
    }
    let base = workspace_root().join("translation/target");
    for profile in ["release", "debug"] {
        let p = base.join(profile).join("libmaxnmin_lib.so");
        if p.exists() {
            return p;
        }
    }
    panic!(
        "libmaxnmin_lib.so not found under {}. Build it with: cargo build --release",
        base.display()
    );
}

/// A pair of freshly mapped libraries with pristine (all-zero) global state.
pub struct Pair {
    pub c: Lib,
    pub r: Lib,
}

impl Pair {
    pub fn fresh() -> Pair {
        Pair {
            c: Lib::load(&c_so_path(), "c"),
            r: Lib::load(&rust_so_path(), "rust"),
        }
    }

    /// Compare `find_node_by_id(id)` between the two libraries: NULL-ness, the
    /// full 80-byte struct behind the pointer, and the pointer's index within
    /// `node_storage` relative to the node found for `base_id` (so pointer
    /// identity is compared even though the two storages live at different
    /// addresses). If `base_id` is absent from both, the index check is skipped
    /// (but the equal absence is still asserted).
    #[track_caller]
    pub fn assert_find(&self, id: i32, base_id: i32, what: &str) {
        assert_eq!(
            self.c.find_snap(id),
            self.r.find_snap(id),
            "{what}: find_node_by_id({id}) snapshot"
        );
        let cb = self.c.find_ptr(base_id);
        let rb = self.r.find_ptr(base_id);
        assert_eq!(
            cb.is_null(),
            rb.is_null(),
            "{what}: base id {base_id} presence differs"
        );
        if !cb.is_null() {
            assert_eq!(
                self.c.rel_to(id, cb),
                self.r.rel_to(id, rb),
                "{what}: find_node_by_id({id}) storage index"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (splitmix64) — fixed seeds for reproducibility
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
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    pub fn next_i32(&mut self) -> i32 {
        self.next_u32() as i32
    }
    /// Uniform in `[lo, hi]` inclusive.
    pub fn range_i32(&mut self, lo: i32, hi: i32) -> i32 {
        debug_assert!(lo <= hi);
        let span = (hi as i64 - lo as i64 + 1) as u64;
        (lo as i64 + (self.next_u64() % span) as i64) as i32
    }
    pub fn range_usize(&mut self, lo: usize, hi: usize) -> usize {
        debug_assert!(lo <= hi);
        lo + (self.next_u64() % ((hi - lo + 1) as u64)) as usize
    }
    /// Arbitrary `f64`, including NaN / inf / subnormals.
    pub fn any_f64(&mut self) -> f64 {
        f64::from_bits(self.next_u64())
    }
    /// A "reasonable" finite double in `[-scale, scale]`.
    pub fn finite_f64(&mut self, scale: f64) -> f64 {
        let u = (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64; // [0,1)
        (u * 2.0 - 1.0) * scale
    }
    pub fn byte_nonzero(&mut self) -> u8 {
        1 + (self.next_u64() % 255) as u8
    }
}

/// Random NUL-free byte string of length `len`.
pub fn rand_name(rng: &mut Rng, len: usize) -> Vec<u8> {
    (0..len).map(|_| rng.byte_nonzero()).collect()
}

/// Random NUL-free byte string whose length is drawn from `[lo, hi]`.
pub fn rand_name_upto(rng: &mut Rng, lo: usize, hi: usize) -> Vec<u8> {
    let len = rng.range_usize(lo, hi);
    rand_name(rng, len)
}

/// A curated set of interesting doubles.
pub fn interesting_doubles() -> Vec<f64> {
    let int_max = i32::MAX as f64;
    let int_min = i32::MIN as f64;
    vec![
        0.0,
        -0.0,
        1.0,
        -1.0,
        0.5,
        -0.5,
        0.9999999999,
        -0.9999999999,
        f64::MIN_POSITIVE,
        -f64::MIN_POSITIVE,
        5e-324, // smallest subnormal
        -5e-324,
        int_max,
        int_max - 1.0,
        int_max + 1.0,
        int_max + 0.5,
        int_max - 0.5,
        f64::from_bits(int_max.to_bits() + 1),
        f64::from_bits(int_max.to_bits() - 1),
        int_min,
        int_min + 1.0,
        int_min - 1.0,
        int_min - 0.5,
        int_min + 0.5,
        f64::from_bits(int_min.to_bits() + 1),
        f64::from_bits(int_min.to_bits() - 1),
        2147483647.999,
        -2147483648.999,
        1e300,
        -1e300,
        f64::MAX,
        f64::MIN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NAN,
        -f64::NAN,
        f64::from_bits(0x7FF0_0000_0000_0001), // signalling NaN
        f64::from_bits(0xFFF8_0000_DEAD_BEEF), // NaN with payload
        10.5,
        20.7,
        15.3,
        5.9,
        8.2,
        12.4,
        -10.5,
        123456789.25,
        -123456789.25,
    ]
}

/// A curated set of interesting `i32`s.
pub fn interesting_ints() -> Vec<i32> {
    vec![
        i32::MIN,
        i32::MIN + 1,
        i32::MIN + 5,
        -1000,
        -7,
        -6,
        -5,
        -3,
        -2,
        -1,
        0,
        1,
        2,
        3,
        5,
        6,
        7,
        1000,
        i32::MAX - 5,
        i32::MAX - 1,
        i32::MAX,
    ]
}

/// Assert two `i32` results from the two libraries agree.
#[track_caller]
pub fn eq_i32(what: &str, cv: i32, rv: i32) {
    assert_eq!(cv, rv, "{what}: C returned {cv}, Rust returned {rv}");
}

/// Assert two doubles agree bit-for-bit (so NaN payloads are checked too).
#[track_caller]
pub fn eq_bits(what: &str, cv: u64, rv: u64) {
    assert_eq!(
        cv,
        rv,
        "{what}: C returned {:?} (bits {:#018x}), Rust returned {:?} (bits {:#018x})",
        f64::from_bits(cv),
        cv,
        f64::from_bits(rv),
        rv
    );
}
