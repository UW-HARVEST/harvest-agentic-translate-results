// Shared harness for the C-vs-Rust differential tests.
//
// BOTH implementations are loaded as shared objects through `libloading` and
// invoked only through their exported C symbols -- the Rust functions are never
// called directly, so the `#[unsafe(no_mangle)] extern "C"` wrappers are part of
// what is under test.
#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, c_void};
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

pub type FmaArrayFn =
    unsafe extern "C" fn(*mut c_int, *const c_int, *const c_int, *const c_int, c_int);
pub type CallFmaFn = unsafe extern "C" fn(*const c_int, c_int) -> c_int;
pub type DriverFn = unsafe extern "C" fn(*const c_char);

/// One loaded implementation (either the C `.so` or the Rust `.so`).
pub struct Impl {
    pub name: &'static str,
    pub path: PathBuf,
    lib: Library,
}

impl Impl {
    pub fn fma_array(&self) -> Symbol<'_, FmaArrayFn> {
        unsafe { self.lib.get(b"fma_array\0") }
            .unwrap_or_else(|e| panic!("{}: missing symbol `fma_array`: {e}", self.name))
    }
    pub fn call_fma(&self) -> Symbol<'_, CallFmaFn> {
        unsafe { self.lib.get(b"call_fma\0") }
            .unwrap_or_else(|e| panic!("{}: missing symbol `call_fma`: {e}", self.name))
    }
    pub fn driver(&self) -> Symbol<'_, DriverFn> {
        unsafe { self.lib.get(b"driver\0") }
            .unwrap_or_else(|e| panic!("{}: missing symbol `driver`: {e}", self.name))
    }
}

pub struct Libs {
    pub c: Impl,
    pub rust: Impl,
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// `<manifest>/../c_src/build/libdriver.so`, overridable with `DRIVER_C_SO`.
pub fn c_so_path() -> PathBuf {
    match std::env::var_os("DRIVER_C_SO") {
        Some(p) => PathBuf::from(p),
        None => manifest_dir().join("../c_src/build/libdriver.so"),
    }
}

/// The Rust cdylib under test, overridable with `DRIVER_RUST_SO` (used to run
/// the same suite against both the debug and the release `.so`).
///
/// `cargo test` alone does NOT emit the cdylib artifact for a `crate-type =
/// ["cdylib"]` package, so `cargo build` must have run first; `./run_tests.sh`
/// does that.
pub fn rust_so_path() -> PathBuf {
    if let Some(p) = std::env::var_os("DRIVER_RUST_SO") {
        let p = PathBuf::from(p);
        assert!(p.exists(), "DRIVER_RUST_SO={} does not exist", p.display());
        return p;
    }
    let exe = std::env::current_exe().expect("current_exe");
    let deps = exe.parent().expect("deps dir");
    let profile = deps.parent().expect("profile dir");
    let target = profile.parent().unwrap_or(profile);
    for dir in [profile, deps, &target.join("debug"), &target.join("release")] {
        let p = dir.join("libdriver.so");
        if p.exists() {
            return p;
        }
    }
    panic!(
        "could not find libdriver.so near {}; run `cargo build` first \
         (cargo test does not emit cdylib artifacts on its own)",
        deps.display()
    );
}

pub fn libs() -> &'static Libs {
    static LIBS: OnceLock<Libs> = OnceLock::new();
    LIBS.get_or_init(|| {
        let cp = c_so_path();
        let rp = rust_so_path();
        assert!(
            cp.exists(),
            "C shared library not found at {}\nBuild it with:\n  cd c_src && mkdir -p build && \
             cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            cp.display()
        );
        let c = unsafe { Library::new(&cp) }.expect("dlopen C libdriver.so");
        let rust = unsafe { Library::new(&rp) }.expect("dlopen Rust libdriver.so");
        Libs {
            c: Impl { name: "C", path: cp, lib: c },
            rust: Impl { name: "Rust", path: rp, lib: rust },
        }
    })
}

// ---------------------------------------------------------------------------
// stdout capture (for `driver`, whose only observable output is `printf`)
// ---------------------------------------------------------------------------

unsafe extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
}

fn capture_lock() -> &'static Mutex<()> {
    static L: OnceLock<Mutex<()>> = OnceLock::new();
    L.get_or_init(|| Mutex::new(()))
}

/// Redirect file descriptor 1 to a temporary file, run `f`, flush every stdio
/// stream (`fflush(NULL)`), restore fd 1 and return the captured bytes.
///
/// Serialised process-wide: fd 1 is global state.
pub fn capture_stdout<F: FnOnce()>(f: F) -> Vec<u8> {
    use std::os::unix::io::AsRawFd;
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEQ: AtomicU64 = AtomicU64::new(0);

    let _guard = capture_lock().lock().unwrap_or_else(|e| e.into_inner());

    let mut path = std::env::temp_dir();
    path.push(format!(
        "driver_stdout_{}_{}.txt",
        std::process::id(),
        SEQ.fetch_add(1, Ordering::Relaxed)
    ));

    let file = std::fs::File::options()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(&path)
        .expect("create capture file");
    let fd = file.as_raw_fd();

    let saved;
    unsafe {
        // Flush anything already pending so it does not land in our capture.
        fflush(std::ptr::null_mut());
        saved = dup(1);
        assert!(saved >= 0, "dup(1) failed");
        assert!(dup2(fd, 1) >= 0, "dup2 failed");
    }

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));

    unsafe {
        fflush(std::ptr::null_mut());
        dup2(saved, 1);
        close(saved);
    }
    drop(file);

    let data = std::fs::read(&path).expect("read capture file");
    let _ = std::fs::remove_file(&path);

    if let Err(p) = result {
        std::panic::resume_unwind(p);
    }
    data
}

/// Call `driver(cstr)` on both implementations and return `(c_stdout, rust_stdout)`.
///
/// `input` must not be re-terminated by the caller; a trailing NUL is appended
/// here. Interior NULs are preserved so the `sscanf` truncation behaviour of
/// row 13 in ERRORS.md can be exercised.
pub fn run_driver_both(input: &[u8]) -> (Vec<u8>, Vec<u8>) {
    let mut buf = input.to_vec();
    buf.push(0);
    let l = libs();
    let c_fn = l.c.driver();
    let r_fn = l.rust.driver();
    let c_out = capture_stdout(|| unsafe { c_fn(buf.as_ptr() as *const c_char) });
    let r_out = capture_stdout(|| unsafe { r_fn(buf.as_ptr() as *const c_char) });
    (c_out, r_out)
}

/// Assert that both `.so`s print byte-identical stdout for `input`.
#[track_caller]
pub fn assert_driver_same(input: &[u8], row: &str) {
    let (c_out, r_out) = run_driver_both(input);
    assert_eq!(
        c_out,
        r_out,
        "[{row}] driver stdout diverged\n  input : {:?}\n  C     : {:?}\n  Rust  : {:?}",
        String::from_utf8_lossy(input),
        String::from_utf8_lossy(&c_out),
        String::from_utf8_lossy(&r_out),
    );
}

/// Assert that both `.so`s return the same value from `call_fma`.
#[track_caller]
pub fn assert_call_fma_same(data: &[c_int], len: c_int, row: &str) -> c_int {
    let l = libs();
    let c_fn = l.c.call_fma();
    let r_fn = l.rust.call_fma();
    let cv = unsafe { c_fn(data.as_ptr(), len) };
    let rv = unsafe { r_fn(data.as_ptr(), len) };
    assert_eq!(
        cv, rv,
        "[{row}] call_fma diverged: len={len} data={:?}\n  C={cv} Rust={rv}",
        &data[..data.len().min(24)]
    );
    cv
}

/// Poison value written into the `out` buffer before calling `fma_array`, so a
/// write past `len` is detectable.
pub const POISON: c_int = 0x5A5A_5A5A;

/// Call `fma_array` on both `.so`s with identically poisoned `out` buffers and
/// assert the FULL output buffers (including the untouched tail) match.
#[track_caller]
pub fn assert_fma_array_same(
    mul1: &[c_int],
    mul2: &[c_int],
    add: &[c_int],
    len: c_int,
    out_cap: usize,
    row: &str,
) -> Vec<c_int> {
    let l = libs();
    let c_fn = l.c.fma_array();
    let r_fn = l.rust.fma_array();

    let mut c_out = vec![POISON; out_cap];
    let mut r_out = vec![POISON; out_cap];
    unsafe {
        c_fn(c_out.as_mut_ptr(), mul1.as_ptr(), mul2.as_ptr(), add.as_ptr(), len);
        r_fn(r_out.as_mut_ptr(), mul1.as_ptr(), mul2.as_ptr(), add.as_ptr(), len);
    }
    assert_eq!(
        c_out, r_out,
        "[{row}] fma_array diverged: len={len} out_cap={out_cap}\n  mul1={:?}\n  mul2={:?}\n  \
         add ={:?}",
        &mul1[..mul1.len().min(16)],
        &mul2[..mul2.len().min(16)],
        &add[..add.len().min(16)],
    );
    // Every element at or past `len` must still hold the poison in both.
    if len > 0 {
        for (i, v) in c_out.iter().enumerate().skip(len as usize) {
            assert_eq!(*v, POISON, "[{row}] fma_array wrote past len at index {i}");
        }
    }
    c_out
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (xorshift64*) -- fixed seed for reproducible property tests
// ---------------------------------------------------------------------------

pub const SEED: u64 = 0x5EED_C0DE_5EED_C0DE;

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
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
    /// Full-range random `i32`.
    pub fn i32_any(&mut self) -> c_int {
        self.next_u32() as i32
    }
    /// Random `i32` in `-bound..=bound`.
    pub fn i32_small(&mut self, bound: i32) -> c_int {
        let span = (bound as i64) * 2 + 1;
        ((self.next_u64() % span as u64) as i64 - bound as i64) as i32
    }
    /// Uniform in `lo..=hi`.
    pub fn range(&mut self, lo: usize, hi: usize) -> usize {
        lo + (self.next_u64() % ((hi - lo + 1) as u64)) as usize
    }
    pub fn pick<T: Copy>(&mut self, xs: &[T]) -> T {
        xs[(self.next_u64() % xs.len() as u64) as usize]
    }
    pub fn vec_any(&mut self, n: usize) -> Vec<c_int> {
        (0..n).map(|_| self.i32_any()).collect()
    }
    pub fn vec_small(&mut self, n: usize, bound: i32) -> Vec<c_int> {
        (0..n).map(|_| self.i32_small(bound)).collect()
    }
}

/// Interesting `i32` values, incl. the multiplication-overflow boundary
/// (46341 * 46341 > i32::MAX) and the additive boundaries.
pub const EXTREMES: &[c_int] = &[
    i32::MIN,
    i32::MIN + 1,
    -65536,
    -46341,
    -46340,
    -2,
    -1,
    0,
    1,
    2,
    46340,
    46341,
    65536,
    i32::MAX - 1,
    i32::MAX,
];

/// Whitespace characters `%d` skips over.
pub const WS: &[u8] = b" \t\n\r\x0b\x0c";
