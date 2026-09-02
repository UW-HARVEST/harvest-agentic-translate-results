// Shared harness: loads BOTH the C `.so` and the Rust `.so` with `libloading`
// and calls every function through its exported C ABI symbol. No Rust function
// is ever called directly, so the `#[no_mangle]` export wrappers are under test
// too.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::{c_char, c_double, c_int, c_void};
use std::path::PathBuf;

pub type FnSafeDoubleToInt = unsafe extern "C" fn(c_double) -> c_int;
pub type FnProcessWithFallthrough = unsafe extern "C" fn(c_int, c_int) -> c_int;
pub type FnCopyDataBlock = unsafe extern "C" fn(*mut c_void, *const c_void);
pub type FnHandlePointerOperations = unsafe extern "C" fn(c_int) -> c_int;
pub type FnOverunder = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

/// The five exported symbols, resolved out of one shared object.
pub struct Api {
    _lib: Library,
    pub name: &'static str,
    pub safe_double_to_int: FnSafeDoubleToInt,
    pub process_with_fallthrough: FnProcessWithFallthrough,
    pub copy_data_block: FnCopyDataBlock,
    pub handle_pointer_operations: FnHandlePointerOperations,
    pub overunder: FnOverunder,
}

impl Api {
    fn load(path: &PathBuf, name: &'static str) -> Api {
        unsafe {
            let lib = Library::new(path)
                .unwrap_or_else(|e| panic!("failed to dlopen {}: {e}", path.display()));
            macro_rules! sym {
                ($t:ty, $n:literal) => {{
                    let s: Symbol<$t> = lib
                        .get($n)
                        .unwrap_or_else(|e| panic!("{} missing symbol {}: {e}", name,
                            String::from_utf8_lossy($n)));
                    *s
                }};
            }
            let safe_double_to_int = sym!(FnSafeDoubleToInt, b"safe_double_to_int\0");
            let process_with_fallthrough =
                sym!(FnProcessWithFallthrough, b"process_with_fallthrough\0");
            let copy_data_block = sym!(FnCopyDataBlock, b"copy_data_block\0");
            let handle_pointer_operations =
                sym!(FnHandlePointerOperations, b"handle_pointer_operations\0");
            let overunder = sym!(FnOverunder, b"overunder\0");
            Api {
                _lib: lib,
                name,
                safe_double_to_int,
                process_with_fallthrough,
                copy_data_block,
                handle_pointer_operations,
                overunder,
            }
        }
    }
}

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <work>/translation
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn find_c_so() -> PathBuf {
    if let Ok(p) = std::env::var("C_SO") {
        return PathBuf::from(p);
    }
    let build = workspace_root().parent().unwrap().join("c_src/build");
    let mut found: Vec<PathBuf> = std::fs::read_dir(&build)
        .unwrap_or_else(|e| panic!("cannot read {}: {e} -- build the C library first", build.display()))
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.starts_with("lib") && n.ends_with(".so"))
                .unwrap_or(false)
        })
        .collect();
    found.sort();
    found
        .pop()
        .unwrap_or_else(|| panic!("no lib*.so in {}", build.display()))
}

fn find_rust_so() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_SO") {
        return PathBuf::from(p);
    }
    let root = workspace_root();
    for profile in ["release", "debug"] {
        let p = root.join("target").join(profile).join("liboverunder_lib.so");
        if p.exists() {
            return p;
        }
    }
    panic!("liboverunder_lib.so not found; run `cargo build --release` first");
}

/// Both implementations, loaded side by side.
pub struct Pair {
    pub c: Api,
    pub r: Api,
}

pub fn load_pair() -> Pair {
    Pair {
        c: Api::load(&find_c_so(), "C"),
        r: Api::load(&find_rust_so(), "RUST"),
    }
}

// ---------------------------------------------------------------------------
// Deterministic RNG (SplitMix64) -- fixed seed for reproducibility.
// ---------------------------------------------------------------------------

pub const SEED: u64 = 0x5EED_1234;

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
    pub fn next_i32(&mut self) -> i32 {
        self.next_u64() as u32 as i32
    }
    pub fn next_u8(&mut self) -> u8 {
        self.next_u64() as u8
    }
    /// Inclusive range.
    pub fn range_i32(&mut self, lo: i32, hi: i32) -> i32 {
        let span = (hi as i64 - lo as i64 + 1) as u64;
        (lo as i64 + (self.next_u64() % span) as i64) as i32
    }
    /// Uniform in [0, 1).
    pub fn next_f64_unit(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }
    /// Arbitrary bit pattern reinterpreted as f64 (may be NaN/inf/subnormal).
    pub fn next_f64_bits(&mut self) -> f64 {
        f64::from_bits(self.next_u64())
    }
}

// ---------------------------------------------------------------------------
// DataBlock layout: { int id; double value; char label[20]; }
//   id @0 (4), pad @4..8, value @8 (8), label @16..36, tail pad @36..40
// The tests never hard-code this: they poison a large buffer and compare which
// bytes each implementation writes, so a layout mismatch would show up as a
// differential failure rather than being assumed away.
// ---------------------------------------------------------------------------
pub const DATABLOCK_SIZE: usize = 40;
pub const PROBE: usize = 128;

/// Aligned scratch buffer big enough to hold a DataBlock plus guard bytes.
#[repr(C, align(16))]
pub struct Probe(pub [u8; PROBE]);

impl Probe {
    pub fn filled(byte: u8) -> Probe {
        Probe([byte; PROBE])
    }
    pub fn as_ptr(&self) -> *const c_void {
        self.0.as_ptr() as *const c_void
    }
    pub fn as_mut_ptr(&mut self) -> *mut c_void {
        self.0.as_mut_ptr() as *mut c_void
    }
}

pub fn fmt_f64(d: f64) -> String {
    format!("{d:?} (bits {:#018x})", d.to_bits())
}

// ---------------------------------------------------------------------------
// stdout capture via dup2 -- both `.so`s write with libc `printf`, i.e. into
// the same process-wide stdio buffers, so a single fflush(NULL) drains both.
// ---------------------------------------------------------------------------

unsafe extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn open(path: *const c_char, flags: c_int, ...) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
}

const O_WRONLY: c_int = 0o1;
const O_CREAT: c_int = 0o100;
const O_TRUNC: c_int = 0o1000;

/// Serialises every fd-1 manipulation inside a test binary (libtest runs tests
/// on multiple threads, and fd 1 is process-wide).
static FD1_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn lock_fd1() -> std::sync::MutexGuard<'static, ()> {
    FD1_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

/// Redirects fd 1 to /dev/null until dropped. `overunder` printf-spams on every
/// call; bulk differential runs would otherwise bury the test output.
pub struct Silence {
    saved: c_int,
    _lock: std::sync::MutexGuard<'static, ()>,
}

pub fn silence_stdout() -> Silence {
    let lock = lock_fd1();
    unsafe {
        fflush(std::ptr::null_mut());
        let saved = dup(1);
        assert!(saved >= 0, "dup(1) failed");
        let devnull = std::ffi::CString::new("/dev/null").unwrap();
        let fd = open(devnull.as_ptr(), O_WRONLY, 0o644 as c_int);
        assert!(fd >= 0, "open(/dev/null) failed");
        assert!(dup2(fd, 1) >= 0, "dup2 failed");
        close(fd);
        Silence { saved, _lock: lock }
    }
}

impl Drop for Silence {
    fn drop(&mut self) {
        unsafe {
            fflush(std::ptr::null_mut());
            dup2(self.saved, 1);
            close(self.saved);
        }
    }
}

/// Run `f` with fd 1 redirected to a fresh temp file; return what was written.
pub fn capture_stdout<F: FnOnce()>(tag: &str, f: F) -> Vec<u8> {
    let _lock = lock_fd1();
    let path = std::env::temp_dir().join(format!("ou_cap_{}_{}.txt", std::process::id(), tag));
    let cpath = std::ffi::CString::new(path.to_str().unwrap()).unwrap();
    unsafe {
        fflush(std::ptr::null_mut());
        let saved = dup(1);
        assert!(saved >= 0, "dup(1) failed");
        let fd = open(cpath.as_ptr(), O_WRONLY | O_CREAT | O_TRUNC, 0o644 as c_int);
        assert!(fd >= 0, "open({}) failed", path.display());
        assert!(dup2(fd, 1) >= 0, "dup2 failed");
        close(fd);

        f();

        fflush(std::ptr::null_mut());
        assert!(dup2(saved, 1) >= 0, "dup2 restore failed");
        close(saved);
    }
    let out = std::fs::read(&path).expect("read capture file");
    let _ = std::fs::remove_file(&path);
    out
}
