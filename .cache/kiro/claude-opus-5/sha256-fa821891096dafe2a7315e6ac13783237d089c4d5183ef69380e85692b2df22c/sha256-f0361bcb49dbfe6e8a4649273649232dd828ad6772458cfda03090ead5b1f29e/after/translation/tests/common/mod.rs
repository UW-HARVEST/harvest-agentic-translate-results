//! Shared harness: loads BOTH the C `.so` and the Rust `.so` through
//! `libloading` and exposes them behind one identical API surface, so every
//! test calls each implementation only via its exported symbols — exactly as an
//! external C consumer would. No Rust function is ever called directly.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::c_int;
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard};

// ---------------------------------------------------------------------------
// Types mirrored from the C ABI
// ---------------------------------------------------------------------------

/// `typedef struct { int *data; size_t size; size_t capacity; } DynamicArray;`
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DynamicArray {
    pub data: *mut c_int,
    pub size: usize,
    pub capacity: usize,
}

/// Snapshot of a `DynamicArray`'s observable state (pointer identity excluded,
/// since the two libraries obviously get different addresses from malloc).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArraySnapshot {
    pub data_is_null: bool,
    pub size: usize,
    pub capacity: usize,
    pub elements: Vec<c_int>,
}

type FnInitArray = unsafe extern "C" fn(usize) -> *mut DynamicArray;
type FnExpandArray = unsafe extern "C" fn(*mut DynamicArray) -> c_int;
type FnAddElement = unsafe extern "C" fn(*mut DynamicArray, c_int) -> c_int;
type FnFreeArray = unsafe extern "C" fn(*mut DynamicArray);
type FnProcessFlags = unsafe extern "C" fn(c_int) -> c_int;
type FnCalcChecksum = unsafe extern "C" fn() -> c_int;
type FnMatrixsum = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

/// One loaded implementation (either the C `.so` or the Rust `.so`).
pub struct Impl {
    pub name: &'static str,
    _lib: Library,
    f_init_array: FnInitArray,
    f_expand_array: FnExpandArray,
    f_add_element: FnAddElement,
    f_free_array: FnFreeArray,
    f_process_flags: FnProcessFlags,
    f_calc_checksum: FnCalcChecksum,
    f_matrixsum: FnMatrixsum,
    /// The exported mutable `int matrix[3][4]` data object.
    p_matrix: *mut c_int,
}

impl Impl {
    unsafe fn load(name: &'static str, path: &PathBuf) -> Impl {
        let lib = Library::new(path)
            .unwrap_or_else(|e| panic!("failed to dlopen {} ({:?}): {e}", name, path));

        macro_rules! sym {
            ($t:ty, $n:literal) => {{
                let s: Symbol<$t> = lib
                    .get($n)
                    .unwrap_or_else(|e| panic!("{} missing symbol {}: {e}", name,
                                               String::from_utf8_lossy($n)));
                *s.into_raw()
            }};
        }

        let f_init_array = sym!(FnInitArray, b"init_array\0");
        let f_expand_array = sym!(FnExpandArray, b"expand_array\0");
        let f_add_element = sym!(FnAddElement, b"add_element\0");
        let f_free_array = sym!(FnFreeArray, b"free_array\0");
        let f_process_flags = sym!(FnProcessFlags, b"process_flags\0");
        let f_calc_checksum = sym!(FnCalcChecksum, b"calculate_matrix_checksum\0");
        let f_matrixsum = sym!(FnMatrixsum, b"matrixsum\0");

        let m: Symbol<*mut c_int> = lib
            .get(b"matrix\0")
            .unwrap_or_else(|e| panic!("{name} missing data symbol matrix: {e}"));
        let p_matrix = *m.into_raw() as *mut c_int;

        Impl {
            name,
            _lib: lib,
            f_init_array,
            f_expand_array,
            f_add_element,
            f_free_array,
            f_process_flags,
            f_calc_checksum,
            f_matrixsum,
            p_matrix,
        }
    }

    // --- thin wrappers over the exported symbols -------------------------

    pub unsafe fn init_array(&self, cap: usize) -> *mut DynamicArray {
        (self.f_init_array)(cap)
    }
    pub unsafe fn expand_array(&self, arr: *mut DynamicArray) -> c_int {
        (self.f_expand_array)(arr)
    }
    pub unsafe fn add_element(&self, arr: *mut DynamicArray, v: c_int) -> c_int {
        (self.f_add_element)(arr, v)
    }
    pub unsafe fn free_array(&self, arr: *mut DynamicArray) {
        (self.f_free_array)(arr)
    }
    pub unsafe fn process_flags(&self, flags: c_int) -> c_int {
        (self.f_process_flags)(flags)
    }
    pub unsafe fn calculate_matrix_checksum(&self) -> c_int {
        (self.f_calc_checksum)()
    }
    pub unsafe fn matrixsum(&self, a: c_int, b: c_int, c: c_int, d: c_int) -> c_int {
        (self.f_matrixsum)(a, b, c, d)
    }

    /// Read all 12 cells of the exported `matrix` global, row-major.
    pub unsafe fn read_matrix(&self) -> [c_int; 12] {
        let mut out = [0; 12];
        for (i, o) in out.iter_mut().enumerate() {
            *o = *self.p_matrix.add(i);
        }
        out
    }

    /// Overwrite all 12 cells of the exported `matrix` global, row-major.
    pub unsafe fn write_matrix(&self, vals: &[c_int; 12]) {
        for (i, v) in vals.iter().enumerate() {
            *self.p_matrix.add(i) = *v;
        }
    }

    /// Observable state of an array returned by this implementation.
    pub unsafe fn snapshot(&self, arr: *mut DynamicArray, n_elems: usize) -> Option<ArraySnapshot> {
        if arr.is_null() {
            return None;
        }
        let a = *arr;
        let mut elements = Vec::with_capacity(n_elems);
        if !a.data.is_null() {
            for i in 0..n_elems {
                elements.push(*a.data.add(i));
            }
        }
        Some(ArraySnapshot {
            data_is_null: a.data.is_null(),
            size: a.size,
            capacity: a.capacity,
            elements,
        })
    }
}

/// The pair of implementations under differential test.
///
/// `matrix` is an exported *mutable* data object, and the dynamic loader
/// refcounts `dlopen`, so both `.so`s' globals are process-wide state shared by
/// every test thread in the binary. Holding this guard for the lifetime of the
/// `Pair` serializes the tests so that a test mutating `matrix` cannot perturb a
/// concurrently running one. (Without it the suite reports spurious
/// divergences; verified.)
pub struct Pair {
    pub c: Impl,
    pub rust: Impl,
    _guard: MutexGuard<'static, ()>,
}

fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.pop();
    p
}

fn find_one(dir: &PathBuf, pred: impl Fn(&str) -> bool) -> Option<PathBuf> {
    let rd = std::fs::read_dir(dir).ok()?;
    let mut hits: Vec<PathBuf> = rd
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.extension().map(|x| x == "so").unwrap_or(false)
                && p.file_name()
                    .and_then(|n| n.to_str())
                    .map(&pred)
                    .unwrap_or(false)
        })
        .collect();
    hits.sort();
    hits.pop()
}

pub fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("MATRIXSUM_C_SO") {
        return PathBuf::from(p);
    }
    let root = repo_root();
    let build = root.join("c_src/build");
    find_one(&build, |_| true).unwrap_or_else(|| {
        panic!(
            "no C .so found in {:?}. Build it with:\n  cd c_src && mkdir -p build && cd build \
             && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            build
        )
    })
}

pub fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("MATRIXSUM_RUST_SO") {
        return PathBuf::from(p);
    }
    let root = repo_root();
    for profile in ["release", "debug"] {
        let dir = root.join("translation/target").join(profile);
        if let Some(p) = find_one(&dir, |n| n.contains("matrixsum_lib")) {
            return p;
        }
    }
    panic!("no Rust cdylib found; run `cargo build --release` in translation/")
}

/// Load both libraries. Each test gets its own `dlopen` handle pair, but the
/// dynamic loader refcounts, so the `matrix` global is process-wide per `.so`;
/// tests that mutate it restore it afterwards, and the returned `Pair` holds a
/// process-wide lock so mutation cannot leak across concurrent test threads.
pub fn load_pair() -> Pair {
    static LOCK: Mutex<()> = Mutex::new(());
    let guard = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    unsafe {
        Pair {
            c: Impl::load("C", &c_so_path()),
            rust: Impl::load("Rust", &rust_so_path()),
            _guard: guard,
        }
    }
}

/// Probe `init_array(cap)` on both implementations **sequentially**: allocate,
/// snapshot, free on the C side before touching the Rust side.
///
/// This matters for large capacities. Holding both multi-gigabyte buffers at
/// once doubles the peak footprint and pushes the second allocation over the
/// process' memory ceiling, which makes whichever implementation is called
/// second spuriously return NULL. (Observed here: a 4 GiB request succeeds
/// alone but fails when the other library's 4 GiB buffer is still live.)
/// Verified: with sequential probing the two agree on every size.
pub fn probe_init_sequential(p: &Pair, cap: usize) -> (Option<ArraySnapshot>, Option<ArraySnapshot>) {
    unsafe {
        let ca = p.c.init_array(cap);
        let cs = p.c.snapshot(ca, 0);
        if !ca.is_null() {
            p.c.free_array(ca);
        }

        let ra = p.rust.init_array(cap);
        let rs = p.rust.snapshot(ra, 0);
        if !ra.is_null() {
            p.rust.free_array(ra);
        }

        (cs, rs)
    }
}

// ---------------------------------------------------------------------------
// Deterministic RNG (fixed seed, reproducible) — SplitMix64
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
    /// Full-range `i32`, uniformly.
    pub fn i32_any(&mut self) -> c_int {
        self.next_u32() as i32
    }
    /// Small-magnitude `i32` in `[-1000, 1000]`.
    pub fn i32_small(&mut self) -> c_int {
        (self.next_u32() % 2001) as i32 - 1000
    }
    /// Small non-zero `i32` in `[-1000, 1000] \ {0}`.
    pub fn i32_small_nonzero(&mut self) -> c_int {
        let v = (self.next_u32() % 2000) as i32 - 1000;
        if v == 0 {
            7
        } else {
            v
        }
    }
    pub fn below(&mut self, n: u64) -> u64 {
        if n == 0 {
            0
        } else {
            self.next_u64() % n
        }
    }
    pub fn range(&mut self, lo: u64, hi_inclusive: u64) -> u64 {
        lo + self.below(hi_inclusive - lo + 1)
    }
}

/// The boundary `int` values every row's cross-product should include.
pub const I32_BOUNDARIES: [c_int; 9] = [
    i32::MIN,
    i32::MIN + 1,
    -1000,
    -1,
    0,
    1,
    1000,
    i32::MAX - 1,
    i32::MAX,
];

/// Assert C and Rust agree, with a message naming the configuration.
#[macro_export]
macro_rules! diff_eq {
    ($cv:expr, $rv:expr, $($ctx:tt)*) => {{
        let cv = $cv;
        let rv = $rv;
        assert_eq!(cv, rv, "C/Rust divergence: {}\n  C    = {:?}\n  Rust = {:?}",
                   format!($($ctx)*), cv, rv);
    }};
}
