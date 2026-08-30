//! Shared harness for the C-vs-Rust differential tests.
//!
//! BOTH implementations are loaded as shared objects through `libloading` and
//! called through their exported `driver` symbol — the Rust functions are never
//! called directly, so the `#[no_mangle] extern "C"` wrapper is under test too.
//!
//! `driver` returns `void` and communicates entirely through `printf`, so the
//! observable output is the byte stream it writes to the C runtime's `stdout`.
//! We capture it by temporarily pointing file descriptor 1 at a scratch file.

#![allow(dead_code)]

use std::ffi::{c_char, c_int, c_void};
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, OnceLock};

use libloading::{Library, Symbol};

unsafe extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    /// `fflush(NULL)` flushes every open C output stream.
    fn fflush(stream: *mut c_void) -> c_int;
    fn setlocale(category: c_int, locale: *const c_char) -> *mut c_char;
}

/// glibc `LC_ALL`.
pub const LC_ALL: c_int = 6;

/// fd 1 redirection is process-global state, so captures must be serialized.
fn capture_lock() -> MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    match LOCK.get_or_init(|| Mutex::new(())).lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_so_path() -> PathBuf {
    let p = manifest_dir().join("../c_src/build/libdriver.so");
    assert!(
        p.exists(),
        "C shared library not found at {p:?}; build it with:\n  \
         cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build ."
    );
    p
}

fn rust_so_path() -> PathBuf {
    // `DRIVER_RUST_SO` lets the runner script point the same test binary at a
    // specific artifact (e.g. the release cdylib).
    if let Some(p) = std::env::var_os("DRIVER_RUST_SO") {
        let p = PathBuf::from(p);
        assert!(p.exists(), "DRIVER_RUST_SO={p:?} does not exist");
        return p;
    }
    // Otherwise use the cdylib of the profile this test binary was built in.
    // The test binary lives in target/<profile>/deps/, so the cdylib is two
    // directories up.  NOTE: `cargo test` alone does not build a cdylib-only
    // lib target, so `cargo build` must have run first — we fail loudly rather
    // than silently falling back to another profile's artifact, because that
    // would mean verifying a binary nobody asked about.
    let exe = std::env::current_exe().expect("current_exe");
    let profile_dir = exe
        .parent()
        .and_then(|p| p.parent())
        .expect("target/<profile>")
        .to_path_buf();
    let p = profile_dir.join("libdriver.so");
    assert!(
        p.exists(),
        "Rust cdylib not found at {p:?}; run `cargo build` (same profile) first, \
         or set DRIVER_RUST_SO"
    );
    p
}

/// Both implementations, each behind its own `dlopen` handle.
pub struct Impls {
    _c_lib: Library,
    _rust_lib: Library,
    c_driver: Symbol<'static, unsafe extern "C" fn(c_char)>,
    rust_driver: Symbol<'static, unsafe extern "C" fn(c_char)>,
    /// The same symbols re-typed to take a full-width `int`, for testing what
    /// happens when a caller pushes a value that does not fit in a `char`.
    c_driver_wide: Symbol<'static, unsafe extern "C" fn(c_int)>,
    rust_driver_wide: Symbol<'static, unsafe extern "C" fn(c_int)>,
}

pub fn impls() -> &'static Impls {
    static IMPLS: OnceLock<Impls> = OnceLock::new();
    IMPLS.get_or_init(|| unsafe {
        let c_lib = Library::new(c_so_path()).expect("dlopen C libdriver.so");
        let rust_lib = Library::new(rust_so_path()).expect("dlopen Rust libdriver.so");

        // SAFETY: the handles are kept alive for the process lifetime inside the
        // same struct as the symbols (and the struct lives in a `OnceLock`), so
        // extending the symbol lifetimes to 'static is sound.
        let c_driver: Symbol<unsafe extern "C" fn(c_char)> =
            c_lib.get(b"driver\0").expect("C driver symbol");
        let rust_driver: Symbol<unsafe extern "C" fn(c_char)> =
            rust_lib.get(b"driver\0").expect("Rust driver symbol");
        let c_driver_wide: Symbol<unsafe extern "C" fn(c_int)> =
            c_lib.get(b"driver\0").expect("C driver symbol");
        let rust_driver_wide: Symbol<unsafe extern "C" fn(c_int)> =
            rust_lib.get(b"driver\0").expect("Rust driver symbol");

        Impls {
            c_driver: std::mem::transmute(c_driver),
            rust_driver: std::mem::transmute(rust_driver),
            c_driver_wide: std::mem::transmute(c_driver_wide),
            rust_driver_wide: std::mem::transmute(rust_driver_wide),
            _c_lib: c_lib,
            _rust_lib: rust_lib,
        }
    })
}

/// Runs `f` with fd 1 pointed at a scratch file and returns everything written.
fn capture<F: FnOnce()>(f: F) -> Vec<u8> {
    let _guard = capture_lock();
    let path = std::env::temp_dir().join(format!(
        "driver-capture-{}-{:?}.bin",
        std::process::id(),
        std::thread::current().id()
    ));
    let bytes = unsafe {
        // Keep the test harness's own buffered output out of the capture.
        let _ = std::io::stdout().flush();
        fflush(std::ptr::null_mut());

        let file = std::fs::File::create(&path).expect("create capture file");
        let fd = {
            use std::os::fd::AsRawFd;
            file.as_raw_fd()
        };
        let saved = dup(1);
        assert!(saved >= 0, "dup(1) failed");
        assert!(dup2(fd, 1) >= 0, "dup2 failed");

        f();

        fflush(std::ptr::null_mut());
        assert!(dup2(saved, 1) >= 0, "restore dup2 failed");
        close(saved);
        drop(file);
        std::fs::read(&path).expect("read capture file")
    };
    let _ = std::fs::remove_file(&path);
    bytes
}

/// Output of the C implementation for `c`.
pub fn c_out(c: c_char) -> Vec<u8> {
    let i = impls();
    capture(|| unsafe { (i.c_driver)(c) })
}

/// Output of the Rust implementation for `c`.
pub fn rust_out(c: c_char) -> Vec<u8> {
    let i = impls();
    capture(|| unsafe { (i.rust_driver)(c) })
}

/// Output of the C implementation when the argument is a full-width `int`.
pub fn c_out_wide(v: c_int) -> Vec<u8> {
    let i = impls();
    capture(|| unsafe { (i.c_driver_wide)(v) })
}

/// Output of the Rust implementation when the argument is a full-width `int`.
pub fn rust_out_wide(v: c_int) -> Vec<u8> {
    let i = impls();
    capture(|| unsafe { (i.rust_driver_wide)(v) })
}

fn render(bytes: &[u8]) -> String {
    String::from_utf8(
        bytes
            .iter()
            .flat_map(|b| std::ascii::escape_default(*b))
            .collect::<Vec<u8>>(),
    )
    .unwrap()
}

/// Asserts the two implementations agree byte-for-byte for input `c`.
#[track_caller]
pub fn assert_same(c: c_char, ctx: &str) {
    let c_bytes = c_out(c);
    let r_bytes = rust_out(c);
    assert!(
        !c_bytes.is_empty(),
        "C produced no output for c={c} ({ctx}) — capture harness is broken"
    );
    assert_eq!(
        render(&c_bytes),
        render(&r_bytes),
        "output mismatch for c={c} (byte 0x{:02x}) [{ctx}]",
        c as u8
    );
    assert_eq!(c_bytes, r_bytes, "byte mismatch for c={c} [{ctx}]");
}

/// Asserts agreement when the argument is passed as a full-width `int`.
#[track_caller]
pub fn assert_same_wide(v: c_int, ctx: &str) {
    let c_bytes = c_out_wide(v);
    let r_bytes = rust_out_wide(v);
    assert_eq!(
        render(&c_bytes),
        render(&r_bytes),
        "output mismatch for wide arg {v} (0x{v:08x}) [{ctx}]"
    );
    assert_eq!(c_bytes, r_bytes, "byte mismatch for wide arg {v} [{ctx}]");
}

/// Asserts agreement for every byte in `range`, plus `count` randomized draws
/// from that range (a fixed-seed property-style sweep).
#[track_caller]
pub fn assert_same_range(range: std::ops::RangeInclusive<u8>, count: usize, ctx: &str) {
    let lo = *range.start();
    let hi = *range.end();
    for b in lo..=hi {
        assert_same(b as c_char, ctx);
    }
    let span = (hi as u32 - lo as u32) + 1;
    let mut rng = Rng::new(0x5EED_0000 ^ ((lo as u64) << 8) ^ hi as u64);
    for _ in 0..count {
        let b = (lo as u32 + (rng.next_u32() % span)) as u8;
        assert_same(b as c_char, ctx);
    }
}

/// Deterministic xorshift64* PRNG — fixed seed, reproducible sequences.
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
    pub fn next_u8(&mut self) -> u8 {
        (self.next_u64() >> 56) as u8
    }
}

/// Forces the process into `name`'s locale; returns false if unavailable.
pub fn try_set_locale(name: &[u8]) -> bool {
    let mut z = name.to_vec();
    z.push(0);
    unsafe { !setlocale(LC_ALL, z.as_ptr() as *const c_char).is_null() }
}
