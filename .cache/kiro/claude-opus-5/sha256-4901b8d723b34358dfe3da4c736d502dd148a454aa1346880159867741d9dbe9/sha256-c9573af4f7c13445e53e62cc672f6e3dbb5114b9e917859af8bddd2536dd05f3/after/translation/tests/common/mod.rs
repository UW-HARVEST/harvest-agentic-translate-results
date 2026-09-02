//! Shared differential-test harness.
//!
//! Both libraries are loaded through `libloading` and called *only* through
//! their exported C symbols — the Rust implementation is never called directly,
//! so the `#[no_mangle]`/`extern "C"` wrappers are part of what is under test.

#![allow(dead_code)]

use libloading::Library;
use std::ffi::{CString, c_char, c_int, c_void};
use std::os::fd::AsRawFd;
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, OnceLock};

// ---------------------------------------------------------------------------
// libc bits the harness itself needs (avoids pulling in the `libc` crate).
// ---------------------------------------------------------------------------
unsafe extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn setenv(name: *const c_char, value: *const c_char, overwrite: c_int) -> c_int;
    fn unsetenv(name: *const c_char) -> c_int;
}

// ---------------------------------------------------------------------------
// Exported signatures, exactly as declared in the C.
// ---------------------------------------------------------------------------
pub type FnParseEnvNumeric = unsafe extern "C" fn(*const c_char, c_int) -> c_int;
pub type FnInitConfigFromEnv = unsafe extern "C" fn(*mut u32);
pub type FnPerformOperation = unsafe extern "C" fn(c_int, c_int, *mut u32) -> c_int;
pub type FnApplyBitOperations = unsafe extern "C" fn(c_int, *mut u32) -> c_int;
pub type FnEnvy = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

pub struct Lib {
    pub name: &'static str,
    pub path: PathBuf,
    _lib: Library,
    pub parse_env_numeric: FnParseEnvNumeric,
    pub init_config_from_env: FnInitConfigFromEnv,
    pub perform_operation: FnPerformOperation,
    pub apply_bit_operations: FnApplyBitOperations,
    pub envy: FnEnvy,
}

impl Lib {
    fn open(name: &'static str, path: PathBuf) -> Lib {
        unsafe {
            let lib = Library::new(&path)
                .unwrap_or_else(|e| panic!("dlopen({}) failed: {e}", path.display()));
            let parse_env_numeric = *lib
                .get::<FnParseEnvNumeric>(b"parse_env_numeric\0")
                .expect("parse_env_numeric");
            let init_config_from_env = *lib
                .get::<FnInitConfigFromEnv>(b"init_config_from_env\0")
                .expect("init_config_from_env");
            let perform_operation = *lib
                .get::<FnPerformOperation>(b"perform_operation\0")
                .expect("perform_operation");
            let apply_bit_operations = *lib
                .get::<FnApplyBitOperations>(b"apply_bit_operations\0")
                .expect("apply_bit_operations");
            let envy = *lib.get::<FnEnvy>(b"envy\0").expect("envy");
            Lib {
                name,
                path,
                _lib: lib,
                parse_env_numeric,
                init_config_from_env,
                perform_operation,
                apply_bit_operations,
                envy,
            }
        }
    }
}

pub fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

pub fn c_so_path() -> PathBuf {
    let dir = crate_root().parent().unwrap().join("c_src/build");
    let mut found: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| {
            panic!(
                "cannot read {} ({e}); build the C library first:\n  cd c_src && mkdir -p build \
                 && cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
                dir.display()
            )
        })
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "so"))
        .collect();
    found.sort();
    assert_eq!(
        found.len(),
        1,
        "expected exactly one .so in {}, found {found:?}",
        dir.display()
    );
    found.pop().unwrap()
}

pub fn rust_so_path() -> PathBuf {
    for profile in ["release", "debug"] {
        let p = crate_root().join("target").join(profile).join("libenvy_lib.so");
        if p.exists() {
            return p;
        }
    }
    panic!("libenvy_lib.so not found; run `cargo build --release` first");
}

/// The two libraries, loaded once. `dlopen` defaults to `RTLD_LOCAL`, so the
/// identically-named symbols in the two objects do not collide.
pub fn libs() -> &'static (Lib, Lib) {
    static LIBS: OnceLock<(Lib, Lib)> = OnceLock::new();
    LIBS.get_or_init(|| {
        (
            Lib::open("C", c_so_path()),
            Lib::open("Rust", rust_so_path()),
        )
    })
}

/// Serialises the process-global state the tests mutate: `environ`, and fds 1/2.
pub fn lock() -> MutexGuard<'static, ()> {
    static L: Mutex<()> = Mutex::new(());
    match L.lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    }
}

// ---------------------------------------------------------------------------
// Environment control. Goes straight through libc so that it is unambiguously
// the same `environ` that both libraries' `getenv` reads.
// ---------------------------------------------------------------------------
pub const ENV_VARS: [&str; 5] = [
    "PROG_VERBOSE",
    "PROG_DEBUG",
    "PROG_OPTIMIZE",
    "PROG_BASE_OFFSET",
    "PROG_MULTIPLIER",
];

pub fn env_set(name: &str, value: &str) {
    let n = CString::new(name).unwrap();
    let v = CString::new(value).unwrap();
    assert_eq!(unsafe { setenv(n.as_ptr(), v.as_ptr(), 1) }, 0);
}

pub fn env_unset(name: &str) {
    let n = CString::new(name).unwrap();
    unsafe { unsetenv(n.as_ptr()) };
}

/// `setenv` with raw C strings, for values that are not valid UTF-8.
pub unsafe fn set_env_raw(name: *const c_char, value: *const c_char) {
    assert_eq!(unsafe { setenv(name, value, 1) }, 0);
}

/// Apply a full environment description: `None` means "remove the variable".
pub fn env_apply(settings: &[(&str, Option<&str>)]) {
    for v in ENV_VARS {
        env_unset(v);
    }
    for (k, v) in settings {
        match v {
            Some(val) => env_set(k, val),
            None => env_unset(k),
        }
    }
}

pub fn env_clear_all() {
    for v in ENV_VARS {
        env_unset(v);
    }
}

// ---------------------------------------------------------------------------
// stdout / stderr capture.
//
// Both libraries write through the same `libc` `stdout`/`stderr` FILE streams,
// so `fflush(NULL)` after the batch flushes whatever either of them buffered.
// ---------------------------------------------------------------------------
fn tmp_file(tag: &str) -> PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    static N: AtomicU64 = AtomicU64::new(0);
    let n = N.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "envy_difftest_{}_{tag}_{n}.bin",
        std::process::id()
    ))
}

/// Run `f`, capturing everything it writes to fd 1 and fd 2.
///
/// Capturing around a whole *batch* of calls (rather than one call at a time)
/// keeps the tests fast and additionally pins down the *ordering* and
/// interleaving of the two streams.
pub fn capture<T>(f: impl FnOnce() -> T) -> (T, Vec<u8>, Vec<u8>) {
    let out_path = tmp_file("out");
    let err_path = tmp_file("err");

    let value;
    unsafe {
        fflush(std::ptr::null_mut()); // drain anything already pending
        let out_file = std::fs::File::create(&out_path).unwrap();
        let err_file = std::fs::File::create(&err_path).unwrap();
        let saved_out = dup(1);
        let saved_err = dup(2);
        assert!(saved_out >= 0 && saved_err >= 0, "dup failed");
        assert!(dup2(out_file.as_raw_fd(), 1) >= 0, "dup2 stdout failed");
        assert!(dup2(err_file.as_raw_fd(), 2) >= 0, "dup2 stderr failed");

        value = f();

        fflush(std::ptr::null_mut());
        dup2(saved_out, 1);
        dup2(saved_err, 2);
        close(saved_out);
        close(saved_err);
    }

    let out = std::fs::read(&out_path).unwrap_or_default();
    let err = std::fs::read(&err_path).unwrap_or_default();
    let _ = std::fs::remove_file(&out_path);
    let _ = std::fs::remove_file(&err_path);
    (value, out, err)
}

/// Like `capture`, but points fd 1 AND fd 2 at the *same* file, so the two
/// streams interleave into one byte sequence exactly as they would on a
/// terminal or in a redirected log.
///
/// This is a stronger observation than comparing the streams separately:
/// `stdout` is block-buffered when it is a file while `stderr` is unbuffered, so
/// the merged order depends on where each library's flush points fall.
pub fn capture_merged<T>(f: impl FnOnce() -> T) -> (T, Vec<u8>) {
    let path = tmp_file("merged");
    let value;
    unsafe {
        fflush(std::ptr::null_mut());
        let file = std::fs::File::create(&path).unwrap();
        let saved_out = dup(1);
        let saved_err = dup(2);
        dup2(file.as_raw_fd(), 1);
        dup2(file.as_raw_fd(), 2);
        value = f();
        fflush(std::ptr::null_mut());
        dup2(saved_out, 1);
        dup2(saved_err, 2);
        close(saved_out);
        close(saved_err);
    }
    let bytes = std::fs::read(&path).unwrap_or_default();
    let _ = std::fs::remove_file(&path);
    (value, bytes)
}

// ---------------------------------------------------------------------------
// Differential comparison helpers.
// ---------------------------------------------------------------------------

/// Result of running one batch against one library.
pub struct Batch {
    pub values: Vec<i64>,
    pub out: Vec<u8>,
    pub err: Vec<u8>,
}

/// Run the same closure against the C library and then the Rust library, and
/// assert the returned value sequences and the captured byte streams match.
pub fn diff(row: &str, body: impl Fn(&Lib) -> Vec<i64>) {
    let (c, r) = libs();
    let (vc, oc, ec) = capture(|| body(c));
    let (vr, or, er) = capture(|| body(r));

    assert_eq!(
        vc.len(),
        vr.len(),
        "[{row}] different number of observations"
    );
    if vc != vr {
        let idx = vc.iter().zip(vr.iter()).position(|(a, b)| a != b).unwrap();
        panic!(
            "[{row}] return-value divergence at observation #{idx}: C = {} ({:#x}), Rust = {} ({:#x})",
            vc[idx], vc[idx], vr[idx], vr[idx]
        );
    }
    assert_streams_eq(row, "stdout", &oc, &or);
    assert_streams_eq(row, "stderr", &ec, &er);
}

pub fn assert_streams_eq(row: &str, which: &str, c: &[u8], r: &[u8]) {
    if c == r {
        return;
    }
    let first = c
        .iter()
        .zip(r.iter())
        .position(|(a, b)| a != b)
        .unwrap_or(c.len().min(r.len()));
    let ctx = |b: &[u8]| {
        let lo = first.saturating_sub(120);
        let hi = (first + 200).min(b.len());
        String::from_utf8_lossy(&b[lo..hi]).into_owned()
    };
    panic!(
        "[{row}] {which} divergence at byte {first} (C {} bytes, Rust {} bytes)\n\
         --- C ---\n{}\n--- Rust ---\n{}\n",
        c.len(),
        r.len(),
        ctx(c),
        ctx(r)
    );
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (SplitMix64) — fixed seed, reproducible.
// ---------------------------------------------------------------------------
pub struct Rng(u64);

pub const SEED: u64 = 0x5EED_1234_ABCD_0001;

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
    /// Full-range `i32`, biased towards small magnitudes and boundaries so that
    /// both ordinary values and overflow-triggering extremes get exercised.
    pub fn next_i32(&mut self) -> i32 {
        let raw = self.next_u64();
        match raw & 0x7 {
            0 => (raw >> 32) as i32,            // full range
            1 => ((raw >> 32) as i32) >> 8,     // medium
            2 => ((raw >> 32) as i32) >> 20,    // small
            3 => (raw >> 32) as u8 as i32,      // tiny positive
            4 => -((raw >> 32) as u8 as i32),   // tiny negative
            5 => i32::MAX - ((raw >> 40) as u8 as i32), // near INT_MAX
            6 => i32::MIN + ((raw >> 40) as u8 as i32), // near INT_MIN
            _ => (raw >> 32) as i32,
        }
    }
}

/// The `int` boundary values every parameter is swept over.
pub const BOUNDS: [i32; 13] = [
    i32::MIN,
    i32::MIN + 1,
    i32::MIN / 2,
    -65536,
    -256,
    -3,
    -1,
    0,
    1,
    3,
    256,
    i32::MAX - 1,
    i32::MAX,
];

/// Build a `ConfigFlags` storage word from individual bitfield values.
///
///   bit 0 verbose, bit 1 debug, bit 2 optimize, bit 3 cache_enabled,
///   bits 4..6 log_level, bit 7 reserved, bits 8..31 padding.
pub fn flags_word(
    verbose: u32,
    debug: u32,
    optimize: u32,
    cache_enabled: u32,
    log_level: u32,
    reserved: u32,
    padding: u32,
) -> u32 {
    (verbose & 1)
        | ((debug & 1) << 1)
        | ((optimize & 1) << 2)
        | ((cache_enabled & 1) << 3)
        | ((log_level & 7) << 4)
        | ((reserved & 1) << 7)
        | (padding & 0xFFFF_FF00)
}

pub const GARBAGE_PADDING: [u32; 8] = [
    0x0000_0000,
    0xFFFF_FF00,
    0xDEAD_BE00,
    0x0000_0100,
    0x8000_0000,
    0x7FFF_FF00,
    0xA5A5_A500,
    0x1234_5600,
];

/// Run `f` with fds 1 and 2 pointed at `/dev/null`.
///
/// Used by `diff_stream`, whose interleaved C/Rust calls make the two streams
/// impossible to separate. Byte-for-byte stream equality for the chatty
/// configurations is asserted by the `diff`-based Phase B and Phase C rows.
pub fn silenced<T>(f: impl FnOnce() -> T) -> T {
    unsafe {
        fflush(std::ptr::null_mut());
        let devnull = std::fs::File::create("/dev/null").unwrap();
        let saved_out = dup(1);
        let saved_err = dup(2);
        dup2(devnull.as_raw_fd(), 1);
        dup2(devnull.as_raw_fd(), 2);
        let v = f();
        fflush(std::ptr::null_mut());
        dup2(saved_out, 1);
        dup2(saved_err, 2);
        close(saved_out);
        close(saved_err);
        v
    }
}

/// Streaming differential comparison, for sweeps far too large to buffer.
///
/// Calls C and Rust back-to-back for each input and compares return values
/// immediately, so nothing accumulates and the input space can be swept much
/// more densely than `diff` allows. Returns the number of inputs swept so the
/// caller can assert the sweep really was the size it intended.
#[must_use]
pub fn diff_stream<I, T>(row: &str, inputs: I, call: impl Fn(&Lib, &T) -> i64) -> u64
where
    I: IntoIterator<Item = T>,
    T: std::fmt::Debug,
{
    let (c, r) = libs();
    let mut n: u64 = 0;
    let bad = silenced(|| {
        for input in inputs {
            let vc = call(c, &input);
            let vr = call(r, &input);
            if vc != vr {
                return Some((format!("{input:?}"), n, vc, vr));
            }
            n += 1;
        }
        None
    });
    if let Some((input, idx, vc, vr)) = bad {
        panic!(
            "[{row}] divergence on input {input} (observation #{idx}): \
             C = {vc} ({vc:#x}), Rust = {vr} ({vr:#x})"
        );
    }
    assert!(n > 0, "[{row}] swept zero inputs");
    eprintln!("[{row}] swept {n} inputs ({} FFI calls)", n * 2);
    n
}
