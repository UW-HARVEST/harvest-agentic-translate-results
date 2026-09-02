//! Shared harness: loads BOTH the C `.so` and the Rust `.so` via `libloading`
//! and calls every function through the FFI boundary only. No Rust function is
//! ever called directly, so the `#[no_mangle]` export wrappers are under test.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, c_void};
use std::path::{Path, PathBuf};

pub type HashBytesFn = unsafe extern "C" fn(*mut c_void, usize, usize) -> usize;
pub type SipHashFn = unsafe extern "C" fn(c_int);

extern "C" {
    fn dup(fd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn open(path: *const c_char, flags: c_int, ...) -> c_int;
    fn lseek(fd: c_int, off: i64, whence: c_int) -> i64;
    fn read(fd: c_int, buf: *mut c_void, n: usize) -> isize;
    fn unlink(path: *const c_char) -> c_int;
}

const O_RDWR: c_int = 2;
const O_CREAT: c_int = 64;
const O_TRUNC: c_int = 512;
const SEEK_SET: c_int = 0;

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Locate the C shared library produced by `c_src/build`.
fn c_lib_path() -> PathBuf {
    let build = manifest_dir().join("..").join("c_src").join("build");
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(rd) = std::fs::read_dir(&build) {
        for e in rd.flatten() {
            let p = e.path();
            let name = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
            if name.starts_with("lib") && name.ends_with(".so") {
                candidates.push(p);
            }
        }
    }
    candidates.sort();
    candidates.into_iter().next().unwrap_or_else(|| {
        panic!(
            "no C .so found in {:?}; build it with:\n  cd c_src && mkdir -p build && cd build \\\n    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            build
        )
    })
}

/// Every Rust `cdylib` build that exists (debug and/or release). Both are tested
/// because `[profile.release] panic = "abort"` and debug overflow checks make
/// them materially different builds of the same source.
fn rust_lib_paths() -> Vec<PathBuf> {
    let target = manifest_dir().join("target");
    let mut out = Vec::new();
    for profile in ["debug", "release"] {
        let p = target.join(profile).join("libsiphash_lib.so");
        if p.exists() {
            out.push(p);
        }
    }
    assert!(
        !out.is_empty(),
        "no Rust cdylib found under {:?}; run `cargo build` / `cargo build --release`",
        target
    );
    out
}

pub struct Pair {
    pub c: Library,
    pub r: Library,
    pub rust_path: PathBuf,
}

impl Pair {
    pub fn c_hash(&self) -> Symbol<'_, HashBytesFn> {
        unsafe { self.c.get(b"stbds_hash_bytes\0") }.expect("C stbds_hash_bytes")
    }
    pub fn r_hash(&self) -> Symbol<'_, HashBytesFn> {
        unsafe { self.r.get(b"stbds_hash_bytes\0") }.expect("Rust stbds_hash_bytes")
    }
    pub fn c_siphash(&self) -> Symbol<'_, SipHashFn> {
        unsafe { self.c.get(b"siphash\0") }.expect("C siphash")
    }
    pub fn r_siphash(&self) -> Symbol<'_, SipHashFn> {
        unsafe { self.r.get(b"siphash\0") }.expect("Rust siphash")
    }
}

/// One `Pair` per available Rust build profile.
pub fn pairs() -> Vec<Pair> {
    let cpath = c_lib_path();
    rust_lib_paths()
        .into_iter()
        .map(|rp| {
            let c = unsafe { Library::new(&cpath) }
                .unwrap_or_else(|e| panic!("dlopen {:?}: {e}", cpath));
            let r =
                unsafe { Library::new(&rp) }.unwrap_or_else(|e| panic!("dlopen {:?}: {e}", rp));
            Pair {
                c,
                r,
                rust_path: rp,
            }
        })
        .collect()
}

/// Serialises the process-wide fd-1 redirect used by `capture_stdout`, since
/// `cargo test` runs test functions on parallel threads.
static STDOUT_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Runs `f` with process stdout redirected to a temp file and returns the raw
/// bytes written. Both `.so`s share the process libc `stdout`, so
/// `fflush(NULL)` captures output from either side.
pub fn capture_stdout<F: FnOnce()>(tag: &str, f: F) -> Vec<u8> {
    let _guard = STDOUT_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    // Drain Rust's own (separately buffered) stdout BEFORE swapping fd 1, so
    // libtest progress output cannot land in the capture file.
    {
        use std::io::Write;
        let _ = std::io::stdout().flush();
        let _ = std::io::stderr().flush();
    }
    let path = std::env::temp_dir().join(format!(
        "harvest_cap_{}_{}_{}.txt",
        tag,
        std::process::id(),
        COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
    ));
    let cpath = std::ffi::CString::new(path.to_str().unwrap()).unwrap();

    unsafe {
        fflush(std::ptr::null_mut());
        let saved = dup(1);
        assert!(saved >= 0, "dup(1) failed");
        let tmp = open(cpath.as_ptr(), O_RDWR | O_CREAT | O_TRUNC, 0o600 as c_int);
        assert!(tmp >= 0, "open temp failed");
        assert!(dup2(tmp, 1) >= 0, "dup2 failed");

        f();

        fflush(std::ptr::null_mut());
        assert!(dup2(saved, 1) >= 0, "dup2 restore failed");
        close(saved);

        lseek(tmp, 0, SEEK_SET);
        let mut out = Vec::new();
        let mut buf = [0u8; 8192];
        loop {
            let n = read(tmp, buf.as_mut_ptr() as *mut c_void, buf.len());
            if n <= 0 {
                break;
            }
            out.extend_from_slice(&buf[..n as usize]);
        }
        close(tmp);
        unlink(cpath.as_ptr());
        out
    }
}

static COUNTER: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

/// Deterministic splitmix64 PRNG — fixed seed, reproducible across runs.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed)
    }
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    pub fn next_u8(&mut self) -> u8 {
        (self.next_u64() >> 33) as u8
    }
    /// Uniform in `[0, n)`; `n` must be non-zero.
    pub fn below(&mut self, n: u64) -> u64 {
        self.next_u64() % n
    }
    pub fn fill(&mut self, buf: &mut [u8]) {
        for b in buf.iter_mut() {
            *b = self.next_u8();
        }
    }
}

/// Assert C and Rust agree for one `(bytes, len, seed)` triple, calling through
/// both `.so` exports.
pub fn assert_hash_eq(p: &Pair, bytes: &mut [u8], len: usize, seed: usize, ctx: &str) {
    let cf = p.c_hash();
    let rf = p.r_hash();
    let ptr = if bytes.is_empty() {
        std::ptr::null_mut()
    } else {
        bytes.as_mut_ptr() as *mut c_void
    };
    let cv = unsafe { cf(ptr, len, seed) };
    let rv = unsafe { rf(ptr, len, seed) };
    assert_eq!(
        cv,
        rv,
        "stbds_hash_bytes mismatch [{}] rust={:?}\n  len={} seed={:#018x}\n  C   = {:#018x}\n  Rust= {:#018x}\n  bytes[..{}]={:02x?}",
        ctx,
        p.rust_path.file_name().unwrap_or_default(),
        len,
        seed,
        cv,
        rv,
        len.min(bytes.len()),
        &bytes[..len.min(bytes.len())]
    );
}

/// Same, but for a raw pointer (used for NULL / unaligned / exact-size buffers).
pub unsafe fn assert_hash_eq_ptr(
    p: &Pair,
    ptr: *mut c_void,
    len: usize,
    seed: usize,
    ctx: &str,
) -> usize {
    let cf = p.c_hash();
    let rf = p.r_hash();
    let cv = cf(ptr, len, seed);
    let rv = rf(ptr, len, seed);
    assert_eq!(
        cv, rv,
        "stbds_hash_bytes mismatch [{}] rust={:?} ptr={:?} len={} seed={:#018x} C={:#018x} Rust={:#018x}",
        ctx,
        p.rust_path.file_name().unwrap_or_default(),
        ptr,
        len,
        seed,
        cv,
        rv
    );
    cv
}

pub fn describe(p: &Pair) -> String {
    format!("{:?}", p.rust_path.file_name().unwrap_or_default())
}

pub fn c_lib_file() -> PathBuf {
    c_lib_path()
}

pub fn rust_lib_files() -> Vec<PathBuf> {
    rust_lib_paths()
}

pub fn exists(p: &Path) -> bool {
    p.exists()
}
