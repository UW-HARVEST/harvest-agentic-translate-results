//! Shared differential-test harness.
//!
//! Loads BOTH shared libraries through `libloading` and calls only their
//! exported `extern "C"` symbols — the Rust implementation is never called
//! directly, so the `#[no_mangle]` wrappers and the C ABI are under test too.
//!
//! stdout is captured by `dup2`-ing a temp file over fd 1 around each call and
//! `fflush(NULL)`-ing, which works for both libraries because they share the
//! process's libc `stdout` FILE.

#![allow(dead_code)]
#![allow(non_camel_case_types)]

use libloading::{Library, Symbol};
use std::ffi::{c_char, c_double, c_int, c_void, CString};
use std::os::unix::io::AsRawFd;
use std::path::PathBuf;
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// house_t — must match the C `typedef struct { int; int; double; } house_t;`
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct house_t {
    pub floors: c_int,
    pub bedrooms: c_int,
    pub bathrooms: c_double,
}

impl house_t {
    pub fn new(floors: i32, bedrooms: i32, bathrooms: f64) -> Self {
        house_t {
            floors,
            bedrooms,
            bathrooms,
        }
    }
    /// Byte-exact comparison key (16 bytes, no padding on x86-64 SysV).
    pub fn key(&self) -> (i32, i32, u64) {
        (self.floors, self.bedrooms, self.bathrooms.to_bits())
    }
    pub fn show(&self) -> String {
        format!(
            "house_t {{ floors: {}, bedrooms: {}, bathrooms: {:?} (bits {:#018x}) }}",
            self.floors,
            self.bedrooms,
            self.bathrooms,
            self.bathrooms.to_bits()
        )
    }
}

pub const DEFAULT_HOUSE: house_t = house_t {
    floors: 2,
    bedrooms: 5,
    bathrooms: 2.5,
};

// ---------------------------------------------------------------------------
// libc bits needed for stdout capture
// ---------------------------------------------------------------------------

extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
}

// ---------------------------------------------------------------------------
// Library discovery
// ---------------------------------------------------------------------------

fn workspace_root() -> PathBuf {
    // translation/ -> ..
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ has a parent")
        .to_path_buf()
}

pub fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("DRIVER_C_SO") {
        return PathBuf::from(p);
    }
    let p = workspace_root().join("c_src/build/libdriver.so");
    assert!(
        p.exists(),
        "C shared library not found at {p:?}. Build it with:\n  cd c_src && mkdir -p build && \
         cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build ."
    );
    p
}

pub fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("DRIVER_RUST_SO") {
        return PathBuf::from(p);
    }
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    // Prefer the profile the tests were built with, then fall back.
    let mut candidates = vec![];
    // The test executable lives in target/<profile>/deps/, so derive the
    // profile directory from it when possible.
    if let Ok(exe) = std::env::current_exe() {
        if let Some(deps) = exe.parent() {
            if let Some(profile_dir) = deps.parent() {
                candidates.push(profile_dir.join("libdriver.so"));
            }
        }
    }
    candidates.push(root.join("target/debug/libdriver.so"));
    candidates.push(root.join("target/release/libdriver.so"));
    for c in &candidates {
        if c.exists() {
            return c.clone();
        }
    }
    panic!("Rust cdylib not found; looked in {candidates:?}. Run `cargo build` first.");
}

// ---------------------------------------------------------------------------
// Loaded libraries (leaked so symbols stay valid for the whole test binary)
// ---------------------------------------------------------------------------

pub struct Impl {
    pub name: &'static str,
    lib: &'static Library,
}

impl Impl {
    fn load(name: &'static str, path: PathBuf) -> Impl {
        let lib = unsafe { Library::new(&path) }
            .unwrap_or_else(|e| panic!("failed to dlopen {name} at {path:?}: {e}"));
        Impl {
            name,
            lib: Box::leak(Box::new(lib)),
        }
    }

    fn driver_sym(&self) -> Symbol<'static, unsafe extern "C" fn(*const c_char)> {
        unsafe { self.lib.get(b"driver\0") }
            .unwrap_or_else(|e| panic!("{}: missing exported symbol `driver`: {e}", self.name))
    }

    fn run_sym(&self) -> Symbol<'static, unsafe extern "C" fn(*mut house_t, c_int)> {
        unsafe { self.lib.get(b"run\0") }
            .unwrap_or_else(|e| panic!("{}: missing exported symbol `run`: {e}", self.name))
    }

    /// `driver(in)` with stdout captured.
    pub fn driver_bytes(&self, input: &[u8]) -> Vec<u8> {
        let cs = CString::new(input).expect("input must not contain an interior NUL");
        let f = self.driver_sym();
        capture_stdout(|| unsafe { f(cs.as_ptr()) })
    }

    /// `driver(in)` where `in` may contain interior NUL bytes (row 13).
    pub fn driver_raw(&self, nul_terminated: &[u8]) -> Vec<u8> {
        assert_eq!(
            nul_terminated.last(),
            Some(&0u8),
            "driver_raw needs an explicit trailing NUL"
        );
        let f = self.driver_sym();
        capture_stdout(|| unsafe { f(nul_terminated.as_ptr() as *const c_char) })
    }

    /// `run(&mut house, extra)` with stdout captured; returns (stdout, mutated house).
    pub fn run_once(&self, house: house_t, extra: i32) -> (Vec<u8>, house_t) {
        self.run_n(house, extra, 1)
    }

    /// `run` called `n` times in a row on the same house (state carry-over).
    pub fn run_n(&self, house: house_t, extra: i32, n: usize) -> (Vec<u8>, house_t) {
        let mut h = house;
        let f = self.run_sym();
        let out = capture_stdout(|| {
            for _ in 0..n {
                unsafe { f(&mut h as *mut house_t, extra) }
            }
        });
        (out, h)
    }

    /// Raw `driver` call with no stdout capture (used by the crash children).
    pub fn driver_uncaptured_ptr(&self, p: *const c_char) {
        let f = self.driver_sym();
        unsafe { f(p) }
    }

    /// Raw `run` call with no stdout capture (used by the crash children).
    pub fn run_uncaptured_ptr(&self, p: *mut house_t, extra: i32) {
        let f = self.run_sym();
        unsafe { f(p, extra) }
    }
}

static C_IMPL: OnceLock<Impl> = OnceLock::new();
static RUST_IMPL: OnceLock<Impl> = OnceLock::new();

pub fn c_impl() -> &'static Impl {
    C_IMPL.get_or_init(|| Impl::load("C", c_so_path()))
}

pub fn rust_impl() -> &'static Impl {
    RUST_IMPL.get_or_init(|| Impl::load("Rust", rust_so_path()))
}

// ---------------------------------------------------------------------------
// stdout capture
// ---------------------------------------------------------------------------

/// Serialises stdout redirection: the whole process shares fd 1.
static CAPTURE_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

pub fn capture_stdout<F: FnOnce()>(body: F) -> Vec<u8> {
    let _guard = CAPTURE_LOCK.lock().unwrap_or_else(|p| p.into_inner());

    let dir = std::env::temp_dir();
    static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let path = dir.join(format!("driver_difftest_{}_{}.out", std::process::id(), n));
    let file = std::fs::File::create(&path).expect("create capture file");
    let file_fd = file.as_raw_fd();

    unsafe {
        fflush(std::ptr::null_mut()); // flush everything pending first
        let saved = dup(1);
        assert!(saved >= 0, "dup(1) failed");
        assert!(dup2(file_fd, 1) >= 0, "dup2 onto stdout failed");

        body();

        fflush(std::ptr::null_mut());
        assert!(dup2(saved, 1) >= 0, "restoring stdout failed");
        close(saved);
    }
    drop(file);
    let bytes = std::fs::read(&path).expect("read capture file");
    let _ = std::fs::remove_file(&path);
    bytes
}

// ---------------------------------------------------------------------------
// Assertions
// ---------------------------------------------------------------------------

fn show(bytes: &[u8]) -> String {
    match std::str::from_utf8(bytes) {
        Ok(s) => format!("{s:?}"),
        Err(_) => format!("{bytes:x?}"),
    }
}

/// Differential check for `driver`.
pub fn diff_driver(input: &[u8]) {
    let c = c_impl().driver_bytes(input);
    let r = rust_impl().driver_bytes(input);
    assert!(
        c == r,
        "driver({}) stdout mismatch\n  C   ({} bytes): {}\n  Rust({} bytes): {}",
        show(input),
        c.len(),
        show(&c),
        r.len(),
        show(&r)
    );
}

/// Differential check for `run` (stdout AND the mutated struct).
pub fn diff_run(house: house_t, extra: i32) {
    diff_run_n(house, extra, 1)
}

pub fn diff_run_n(house: house_t, extra: i32, n: usize) {
    let (co, ch) = c_impl().run_n(house, extra, n);
    let (ro, rh) = rust_impl().run_n(house, extra, n);
    assert!(
        co == ro,
        "run({}, extra={extra}, n={n}) stdout mismatch\n  C   ({} bytes): {}\n  Rust({} bytes): {}",
        house.show(),
        co.len(),
        show(&co),
        ro.len(),
        show(&ro)
    );
    assert!(
        ch.key() == rh.key(),
        "run({}, extra={extra}, n={n}) mutated-struct mismatch\n  C   : {}\n  Rust: {}",
        house.show(),
        ch.show(),
        rh.show()
    );
}

/// Assert that `driver` produced exactly the C rejection message.
pub fn diff_driver_rejects(input: &[u8]) {
    diff_driver(input);
    let c = c_impl().driver_bytes(input);
    assert_eq!(
        c,
        b"An error occurred\n".to_vec(),
        "expected the rejection sentinel for {}, got {}",
        show(input),
        show(&c)
    );
}

/// Assert that `driver` accepted the input (8 house lines: 4 per `run`, twice).
pub fn diff_driver_accepts(input: &[u8]) {
    diff_driver(input);
    let c = c_impl().driver_bytes(input);
    assert_ne!(
        c,
        b"An error occurred\n".to_vec(),
        "expected acceptance for {}",
        show(input)
    );
    assert_eq!(
        c.iter().filter(|&&b| b == b'\n').count(),
        8,
        "expected 8 lines (2 x run x 4 print_house) for {}, got {}",
        show(input),
        show(&c)
    );
}

// ---------------------------------------------------------------------------
// Deterministic RNG (SplitMix64) — fixed seed for reproducibility
// ---------------------------------------------------------------------------

pub const SEED: u64 = 0x5EED_1234_ABCD_0001;

pub struct Rng(u64);

impl Rng {
    pub fn new(stream: u64) -> Rng {
        Rng(SEED ^ stream.wrapping_mul(0x9E37_79B9_7F4A_7C15))
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
    /// Uniform in `0..n`.
    pub fn below(&mut self, n: u64) -> u64 {
        self.next_u64() % n
    }
    /// A finite double spanning many magnitudes.
    pub fn next_finite_f64(&mut self) -> f64 {
        loop {
            let bits = self.next_u64();
            let v = f64::from_bits(bits);
            if v.is_finite() {
                return v;
            }
        }
    }
    /// A "plausible" bathroom count: small multiples of 0.5 plus noise.
    pub fn next_bathrooms(&mut self) -> f64 {
        match self.below(4) {
            0 => (self.below(40) as f64) * 0.5,
            1 => (self.next_i32() as f64) / 8.0,
            2 => self.next_finite_f64(),
            _ => {
                let m = self.below(21) as i32 - 10;
                (self.next_u32() % 1000) as f64 * 10f64.powi(m)
            }
        }
    }
}
