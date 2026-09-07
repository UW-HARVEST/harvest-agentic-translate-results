//! Shared harness: loads BOTH the C `.so` and the Rust `.so` via `libloading`
//! and exposes every exported symbol behind an identical interface, so a test
//! can drive the two implementations with the same script and diff the results.
//!
//! Rust functions are NEVER called directly — always through the `.so` exports.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::os::raw::{c_int, c_void};
use std::path::PathBuf;

pub const RECORD_SIZE: usize = 48;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DataRecord {
    pub id: c_int,
    pub value: c_int,
    pub timestamp: i64,
    pub name: [u8; 32],
}

impl DataRecord {
    pub fn zeroed() -> Self {
        DataRecord { id: 0, value: 0, timestamp: 0, name: [0; 32] }
    }
}

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR = <root>/translation
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).parent().unwrap().to_path_buf()
}

pub fn c_so_path() -> PathBuf {
    let build = workspace_root().join("c_src/build");
    let mut found = None;
    for entry in std::fs::read_dir(&build)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}. Build the C library first.", build.display()))
    {
        let p = entry.unwrap().path();
        let name = p.file_name().unwrap().to_string_lossy().to_string();
        if name.starts_with("lib") && name.ends_with(".so") {
            found = Some(p);
        }
    }
    found.expect("no lib*.so in c_src/build — build the C library first")
}

pub fn rust_so_path() -> PathBuf {
    let man = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for profile in ["release", "debug"] {
        let p = man.join("target").join(profile).join("libhatch_lib.so");
        if p.exists() {
            return p;
        }
    }
    panic!("libhatch_lib.so not found — run `cargo build --release` first");
}

/// One loaded implementation. Because the globals `global_counter` /
/// `global_accumulator` are file-scope statics, a fresh `Lib` is NOT necessarily
/// fresh state: `dlopen` of an already-loaded library returns the same handle.
/// Tests that need pristine globals use `Impl::fresh_pair()` which loads copies
/// under unique filenames.
pub struct Impl {
    lib: Library,
    pub tag: &'static str,
}

macro_rules! getfn {
    ($self:ident, $name:literal, $t:ty) => {{
        let f: Symbol<$t> = unsafe { $self.lib.get($name.as_bytes()) }
            .unwrap_or_else(|e| panic!("{}: missing symbol {}: {e}", $self.tag, $name));
        f
    }};
}

impl Impl {
    pub fn open(path: &std::path::Path, tag: &'static str) -> Impl {
        let lib = unsafe { Library::new(path) }
            .unwrap_or_else(|e| panic!("dlopen {} failed: {e}", path.display()));
        Impl { lib, tag }
    }

    // ---- exported entry points (all 12) ----

    pub fn increment_counter(&self, value: c_int, unused: c_int) {
        let f = getfn!(self, "increment_counter", unsafe extern "C" fn(c_int, c_int));
        unsafe { f(value, unused) }
    }

    pub fn update_accumulator(&self, value: c_int, unused: c_int) {
        let f = getfn!(self, "update_accumulator", unsafe extern "C" fn(c_int, c_int));
        unsafe { f(value, unused) }
    }

    pub fn add_three(&self, a: c_int, b: c_int, c: c_int) -> c_int {
        let f = getfn!(self, "add_three", unsafe extern "C" fn(c_int, c_int, c_int) -> c_int);
        unsafe { f(a, b, c) }
    }

    pub fn multiply_add(&self, a: c_int, b: c_int, c: c_int) -> c_int {
        let f = getfn!(self, "multiply_add", unsafe extern "C" fn(c_int, c_int, c_int) -> c_int);
        unsafe { f(a, b, c) }
    }

    pub fn complex_calc(&self, a: c_int, b: c_int, c: c_int) -> c_int {
        let f = getfn!(self, "complex_calc", unsafe extern "C" fn(c_int, c_int, c_int) -> c_int);
        unsafe { f(a, b, c) }
    }

    /// Raw address of an exported `int(int,int,int)` symbol in *this* library,
    /// for use as the `operation_func` argument of `apply_operation`.
    pub fn op3_addr(&self, name: &str) -> *const c_void {
        let f: Symbol<unsafe extern "C" fn(c_int, c_int, c_int) -> c_int> =
            unsafe { self.lib.get(name.as_bytes()) }
                .unwrap_or_else(|e| panic!("{}: missing symbol {name}: {e}", self.tag));
        (*f) as *const c_void
    }

    /// Raw address of an exported `void(int,int)` symbol (deliberate ABI mismatch
    /// when handed to `apply_operation`, exactly as the C would allow).
    pub fn mod2_addr(&self, name: &str) -> *const c_void {
        let f: Symbol<unsafe extern "C" fn(c_int, c_int)> =
            unsafe { self.lib.get(name.as_bytes()) }
                .unwrap_or_else(|e| panic!("{}: missing symbol {name}: {e}", self.tag));
        (*f) as *const c_void
    }

    pub fn apply_operation(&self, op: *const c_void, a: c_int, b: c_int, c: c_int) -> c_int {
        let f = getfn!(
            self,
            "apply_operation",
            unsafe extern "C" fn(*const c_void, c_int, c_int, c_int) -> c_int
        );
        unsafe { f(op, a, b, c) }
    }

    pub fn shift_array_data(&self, arr: *mut c_int, size: c_int, shift_by: c_int) {
        let f = getfn!(self, "shift_array_data", unsafe extern "C" fn(*mut c_int, c_int, c_int));
        unsafe { f(arr, size, shift_by) }
    }

    pub fn process_pointer_data(&self, ptr: *mut c_int, multiplier: c_int) -> c_int {
        let f =
            getfn!(self, "process_pointer_data", unsafe extern "C" fn(*mut c_int, c_int) -> c_int);
        unsafe { f(ptr, multiplier) }
    }

    pub fn compute_with_dynamic_memory(&self, base: c_int, count: c_int) -> c_int {
        let f = getfn!(
            self,
            "compute_with_dynamic_memory",
            unsafe extern "C" fn(c_int, c_int) -> c_int
        );
        unsafe { f(base, count) }
    }

    pub fn get_time_based_value(&self, seed: c_int) -> c_int {
        let f = getfn!(self, "get_time_based_value", unsafe extern "C" fn(c_int) -> c_int);
        unsafe { f(seed) }
    }

    pub fn manipulate_records(
        &self,
        records: *mut DataRecord,
        num_records: c_int,
        shift: c_int,
    ) -> c_int {
        let f = getfn!(
            self,
            "manipulate_records",
            unsafe extern "C" fn(*mut DataRecord, c_int, c_int) -> c_int
        );
        unsafe { f(records, num_records, shift) }
    }

    pub fn hatch(&self, p1: c_int, p2: c_int, p3: c_int, p4: c_int) -> c_int {
        let f = getfn!(self, "hatch", unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int);
        unsafe { f(p1, p2, p3, p4) }
    }
}

/// A C/Rust pair sharing one process. Both libraries keep their own copy of the
/// statics, so state accumulates independently but in lockstep.
pub struct Pair {
    pub c: Impl,
    pub r: Impl,
    _tmp: Option<tempdir::TempDirLike>,
}

pub mod tempdir {
    use std::path::PathBuf;
    pub struct TempDirLike(pub PathBuf);
    impl Drop for TempDirLike {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
}

static FRESH_SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

impl Pair {
    /// Loads whatever is already resident (state may be carried over).
    pub fn shared() -> Pair {
        Pair {
            c: Impl::open(&c_so_path(), "C"),
            r: Impl::open(&rust_so_path(), "RUST"),
            _tmp: None,
        }
    }

    /// Loads a *pristine* copy of each library (zeroed globals) by copying the
    /// `.so`s to unique paths, so `dlopen` cannot return a cached handle.
    pub fn fresh() -> Pair {
        let n = FRESH_SEQ.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!(
            "hatchdiff-{}-{}-{}",
            std::process::id(),
            n,
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .subsec_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let cdst = dir.join("libc_impl.so");
        let rdst = dir.join("librust_impl.so");
        std::fs::copy(c_so_path(), &cdst).unwrap();
        std::fs::copy(rust_so_path(), &rdst).unwrap();
        Pair {
            c: Impl::open(&cdst, "C"),
            r: Impl::open(&rdst, "RUST"),
            _tmp: Some(tempdir::TempDirLike(dir)),
        }
    }
}

/// Deterministic xorshift PRNG (fixed seed ⇒ reproducible property tests).
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed | 1)
    }
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
    pub fn i32(&mut self) -> i32 {
        self.next_u64() as u32 as i32
    }
    /// Small-magnitude value: avoids the (unavoidable) overflow noise when a test
    /// wants to exercise a shape rather than wrap-around.
    pub fn small(&mut self) -> i32 {
        (self.next_u64() % 2001) as i32 - 1000
    }
    pub fn range(&mut self, lo: i32, hi: i32) -> i32 {
        debug_assert!(lo < hi);
        let span = (hi as i64 - lo as i64) as u64;
        (lo as i64 + (self.next_u64() % span) as i64) as i32
    }
    /// Mixes plain randoms with the boundary values that trigger overflow.
    pub fn edgy(&mut self) -> i32 {
        match self.next_u64() % 8 {
            0 => i32::MIN,
            1 => i32::MAX,
            2 => i32::MIN + 1,
            3 => i32::MAX - 1,
            4 => 0,
            5 => -1,
            6 => 1,
            _ => self.i32(),
        }
    }
}

pub fn as_bytes<T>(v: &[T]) -> &[u8] {
    unsafe { std::slice::from_raw_parts(v.as_ptr() as *const u8, std::mem::size_of_val(v)) }
}
