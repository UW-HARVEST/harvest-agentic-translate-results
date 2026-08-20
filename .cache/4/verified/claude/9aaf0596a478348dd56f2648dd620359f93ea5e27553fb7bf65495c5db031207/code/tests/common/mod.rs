//! Differential-test harness.
//!
//! Loads BOTH shared objects through `libloading` and calls every function
//! through its exported C symbol, exactly like an external consumer. The Rust
//! functions are never called directly, so the `#[no_mangle]` / `extern "C"`
//! export wrappers and the C ABI struct layout are part of what gets tested.
//!
//! Both libraries keep their node graph in file-scope `static` storage with no
//! reset entry point, so every `Pair` copies the two `.so` files to a unique
//! path and `dlopen`s the copies. A distinct path/inode makes the loader create
//! a fresh mapping with a fresh, zeroed data segment, which is the only way to
//! observe the `node_count == 0` state.

#![allow(dead_code)]

use std::ffi::{c_char, c_double, c_int};
use std::path::PathBuf;
use std::process;
use std::sync::atomic::{AtomicU64, Ordering};

// ---------------------------------------------------------------- C constants

/// `#define MAX_NODES 100`
pub const MAX_NODES: usize = 100;
/// `#define MAX_NAME_LEN 50`
pub const MAX_NAME_LEN: usize = 50;
pub const INT_MAX: c_int = i32::MAX;
pub const INT_MIN: c_int = i32::MIN;

/// Field offsets of the C `Node` struct on x86-64 SysV:
/// `int id; int parent_id; char name[50]; double value; int active;`
pub const OFF_ID: usize = 0;
pub const OFF_PARENT_ID: usize = 4;
pub const OFF_NAME: usize = 8;
pub const OFF_VALUE: usize = 64;
pub const OFF_ACTIVE: usize = 72;
pub const SIZEOF_NODE: usize = 80;

// ------------------------------------------------------------- fn ptr aliases

type FnAddNode = unsafe extern "C" fn(c_int, c_int, *const c_char, c_double) -> c_int;
type FnFindNodeById = unsafe extern "C" fn(c_int) -> *mut u8;
type FnGetChildrenCount = unsafe extern "C" fn(c_int) -> c_int;
type FnCalculateSubtreeSum = unsafe extern "C" fn(c_int) -> c_double;
type FnProcessString = unsafe extern "C" fn(*mut c_char) -> c_int;
type FnSafeDoubleToInt = unsafe extern "C" fn(c_double) -> c_int;
type FnMaxnmin = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

/// One loaded implementation (either the C `.so` or the Rust `.so`).
pub struct Lib {
    _lib: libloading::Library,
    pub add_node: FnAddNode,
    pub find_node_by_id: FnFindNodeById,
    pub get_children_count: FnGetChildrenCount,
    pub calculate_subtree_sum: FnCalculateSubtreeSum,
    pub process_string: FnProcessString,
    pub safe_double_to_int: FnSafeDoubleToInt,
    pub maxnmin: FnMaxnmin,
}

impl Lib {
    pub fn open(path: &PathBuf) -> Lib {
        unsafe {
            let lib = libloading::Library::new(path)
                .unwrap_or_else(|e| panic!("dlopen {} failed: {e}", path.display()));
            // Resolve every exported symbol by its exact C name.
            let add_node = *lib
                .get::<FnAddNode>(b"add_node\0")
                .expect("missing symbol add_node");
            let find_node_by_id = *lib
                .get::<FnFindNodeById>(b"find_node_by_id\0")
                .expect("missing symbol find_node_by_id");
            let get_children_count = *lib
                .get::<FnGetChildrenCount>(b"get_children_count\0")
                .expect("missing symbol get_children_count");
            let calculate_subtree_sum = *lib
                .get::<FnCalculateSubtreeSum>(b"calculate_subtree_sum\0")
                .expect("missing symbol calculate_subtree_sum");
            let process_string = *lib
                .get::<FnProcessString>(b"process_string\0")
                .expect("missing symbol process_string");
            let safe_double_to_int = *lib
                .get::<FnSafeDoubleToInt>(b"safe_double_to_int\0")
                .expect("missing symbol safe_double_to_int");
            let maxnmin = *lib
                .get::<FnMaxnmin>(b"maxnmin\0")
                .expect("missing symbol maxnmin");
            Lib {
                _lib: lib,
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
}

// --------------------------------------------------------------- .so location

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn mtime(p: &std::path::Path) -> std::time::SystemTime {
    std::fs::metadata(p)
        .unwrap_or_else(|e| panic!("stat {}: {e}", p.display()))
        .modified()
        .expect("mtime")
}

/// Guard against silently testing a stale artifact. `cargo test` does NOT
/// regenerate a `cdylib`-only lib target (integration tests do not link it), so
/// without this check an edit to `src/lib.rs` can appear to pass while the tests
/// are still running against the previously built `.so`.
fn assert_not_older_than(artifact: &std::path::Path, sources: &[PathBuf], how_to_build: &str) {
    let a = mtime(artifact);
    for s in sources {
        if !s.is_file() {
            continue;
        }
        if mtime(s) > a {
            panic!(
                "STALE ARTIFACT: {} is older than its source {}.\nRebuild it with:\n  {}",
                artifact.display(),
                s.display(),
                how_to_build
            );
        }
    }
}

/// `c_src/build/libtranslated_rust.so`, built by CMake.
pub fn c_so_path() -> PathBuf {
    let p = manifest_dir()
        .join("c_src")
        .join("build")
        .join("libtranslated_rust.so");
    let how = "cd c_src && mkdir -p build && cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .";
    assert!(
        p.is_file(),
        "C shared library not found at {}.\nBuild it with:\n  {how}",
        p.display()
    );
    let c = manifest_dir().join("c_src");
    assert_not_older_than(
        &p,
        &[c.join("src").join("lib.c"), c.join("include").join("lib.h")],
        how,
    );
    p
}

/// `target/<profile>/libmaxnmin_lib.so`, the cdylib produced from `src/lib.rs`.
pub fn rust_so_path() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    // .../target/<profile>/deps/<test-bin> -> walk up looking for the cdylib.
    for dir in exe.ancestors().skip(1) {
        let cand = dir.join("libmaxnmin_lib.so");
        if cand.is_file() {
            assert_not_older_than(
                &cand,
                &[manifest_dir().join("src").join("lib.rs")],
                "cargo build            # add --release when testing the release profile",
            );
            return cand;
        }
    }
    panic!(
        "Rust cdylib libmaxnmin_lib.so not found above {}. Run `cargo build` first.",
        exe.display()
    );
}

static COUNTER: AtomicU64 = AtomicU64::new(0);

/// A freshly `dlopen`ed C + Rust pair with zeroed global state.
pub struct Pair {
    pub c: Lib,
    pub r: Lib,
    dir: PathBuf,
    pub tag: String,
}

impl Pair {
    pub fn new(tag: &str) -> Pair {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!("diffcase-{}-{}", process::id(), n));
        std::fs::create_dir_all(&dir).expect("create temp dir");
        // Unique file names => unique inodes => fresh, zeroed .bss per Pair.
        let cdst = dir.join("libc_impl.so");
        let rdst = dir.join("librs_impl.so");
        std::fs::copy(c_so_path(), &cdst).expect("copy C .so");
        std::fs::copy(rust_so_path(), &rdst).expect("copy Rust .so");
        Pair {
            c: Lib::open(&cdst),
            r: Lib::open(&rdst),
            dir,
            tag: tag.to_string(),
        }
    }
}

impl Drop for Pair {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

// ------------------------------------------------------------ node field view

/// The C-visible contents of a `Node`, read field-by-field at its C offset
/// through the raw `Node*` the library handed back. Reading at hard-coded
/// offsets means a struct-layout mismatch in the Rust translation shows up as a
/// value mismatch.
#[derive(Clone, PartialEq, Eq)]
pub struct NodeView {
    pub id: c_int,
    pub parent_id: c_int,
    pub name: [u8; MAX_NAME_LEN],
    /// raw bits, so NaN compares equal to itself and -0.0 != 0.0
    pub value_bits: u64,
    pub active: c_int,
}

impl std::fmt::Debug for NodeView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NodeView")
            .field("id", &self.id)
            .field("parent_id", &self.parent_id)
            .field("name", &format_args!("{:02x?}", &self.name[..]))
            .field("value_bits", &format_args!("{:#018x}", self.value_bits))
            .field("value", &f64::from_bits(self.value_bits))
            .field("active", &self.active)
            .finish()
    }
}

unsafe fn read_node(p: *mut u8) -> NodeView {
    let mut name = [0u8; MAX_NAME_LEN];
    std::ptr::copy_nonoverlapping(p.add(OFF_NAME), name.as_mut_ptr(), MAX_NAME_LEN);
    NodeView {
        id: std::ptr::read_unaligned(p.add(OFF_ID) as *const c_int),
        parent_id: std::ptr::read_unaligned(p.add(OFF_PARENT_ID) as *const c_int),
        name,
        value_bits: std::ptr::read_unaligned(p.add(OFF_VALUE) as *const u64),
        active: std::ptr::read_unaligned(p.add(OFF_ACTIVE) as *const c_int),
    }
}

// -------------------------------------------------------- differential drivers
//
// Every driver calls the C export and the Rust export with identical arguments
// and asserts the observable results are identical, then returns the value so
// the caller can build further assertions on it.

impl Pair {
    pub fn safe_double_to_int(&self, d: f64) -> c_int {
        let cv = unsafe { (self.c.safe_double_to_int)(d) };
        let rv = unsafe { (self.r.safe_double_to_int)(d) };
        assert_eq!(
            cv, rv,
            "[{}] safe_double_to_int({d:?} bits={:#018x}): C={cv} Rust={rv}",
            self.tag,
            d.to_bits()
        );
        cv
    }

    /// `bytes` must already be NUL-terminated (so interior NULs can be tested).
    pub fn process_string(&self, bytes: &[u8]) -> c_int {
        assert!(bytes.contains(&0), "process_string input must be NUL terminated");
        let mut cbuf = bytes.to_vec();
        let mut rbuf = bytes.to_vec();
        let cv = unsafe { (self.c.process_string)(cbuf.as_mut_ptr() as *mut c_char) };
        let rv = unsafe { (self.r.process_string)(rbuf.as_mut_ptr() as *mut c_char) };
        assert_eq!(
            cv,
            rv,
            "[{}] process_string(len={} {:02x?}): C={cv} Rust={rv}",
            self.tag,
            bytes.len(),
            &bytes[..bytes.len().min(32)]
        );
        // The C function must not modify the buffer.
        assert_eq!(cbuf, rbuf, "[{}] process_string mutated its buffer differently", self.tag);
        cv
    }

    /// `name` must already be NUL-terminated.
    pub fn add_node_raw(&self, id: c_int, parent_id: c_int, name: &[u8], value: f64) -> c_int {
        assert!(name.contains(&0), "add_node name must be NUL terminated");
        let cv = unsafe { (self.c.add_node)(id, parent_id, name.as_ptr() as *const c_char, value) };
        let rv = unsafe { (self.r.add_node)(id, parent_id, name.as_ptr() as *const c_char, value) };
        assert_eq!(
            cv,
            rv,
            "[{}] add_node(id={id}, parent={parent_id}, name={:02x?}, value={value:?}): C={cv} Rust={rv}",
            self.tag,
            &name[..name.len().min(32)]
        );
        cv
    }

    pub fn add_node(&self, id: c_int, parent_id: c_int, name: &str, value: f64) -> c_int {
        let mut buf = name.as_bytes().to_vec();
        buf.push(0);
        self.add_node_raw(id, parent_id, &buf, value)
    }

    /// Returns `Some((c_ptr, r_ptr))` when both found the node, `None` when both
    /// returned NULL. Also asserts the pointed-to `Node` contents match.
    pub fn find_node_by_id(&self, id: c_int) -> Option<(*mut u8, *mut u8)> {
        let cp = unsafe { (self.c.find_node_by_id)(id) };
        let rp = unsafe { (self.r.find_node_by_id)(id) };
        assert_eq!(
            cp.is_null(),
            rp.is_null(),
            "[{}] find_node_by_id({id}): C null={} Rust null={}",
            self.tag,
            cp.is_null(),
            rp.is_null()
        );
        if cp.is_null() {
            return None;
        }
        let cn = unsafe { read_node(cp) };
        let rn = unsafe { read_node(rp) };
        assert_eq!(
            cn, rn,
            "[{}] find_node_by_id({id}) node contents differ:\n C={cn:?}\n R={rn:?}",
            self.tag
        );
        Some((cp, rp))
    }

    /// Field view of a node, asserting C and Rust agree.
    pub fn node_view(&self, id: c_int) -> Option<NodeView> {
        self.find_node_by_id(id)
            .map(|(cp, _)| unsafe { read_node(cp) })
    }

    pub fn get_children_count(&self, parent_id: c_int) -> c_int {
        let cv = unsafe { (self.c.get_children_count)(parent_id) };
        let rv = unsafe { (self.r.get_children_count)(parent_id) };
        assert_eq!(
            cv, rv,
            "[{}] get_children_count({parent_id}): C={cv} Rust={rv}",
            self.tag
        );
        cv
    }

    /// Compares the **raw bit patterns** of the returned doubles, so NaN,
    /// -0.0 and inf all have to match exactly.
    pub fn calculate_subtree_sum(&self, node_id: c_int) -> f64 {
        let cv = unsafe { (self.c.calculate_subtree_sum)(node_id) };
        let rv = unsafe { (self.r.calculate_subtree_sum)(node_id) };
        assert_eq!(
            cv.to_bits(),
            rv.to_bits(),
            "[{}] calculate_subtree_sum({node_id}): C={cv:?} ({:#018x}) Rust={rv:?} ({:#018x})",
            self.tag,
            cv.to_bits(),
            rv.to_bits()
        );
        cv
    }

    pub fn maxnmin(&self, p1: c_int, p2: c_int, p3: c_int, p4: c_int) -> c_int {
        let cv = unsafe { (self.c.maxnmin)(p1, p2, p3, p4) };
        let rv = unsafe { (self.r.maxnmin)(p1, p2, p3, p4) };
        assert_eq!(
            cv, rv,
            "[{}] maxnmin({p1}, {p2}, {p3}, {p4}): C={cv} Rust={rv}",
            self.tag
        );
        cv
    }

    /// Write `active` through the `Node*` both libraries handed back, mirroring
    /// what a C consumer can legally do with the returned pointer.
    pub fn set_active(&self, id: c_int, v: c_int) {
        let (cp, rp) = self
            .find_node_by_id(id)
            .unwrap_or_else(|| panic!("[{}] set_active: node {id} not found", self.tag));
        unsafe {
            std::ptr::write_unaligned(cp.add(OFF_ACTIVE) as *mut c_int, v);
            std::ptr::write_unaligned(rp.add(OFF_ACTIVE) as *mut c_int, v);
        }
    }

    /// Write any `Node` field through the returned pointer at a raw offset.
    pub fn poke_i32(&self, id: c_int, offset: usize, v: c_int) {
        let (cp, rp) = self
            .find_node_by_id(id)
            .unwrap_or_else(|| panic!("[{}] poke_i32: node {id} not found", self.tag));
        unsafe {
            std::ptr::write_unaligned(cp.add(offset) as *mut c_int, v);
            std::ptr::write_unaligned(rp.add(offset) as *mut c_int, v);
        }
    }

    /// Struct stride between two adjacent storage slots, as observed through
    /// the returned pointers. Must be `sizeof(Node)` in both libraries.
    pub fn observed_stride(&self, id_a: c_int, id_b: c_int) -> (isize, isize) {
        let (ca, ra) = self.find_node_by_id(id_a).expect("id_a");
        let (cb, rb) = self.find_node_by_id(id_b).expect("id_b");
        (
            unsafe { cb.offset_from(ca) },
            unsafe { rb.offset_from(ra) },
        )
    }

    /// Probe every read-only entry point over a range of arguments. Used to
    /// re-verify total library state after each mutation.
    pub fn probe_all(&self, ids: &[c_int]) {
        for &id in ids {
            self.find_node_by_id(id);
            self.get_children_count(id);
            self.calculate_subtree_sum(id);
        }
    }
}

// ------------------------------------------------------------------------ PRNG

/// SplitMix64 — deterministic, seeded, no external dependency.
pub struct Rng(pub u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed)
    }
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E3779B97F4A7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
        z ^ (z >> 31)
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
    /// Any bit pattern reinterpreted as f64 (covers NaN / inf / subnormal).
    pub fn next_f64_bits(&mut self) -> f64 {
        f64::from_bits(self.next_u64())
    }
    /// A "reasonable" finite double spread over many magnitudes.
    pub fn next_f64_spread(&mut self) -> f64 {
        let mant = (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64;
        let exp = self.range_i32(-40, 40);
        let sign = if self.next_u64() & 1 == 0 { 1.0 } else { -1.0 };
        sign * mant * 2f64.powi(exp)
    }
    pub fn bytes(&mut self, len: usize, lo: u8, hi: u8) -> Vec<u8> {
        let span = (hi - lo) as u64 + 1;
        (0..len)
            .map(|_| lo.wrapping_add(self.below(span) as u8))
            .collect()
    }
}

/// The interesting scalar values for any `int` parameter.
pub const INT_CLASSES: &[c_int] = &[
    INT_MIN,
    INT_MIN + 1,
    -2147483647,
    -1000003,
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
    1000003,
    INT_MAX - 1,
    INT_MAX,
];

/// The interesting `double` classes for `safe_double_to_int`.
pub fn double_classes() -> Vec<f64> {
    let imax = INT_MAX as f64; //  2147483647.0
    let imin = INT_MIN as f64; // -2147483648.0
    vec![
        f64::NAN,
        -f64::NAN,
        f64::from_bits(0x7FF0_0000_0000_0001), // signalling NaN
        f64::from_bits(0xFFF8_0000_0000_00FF), // NaN, custom payload
        f64::INFINITY,
        f64::NEG_INFINITY,
        0.0,
        -0.0,
        f64::MIN_POSITIVE,
        -f64::MIN_POSITIVE,
        5e-324,  // smallest subnormal
        -5e-324,
        0.5,
        -0.5,
        1.0,
        -1.0,
        1.9,
        -1.9,
        1.5,
        -1.5,
        2.5,
        -2.5,
        imax,
        imax - 1.0,
        imax + 1.0, // == 2147483648.0, strictly > INT_MAX
        2147483648.0,
        2147483647.5,
        f64::from_bits(imax.to_bits() + 1), // nextafter(INT_MAX, +inf)
        imin,
        imin + 1.0,
        imin - 1.0,
        -2147483648.5,
        -2147483649.0,
        f64::from_bits(imin.to_bits() + 1), // nextafter(INT_MIN, -inf)
        1e300,
        -1e300,
        f64::MAX,
        f64::MIN,
        4294967296.0,
        -4294967296.0,
    ]
}
