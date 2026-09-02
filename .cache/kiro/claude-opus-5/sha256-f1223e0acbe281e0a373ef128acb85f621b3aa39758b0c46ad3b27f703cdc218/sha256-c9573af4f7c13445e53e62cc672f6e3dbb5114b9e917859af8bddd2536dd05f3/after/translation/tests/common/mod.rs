//! Differential-test support: loads BOTH the C `.so` and the Rust `.so` through
//! `libloading` and calls every entry point across the FFI boundary only.
//!
//! No Rust function is ever called directly — every call goes through
//! `dlsym`, so the `#[no_mangle]`/`extern "C"` export wrappers are under test
//! too.
//!
//! Both libraries carry process-wide `static` state (`node_storage` /
//! `node_count`). To give each test a pristine store (`node_count == 0`) the
//! `.so` files are *copied* to a unique temp path before `dlopen`. glibc keys
//! already-loaded objects by (st_dev, st_ino), so a real copy is guaranteed to
//! get its own fresh copy of the statics — unlike a symlink/hardlink, and
//! unlike relying on `dlclose` actually unloading (it often does not, because
//! the Rust cdylib pulls in TLS).

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::{c_char, c_double, c_int};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

pub const MAX_NODES: usize = 100;
pub const MAX_NAME_LEN: usize = 50;

/// Mirrors the C `Node` struct.
#[repr(C)]
#[derive(Copy, Clone)]
pub struct Node {
    pub id: c_int,
    pub parent_id: c_int,
    pub name: [c_char; MAX_NAME_LEN],
    pub value: c_double,
    pub active: c_int,
}

/// A plain-data snapshot of a `Node`, comparable with `==`.
/// `value` is captured as raw bits so NaN payloads and `-0.0` compare exactly.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct NodeSnap {
    pub id: i32,
    pub parent_id: i32,
    pub name: Vec<u8>,
    pub value_bits: u64,
    pub active: i32,
}

impl NodeSnap {
    unsafe fn from_ptr(p: *const Node) -> NodeSnap {
        let n = unsafe { &*p };
        NodeSnap {
            id: n.id,
            parent_id: n.parent_id,
            name: n.name.iter().map(|&b| b as u8).collect(),
            value_bits: n.value.to_bits(),
            active: n.active,
        }
    }
}

// ---------------------------------------------------------------------------
// .so discovery
// ---------------------------------------------------------------------------

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crate dir has a parent")
        .to_path_buf()
}

fn find_c_so() -> PathBuf {
    if let Ok(p) = std::env::var("C_SO") {
        return PathBuf::from(p);
    }
    let build = workspace_root().join("c_src/build");
    let mut found: Vec<PathBuf> = std::fs::read_dir(&build)
        .unwrap_or_else(|e| panic!("cannot read {}: {e} — build the C library first", build.display()))
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().map(|x| x == "so").unwrap_or(false))
        .collect();
    found.sort();
    assert_eq!(
        found.len(),
        1,
        "expected exactly one .so in {}, found {found:?}",
        build.display()
    );
    found.pop().unwrap()
}

fn find_rust_so() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_SO") {
        return PathBuf::from(p);
    }
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    for profile in ["release", "debug"] {
        let p = root.join("target").join(profile).join("libmaxnmin_lib.so");
        if p.exists() {
            return p;
        }
    }
    panic!("libmaxnmin_lib.so not found — run `cargo build --release` first");
}

// ---------------------------------------------------------------------------
// A single loaded library
// ---------------------------------------------------------------------------

pub struct Lib {
    _lib: Library,
    pub add_node: unsafe extern "C" fn(c_int, c_int, *const c_char, c_double) -> c_int,
    pub find_node_by_id: unsafe extern "C" fn(c_int) -> *mut Node,
    pub get_children_count: unsafe extern "C" fn(c_int) -> c_int,
    pub calculate_subtree_sum: unsafe extern "C" fn(c_int) -> c_double,
    pub process_string: unsafe extern "C" fn(*mut c_char) -> c_int,
    pub safe_double_to_int: unsafe extern "C" fn(c_double) -> c_int,
    pub maxnmin: unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int,
}

macro_rules! sym {
    ($lib:expr, $name:literal, $ty:ty) => {{
        let s: Symbol<$ty> = unsafe { $lib.get(concat!($name, "\0").as_bytes()) }
            .unwrap_or_else(|e| panic!("missing symbol {}: {e}", $name));
        *s
    }};
}

impl Lib {
    pub fn open(path: &Path) -> Lib {
        let lib = unsafe { Library::new(path) }
            .unwrap_or_else(|e| panic!("dlopen {} failed: {e}", path.display()));
        let l = Lib {
            add_node: sym!(
                lib,
                "add_node",
                unsafe extern "C" fn(c_int, c_int, *const c_char, c_double) -> c_int
            ),
            find_node_by_id: sym!(lib, "find_node_by_id", unsafe extern "C" fn(c_int) -> *mut Node),
            get_children_count: sym!(lib, "get_children_count", unsafe extern "C" fn(c_int) -> c_int),
            calculate_subtree_sum: sym!(
                lib,
                "calculate_subtree_sum",
                unsafe extern "C" fn(c_int) -> c_double
            ),
            process_string: sym!(lib, "process_string", unsafe extern "C" fn(*mut c_char) -> c_int),
            safe_double_to_int: sym!(
                lib,
                "safe_double_to_int",
                unsafe extern "C" fn(c_double) -> c_int
            ),
            maxnmin: sym!(
                lib,
                "maxnmin",
                unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int
            ),
            _lib: lib,
        };
        l
    }
}

// ---------------------------------------------------------------------------
// The C/Rust pair
// ---------------------------------------------------------------------------

static COUNTER: AtomicU64 = AtomicU64::new(0);

pub struct Pair {
    pub c: Lib,
    pub r: Lib,
    tmp: PathBuf,
}

impl Drop for Pair {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.tmp);
    }
}

impl Pair {
    /// Fresh, independent instances of both libraries (`node_count == 0`).
    pub fn fresh() -> Pair {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let tmp = std::env::temp_dir().join(format!(
            "difftest-{}-{}-{}",
            std::process::id(),
            n,
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&tmp).expect("mkdir temp");

        let c_dst = tmp.join("libc_under_test.so");
        let r_dst = tmp.join("librust_under_test.so");
        std::fs::copy(find_c_so(), &c_dst).expect("copy C .so");
        std::fs::copy(find_rust_so(), &r_dst).expect("copy Rust .so");

        Pair {
            c: Lib::open(&c_dst),
            r: Lib::open(&r_dst),
            tmp,
        }
    }

    // --- differential wrappers: call both, assert equal, return the value ---

    #[track_caller]
    pub fn add_node(&self, id: i32, parent_id: i32, name: &[u8], value: f64) -> i32 {
        let mut buf: Vec<u8> = name.to_vec();
        buf.push(0);
        let cv = unsafe { (self.c.add_node)(id, parent_id, buf.as_ptr() as *const c_char, value) };
        let rv = unsafe { (self.r.add_node)(id, parent_id, buf.as_ptr() as *const c_char, value) };
        assert_eq!(
            cv,
            rv,
            "add_node(id={id}, parent={parent_id}, name={:?}, value={value:e}): C={cv} Rust={rv}",
            String::from_utf8_lossy(name)
        );
        cv
    }

    /// Compares null-ness and, when non-null, every field of the target struct.
    #[track_caller]
    pub fn find_node_by_id(&self, id: i32) -> Option<(*mut Node, *mut Node)> {
        let cp = unsafe { (self.c.find_node_by_id)(id) };
        let rp = unsafe { (self.r.find_node_by_id)(id) };
        assert_eq!(
            cp.is_null(),
            rp.is_null(),
            "find_node_by_id({id}) null-ness: C_null={} Rust_null={}",
            cp.is_null(),
            rp.is_null()
        );
        if cp.is_null() {
            return None;
        }
        let cs = unsafe { NodeSnap::from_ptr(cp) };
        let rs = unsafe { NodeSnap::from_ptr(rp) };
        assert_eq!(cs, rs, "find_node_by_id({id}) struct contents differ");
        Some((cp, rp))
    }

    #[track_caller]
    pub fn get_children_count(&self, parent_id: i32) -> i32 {
        let cv = unsafe { (self.c.get_children_count)(parent_id) };
        let rv = unsafe { (self.r.get_children_count)(parent_id) };
        assert_eq!(cv, rv, "get_children_count({parent_id}): C={cv} Rust={rv}");
        cv
    }

    /// Bitwise comparison so NaN payloads and `-0.0` are distinguished.
    #[track_caller]
    pub fn calculate_subtree_sum(&self, node_id: i32) -> f64 {
        let cv = unsafe { (self.c.calculate_subtree_sum)(node_id) };
        let rv = unsafe { (self.r.calculate_subtree_sum)(node_id) };
        assert_eq!(
            cv.to_bits(),
            rv.to_bits(),
            "calculate_subtree_sum({node_id}): C={cv:?} (0x{:016x}) Rust={rv:?} (0x{:016x})",
            cv.to_bits(),
            rv.to_bits()
        );
        cv
    }

    #[track_caller]
    pub fn process_string(&self, s: &[u8]) -> i32 {
        let mut cbuf: Vec<u8> = s.to_vec();
        cbuf.push(0);
        let mut rbuf: Vec<u8> = s.to_vec();
        rbuf.push(0);
        let cv = unsafe { (self.c.process_string)(cbuf.as_mut_ptr() as *mut c_char) };
        let rv = unsafe { (self.r.process_string)(rbuf.as_mut_ptr() as *mut c_char) };
        assert_eq!(
            cv,
            rv,
            "process_string(len={}, {:?}): C={cv} Rust={rv}",
            s.len(),
            &s[..s.len().min(24)]
        );
        // The C function must not have modified the buffer either.
        assert_eq!(cbuf, rbuf, "process_string mutated its buffer differently");
        cv
    }

    #[track_caller]
    pub fn safe_double_to_int(&self, d: f64) -> i32 {
        let cv = unsafe { (self.c.safe_double_to_int)(d) };
        let rv = unsafe { (self.r.safe_double_to_int)(d) };
        assert_eq!(
            cv,
            rv,
            "safe_double_to_int({d:?} / 0x{:016x}): C={cv} Rust={rv}",
            d.to_bits()
        );
        cv
    }

    #[track_caller]
    pub fn maxnmin(&self, p1: i32, p2: i32, p3: i32, p4: i32) -> i32 {
        let cv = unsafe { (self.c.maxnmin)(p1, p2, p3, p4) };
        let rv = unsafe { (self.r.maxnmin)(p1, p2, p3, p4) };
        assert_eq!(
            cv, rv,
            "maxnmin({p1}, {p2}, {p3}, {p4}): C={cv} Rust={rv}"
        );
        cv
    }

    // --- state-mutation helpers (applied identically to both libraries) ---

    /// Writes `active` through the pointer `find_node_by_id` returned, in both
    /// libraries. Also exercises the `#[repr(C)]` field layout.
    #[track_caller]
    pub fn set_active(&self, id: i32, active: i32) {
        let (cp, rp) = self
            .find_node_by_id(id)
            .unwrap_or_else(|| panic!("set_active: node {id} not found"));
        unsafe {
            (*cp).active = active;
            (*rp).active = active;
        }
    }

    #[track_caller]
    pub fn set_value(&self, id: i32, value: f64) {
        let (cp, rp) = self
            .find_node_by_id(id)
            .unwrap_or_else(|| panic!("set_value: node {id} not found"));
        unsafe {
            (*cp).value = value;
            (*rp).value = value;
        }
    }

    /// `process_string` fed the live `name` field of a node in each library
    /// (the exact composition `maxnmin` performs).
    #[track_caller]
    pub fn process_node_name(&self, id: i32) -> i32 {
        let (cp, rp) = self
            .find_node_by_id(id)
            .unwrap_or_else(|| panic!("process_node_name: node {id} not found"));
        let cv = unsafe { (self.c.process_string)((*cp).name.as_mut_ptr()) };
        let rv = unsafe { (self.r.process_string)((*rp).name.as_mut_ptr()) };
        assert_eq!(cv, rv, "process_string(node {id}.name): C={cv} Rust={rv}");
        cv
    }

    /// Snapshot the whole visible store by probing every id we know about.
    #[track_caller]
    pub fn assert_store_agrees(&self, ids: &[i32]) {
        for &id in ids {
            self.find_node_by_id(id);
            self.get_children_count(id);
            self.calculate_subtree_sum(id);
        }
    }
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (xorshift64*)
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(if seed == 0 { 0x9E37_79B9_7F4A_7C15 } else { seed })
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
    /// Uniform in `lo..=hi`.
    pub fn range_i32(&mut self, lo: i32, hi: i32) -> i32 {
        let span = (hi as i64 - lo as i64 + 1) as u64;
        (lo as i64 + self.below(span) as i64) as i32
    }
    /// Arbitrary bit pattern reinterpreted as `f64` (inf/NaN/subnormal included).
    pub fn next_f64_bits(&mut self) -> f64 {
        f64::from_bits(self.next_u64())
    }
    /// A "reasonable" finite double, log-uniform-ish magnitude, random sign.
    pub fn next_finite_f64(&mut self) -> f64 {
        let mant = (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64;
        let exp = self.range_i32(-40, 40);
        let sign = if self.next_u64() & 1 == 0 { 1.0 } else { -1.0 };
        sign * mant * 2f64.powi(exp)
    }
    /// A double inside `[INT_MIN, INT_MAX]`.
    pub fn next_in_int_range(&mut self) -> f64 {
        let u = (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64; // [0,1)
        i32::MIN as f64 + u * (i32::MAX as f64 - i32::MIN as f64)
    }
    pub fn bytes(&mut self, len: usize, lo: u8, hi: u8) -> Vec<u8> {
        let span = (hi - lo) as u64 + 1;
        (0..len).map(|_| lo + self.below(span) as u8).collect()
    }
}

/// The seed mandated by CONFIGS.md for reproducibility.
pub const SEED: u64 = 0x5EED_1234_5678_9ABC;

// ---------------------------------------------------------------------------
// Crash-isolation helper (for the two NULL-deref UB sites)
// ---------------------------------------------------------------------------

/// Re-executes this test binary, running only `test_name`, with
/// `DIFFTEST_CRASH_MODE` set. Returns the raw wait status.
pub fn run_isolated(test_name: &str, mode: &str) -> std::process::Output {
    let exe = std::env::current_exe().expect("current_exe");
    std::process::Command::new(exe)
        .args(["--exact", test_name, "--nocapture", "--test-threads", "1"])
        .env("DIFFTEST_CRASH_MODE", mode)
        .output()
        .expect("spawn isolated child")
}

pub fn crash_mode() -> Option<String> {
    std::env::var("DIFFTEST_CRASH_MODE").ok()
}
