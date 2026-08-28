//! Shared differential-test harness.
//!
//! Loads BOTH the C `.so` and the Rust `.so` with `libloading` and exposes the
//! exported symbols through identical wrappers. Nothing here calls a Rust
//! function directly — every call goes through `dlsym` on the cdylib, exactly
//! like an external consumer, so the `#[no_mangle]`/`extern "C"` wrappers are
//! under test too.
#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::c_int;
use std::path::{Path, PathBuf};

/// Mirror of the C `DynamicArray` struct (`c_src/src/lib.c:39`).
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DynamicArray {
    pub data: *mut c_int,
    pub size: usize,
    pub capacity: usize,
}

type FnInitArray = unsafe extern "C" fn(usize) -> *mut DynamicArray;
type FnExpandArray = unsafe extern "C" fn(*mut DynamicArray) -> c_int;
type FnAddElement = unsafe extern "C" fn(*mut DynamicArray, c_int) -> c_int;
type FnFreeArray = unsafe extern "C" fn(*mut DynamicArray);
type FnProcessFlags = unsafe extern "C" fn(c_int) -> c_int;
type FnMatrixChecksum = unsafe extern "C" fn() -> c_int;
type FnMatrixsum = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

/// One loaded implementation (either the C one or the Rust one).
pub struct Impl {
    pub name: &'static str,
    pub path: PathBuf,
    _lib: Library,
    p_init_array: FnInitArray,
    p_expand_array: FnExpandArray,
    p_add_element: FnAddElement,
    p_free_array: FnFreeArray,
    p_process_flags: FnProcessFlags,
    p_calculate_matrix_checksum: FnMatrixChecksum,
    p_matrixsum: FnMatrixsum,
    p_matrix: *mut c_int,
}

impl Impl {
    fn load(name: &'static str, path: PathBuf) -> Impl {
        unsafe {
            let lib = Library::new(&path)
                .unwrap_or_else(|e| panic!("failed to dlopen {} ({}): {e}", name, path.display()));

            macro_rules! sym {
                ($t:ty, $s:expr) => {{
                    let s: Symbol<$t> = lib.get($s).unwrap_or_else(|e| {
                        panic!("{} ({}) is missing symbol {:?}: {e}", name, path.display(), $s)
                    });
                    *s
                }};
            }

            let p_init_array = sym!(FnInitArray, b"init_array\0");
            let p_expand_array = sym!(FnExpandArray, b"expand_array\0");
            let p_add_element = sym!(FnAddElement, b"add_element\0");
            let p_free_array = sym!(FnFreeArray, b"free_array\0");
            let p_process_flags = sym!(FnProcessFlags, b"process_flags\0");
            let p_calculate_matrix_checksum = sym!(FnMatrixChecksum, b"calculate_matrix_checksum\0");
            let p_matrixsum = sym!(FnMatrixsum, b"matrixsum\0");

            // `matrix` is an exported *data* object, not a function.
            let matrix_sym: Symbol<*mut c_int> = lib
                .get(b"matrix\0")
                .unwrap_or_else(|e| panic!("{} is missing data symbol `matrix`: {e}", name));
            let p_matrix = matrix_sym.into_raw().into_raw() as *mut c_int;

            Impl {
                name,
                path,
                _lib: lib,
                p_init_array,
                p_expand_array,
                p_add_element,
                p_free_array,
                p_process_flags,
                p_calculate_matrix_checksum,
                p_matrixsum,
                p_matrix,
            }
        }
    }

    // ---- exported functions -------------------------------------------------

    pub unsafe fn init_array(&self, cap: usize) -> *mut DynamicArray {
        (self.p_init_array)(cap)
    }
    pub unsafe fn expand_array(&self, arr: *mut DynamicArray) -> c_int {
        (self.p_expand_array)(arr)
    }
    pub unsafe fn add_element(&self, arr: *mut DynamicArray, v: c_int) -> c_int {
        (self.p_add_element)(arr, v)
    }
    pub unsafe fn free_array(&self, arr: *mut DynamicArray) {
        (self.p_free_array)(arr)
    }
    pub fn process_flags(&self, flags: c_int) -> c_int {
        unsafe { (self.p_process_flags)(flags) }
    }
    pub fn calculate_matrix_checksum(&self) -> c_int {
        unsafe { (self.p_calculate_matrix_checksum)() }
    }
    pub fn matrixsum(&self, a: c_int, b: c_int, c: c_int, d: c_int) -> c_int {
        unsafe { (self.p_matrixsum)(a, b, c, d) }
    }

    // ---- exported `matrix` data object -------------------------------------

    /// Read the exported `matrix[3][4]` as a flat 12-element array.
    pub fn matrix_read(&self) -> [c_int; 12] {
        let mut out = [0; 12];
        unsafe { std::ptr::copy_nonoverlapping(self.p_matrix, out.as_mut_ptr(), 12) };
        out
    }
    /// Overwrite the exported `matrix[3][4]` from a flat 12-element array.
    pub fn matrix_write(&self, v: &[c_int; 12]) {
        unsafe { std::ptr::copy_nonoverlapping(v.as_ptr(), self.p_matrix, 12) };
    }
    /// Raw 48 bytes of the exported `matrix` object.
    pub fn matrix_bytes(&self) -> [u8; 48] {
        let mut out = [0u8; 48];
        unsafe { std::ptr::copy_nonoverlapping(self.p_matrix as *const u8, out.as_mut_ptr(), 48) };
        out
    }
    pub fn matrix_reset(&self) {
        self.matrix_write(&DEFAULT_MATRIX);
    }

    // ---- struct field readers (through the raw pointer) ---------------------

    /// Read the `DynamicArray` header. `arr` must be non-NULL.
    pub unsafe fn header(&self, arr: *mut DynamicArray) -> DynamicArray {
        std::ptr::read(arr)
    }
    /// Read `n` elements out of `arr->data`.
    pub unsafe fn elements(&self, arr: *mut DynamicArray, n: usize) -> Vec<c_int> {
        let h = std::ptr::read(arr);
        if h.data.is_null() || n == 0 {
            return Vec::new();
        }
        std::slice::from_raw_parts(h.data as *const c_int, n).to_vec()
    }
}

/// The default contents of `matrix` (`c_src/src/lib.c:28`).
pub const DEFAULT_MATRIX: [c_int; 12] = [
    0x01, 0x02, 0x03, 0x04, 0x10, 0x20, 0x30, 0x40, 0xA1, 0xB2, 0xC3, 0xD4,
];

/// The C and the Rust implementation, loaded side by side.
///
/// Holds a process-wide lock: both libraries export a *mutable global*
/// (`matrix`), so tests that run concurrently in the same test binary would
/// otherwise clobber each other's global state. Serialising `load()` keeps every
/// test's view of the globals deterministic.
pub struct Pair {
    pub c: Impl,
    pub rs: Impl,
    _guard: std::sync::MutexGuard<'static, ()>,
}

static GLOBAL_STATE_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn find_c_so() -> PathBuf {
    if let Ok(p) = std::env::var("C_SO") {
        return PathBuf::from(p);
    }
    let build = manifest_dir().join("..").join("c_src").join("build");
    let mut cands: Vec<PathBuf> = Vec::new();
    if let Ok(rd) = std::fs::read_dir(&build) {
        for e in rd.flatten() {
            let p = e.path();
            let name = p.file_name().unwrap_or_default().to_string_lossy().to_string();
            if name.starts_with("lib") && name.ends_with(".so") {
                cands.push(p);
            }
        }
    }
    cands.sort();
    cands.pop().unwrap_or_else(|| {
        panic!(
            "no C shared library found in {}.\n\
             Build it first:\n  cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            build.display()
        )
    })
}

fn find_rust_so() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_SO") {
        return PathBuf::from(p);
    }
    // `cargo test` does not build the cdylib artifact, so pick whichever
    // freshly-built artifact exists (newest wins).
    let mut best: Option<(std::time::SystemTime, PathBuf)> = None;
    for profile in ["release", "debug"] {
        let p = manifest_dir()
            .join("target")
            .join(profile)
            .join("libmatrixsum_lib.so");
        if let Ok(md) = std::fs::metadata(&p) {
            let t = md.modified().unwrap_or(std::time::UNIX_EPOCH);
            if best.as_ref().map(|(bt, _)| t > *bt).unwrap_or(true) {
                best = Some((t, p));
            }
        }
    }
    best.map(|(_, p)| p).unwrap_or_else(|| {
        panic!(
            "no Rust cdylib found under {}/target/{{release,debug}}/libmatrixsum_lib.so.\n\
             Build it first:  cd translation && cargo build --release --offline",
            manifest_dir().display()
        )
    })
}

pub fn load() -> Pair {
    let c_path = find_c_so();
    let rs_path = find_rust_so();
    assert!(Path::new(&c_path).exists(), "missing {}", c_path.display());
    assert!(Path::new(&rs_path).exists(), "missing {}", rs_path.display());
    let guard = GLOBAL_STATE_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let pair = Pair {
        c: Impl::load("C", c_path),
        rs: Impl::load("Rust", rs_path),
        _guard: guard,
    };

    // Snapshot the *as-linked initializers* of the exported `matrix` object
    // BEFORE anything writes to it. This must happen on the very first `load()`
    // of the process, because `matrix_reset()` below (and the tests themselves)
    // overwrite the global — without this snapshot a wrong initializer in the
    // Rust `.so` would be masked by the reset. `dlopen` is reference-counted, so
    // every later `load()` in the same process sees the same (possibly already
    // mutated) memory; the `OnceLock` therefore keeps only the first, genuine
    // reading.
    let _ = PRISTINE_MATRIX.set((pair.c.matrix_bytes(), pair.rs.matrix_bytes()));

    // Every test then starts from a known global state.
    pair.c.matrix_reset();
    pair.rs.matrix_reset();
    pair
}

/// Like [`load`], but does NOT write to the exported `matrix` global, so the
/// as-linked initializer stays observable. Only for use by test binaries whose
/// tests never mutate `matrix`.
pub fn load_pristine() -> Pair {
    let c_path = find_c_so();
    let rs_path = find_rust_so();
    let guard = GLOBAL_STATE_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let pair = Pair {
        c: Impl::load("C", c_path),
        rs: Impl::load("Rust", rs_path),
        _guard: guard,
    };
    let _ = PRISTINE_MATRIX.set((pair.c.matrix_bytes(), pair.rs.matrix_bytes()));
    pair
}

/// `(C initializer bytes, Rust initializer bytes)` of the exported `matrix`
/// object, captured at the first `load()` in this process.
static PRISTINE_MATRIX: std::sync::OnceLock<([u8; 48], [u8; 48])> = std::sync::OnceLock::new();

/// The initializer bytes of `matrix` as they were linked into each `.so`.
pub fn pristine_matrix_bytes() -> ([u8; 48], [u8; 48]) {
    *PRISTINE_MATRIX
        .get()
        .expect("pristine_matrix_bytes() called before the first load()")
}

// ---------------------------------------------------------------------------
// Deterministic RNG (xorshift64*) — fixed seeds, reproducible runs.
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
    pub fn next_i32(&mut self) -> c_int {
        (self.next_u64() >> 32) as u32 as i32
    }
    /// Uniform in `0..n`.
    pub fn below(&mut self, n: u64) -> u64 {
        if n == 0 {
            0
        } else {
            self.next_u64() % n
        }
    }
    /// An `i32` biased towards interesting/edge values.
    pub fn spicy_i32(&mut self) -> c_int {
        match self.below(10) {
            0 => 0,
            1 => 1,
            2 => -1,
            3 => c_int::MAX,
            4 => c_int::MIN,
            5 => c_int::MAX - 1,
            6 => c_int::MIN + 1,
            7 => (self.next_u64() & 0xF) as c_int,
            8 => -((self.next_u64() & 0xF) as c_int),
            _ => self.next_i32(),
        }
    }
}

/// The base seed used by every property-style test.
pub const SEED: u64 = 0x5EED_1234_ABCD_0001;

/// Free memory allocated by either library's `malloc`, from the test process
/// (both `.so`s and the test binary share glibc's allocator).
pub unsafe fn libc_free(p: *mut std::ffi::c_void) {
    extern "C" {
        fn free(p: *mut std::ffi::c_void);
    }
    free(p)
}

/// Allocate with the same glibc allocator both libraries use, so a
/// caller-constructed `DynamicArray` is indistinguishable from one the library
/// made itself (and can legally be handed to `free_array`).
pub unsafe fn libc_malloc(n: usize) -> *mut std::ffi::c_void {
    extern "C" {
        fn malloc(n: usize) -> *mut std::ffi::c_void;
    }
    malloc(n)
}

/// Build a `DynamicArray` on the glibc heap with `capacity` ints of storage and
/// the given `size`, filled from `fill`.
pub unsafe fn make_array(capacity: usize, size: usize, fill: &[c_int]) -> *mut DynamicArray {
    let arr = libc_malloc(std::mem::size_of::<DynamicArray>()) as *mut DynamicArray;
    assert!(!arr.is_null());
    let data = if capacity == 0 {
        libc_malloc(0) as *mut c_int
    } else {
        libc_malloc(capacity * std::mem::size_of::<c_int>()) as *mut c_int
    };
    assert!(!data.is_null());
    for (i, &v) in fill.iter().enumerate().take(capacity) {
        *data.add(i) = v;
    }
    std::ptr::write(arr, DynamicArray { data, size, capacity });
    arr
}
