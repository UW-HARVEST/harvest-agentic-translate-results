//! Shared differential-test harness.
//!
//! BOTH implementations are loaded as shared objects through `libloading` and
//! called only through their exported C ABI symbols — the Rust functions are
//! never called directly, so the `#[no_mangle] extern "C"` wrappers are part of
//! what is under test.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::os::raw::{c_char, c_int, c_void};
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, OnceLock};

pub type GotomachFn = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;
pub type OpFn = unsafe extern "C" fn(c_int, c_int, *mut c_void) -> c_int;

/// The four public symbols exported by both `.so`s.
pub const PUBLIC_SYMBOLS: [&str; 4] = ["gotomach", "process_value", "double_value", "triple_value"];

pub struct Impl {
    pub name: &'static str,
    pub path: PathBuf,
    // Keep the library alive for the whole process lifetime.
    _lib: &'static Library,
    pub gotomach: GotomachFn,
    pub process_value: OpFn,
    pub double_value: OpFn,
    pub triple_value: OpFn,
}

impl Impl {
    pub fn op(&self, mode: c_int) -> OpFn {
        // Mirrors the `switch (mode)` selection in gotomach().
        match mode {
            0 => self.process_value,
            1 => self.double_value,
            2 => self.triple_value,
            _ => self.process_value,
        }
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

pub fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("C_SO_PATH") {
        return PathBuf::from(p);
    }
    let p = manifest_dir().join("c_src/build/libtranslated_rust.so");
    assert!(
        p.exists(),
        "C shared library not found at {}.\nBuild it with:\n  cd c_src && mkdir -p build && cd build \\\n    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        p.display()
    );
    p
}

pub fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_SO_PATH") {
        return PathBuf::from(p);
    }
    // The test binary lives in <target>/<profile>/deps/, so the cdylib produced
    // for the very same profile is one directory up — prefer it, so the .so
    // under test always matches the profile/features `cargo test` was run with.
    let mut cands: Vec<PathBuf> = Vec::new();
    if let Ok(exe) = std::env::current_exe() {
        if let Some(deps) = exe.parent() {
            if let Some(profile) = deps.parent() {
                cands.push(profile.join("libgotomach_lib.so"));
            }
        }
    }
    cands.push(manifest_dir().join("target/debug/libgotomach_lib.so"));
    cands.push(manifest_dir().join("target/release/libgotomach_lib.so"));

    for c in &cands {
        if c.exists() {
            return c.clone();
        }
    }
    panic!(
        "Rust cdylib libgotomach_lib.so not found. `cargo test` does NOT build \
         cdylib artifacts, so run `cargo build` (same profile/features) first, or \
         set RUST_SO_PATH.\nsearched: {cands:#?}"
    )
}

unsafe fn load(name: &'static str, path: PathBuf) -> Impl {
    let lib: &'static Library = Box::leak(Box::new(
        Library::new(&path).unwrap_or_else(|e| panic!("failed to dlopen {}: {e}", path.display())),
    ));
    let get_op = |sym: &str| -> OpFn {
        let s: Symbol<OpFn> = lib
            .get(format!("{sym}\0").as_bytes())
            .unwrap_or_else(|e| panic!("{name}: missing symbol {sym}: {e}"));
        *s
    };
    let gotomach: GotomachFn = {
        let s: Symbol<GotomachFn> = lib
            .get(b"gotomach\0")
            .unwrap_or_else(|e| panic!("{name}: missing symbol gotomach: {e}"));
        *s
    };
    Impl {
        name,
        path,
        _lib: lib,
        gotomach,
        process_value: get_op("process_value"),
        double_value: get_op("double_value"),
        triple_value: get_op("triple_value"),
    }
}

pub fn c_impl() -> &'static Impl {
    static I: OnceLock<Impl> = OnceLock::new();
    I.get_or_init(|| unsafe { load("C", c_so_path()) })
}

pub fn rust_impl() -> &'static Impl {
    static I: OnceLock<Impl> = OnceLock::new();
    I.get_or_init(|| unsafe { load("Rust", rust_so_path()) })
}

// ---------------------------------------------------------------------------
// stdout capture
//
// Both libraries write with libc `printf`/`puts` straight to fd 1, bypassing
// Rust's own test harness capture, so we redirect fd 1 to a temp file.
// A process-wide mutex serialises capture across parallel test threads.
// ---------------------------------------------------------------------------
extern "C" {
    fn dup(fd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
}

fn capture_lock() -> MutexGuard<'static, ()> {
    static L: OnceLock<Mutex<()>> = OnceLock::new();
    L.get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}

/// Runs `f` with fd 1 redirected to a temp file and returns `(f's value, bytes
/// written to stdout)`.
pub fn capture_stdout<T, F: FnOnce() -> T>(f: F) -> (T, Vec<u8>) {
    use std::io::{Read, Seek, SeekFrom};
    use std::os::unix::io::AsRawFd;

    let _guard = capture_lock();

    let mut tmp_path = std::env::temp_dir();
    tmp_path.push(format!(
        "gotomach_capture_{}_{:?}.txt",
        std::process::id(),
        std::thread::current().id()
    ));
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(true)
        .open(&tmp_path)
        .expect("create capture temp file");

    let out = unsafe {
        fflush(std::ptr::null_mut()); // flush every open stream (incl. stdout)
        let saved = dup(1);
        assert!(saved >= 0, "dup(1) failed");
        assert!(dup2(file.as_raw_fd(), 1) >= 0, "dup2 failed");

        let value = f();

        fflush(std::ptr::null_mut());
        assert!(dup2(saved, 1) >= 0, "dup2 restore failed");
        close(saved);
        value
    };

    file.seek(SeekFrom::Start(0)).unwrap();
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes).unwrap();
    drop(file);
    let _ = std::fs::remove_file(&tmp_path);
    (out, bytes)
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (xorshift64*) — fixed seeds keep runs reproducible.
// ---------------------------------------------------------------------------
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(if seed == 0 { 0x9E3779B97F4A7C15 } else { seed })
    }
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545F4914F6CDD1D)
    }
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    pub fn next_i32(&mut self) -> i32 {
        self.next_u32() as i32
    }
    /// Uniform in `[lo, hi]` inclusive (`i64` math, so `i32::MIN..=i32::MAX` works).
    pub fn range(&mut self, lo: i64, hi: i64) -> i64 {
        assert!(hi >= lo);
        let span = (hi - lo) as u64 + 1;
        if span == 0 {
            return self.next_u64() as i64;
        }
        lo + (self.next_u64() % span) as i64
    }
    pub fn range_i32(&mut self, lo: i32, hi: i32) -> i32 {
        self.range(lo as i64, hi as i64) as i32
    }
}

// ---------------------------------------------------------------------------
// Differential drivers
// ---------------------------------------------------------------------------

/// One `gotomach` argument tuple.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Args {
    pub iterations: c_int,
    pub seed: c_int,
    pub mode: c_int,
    pub threshold: c_int,
}

impl Args {
    pub fn new(iterations: c_int, seed: c_int, mode: c_int, threshold: c_int) -> Self {
        Args {
            iterations,
            seed,
            mode,
            threshold,
        }
    }
}

/// Calls `gotomach` in BOTH `.so`s for every tuple, comparing the returned
/// `int` for each tuple and the complete stdout byte stream of the whole batch.
pub fn diff_gotomach_batch(row: &str, inputs: &[Args]) {
    let c = c_impl();
    let r = rust_impl();

    let (c_ret, c_out) = capture_stdout(|| {
        inputs
            .iter()
            .map(|a| unsafe { (c.gotomach)(a.iterations, a.seed, a.mode, a.threshold) })
            .collect::<Vec<c_int>>()
    });
    let (r_ret, r_out) = capture_stdout(|| {
        inputs
            .iter()
            .map(|a| unsafe { (r.gotomach)(a.iterations, a.seed, a.mode, a.threshold) })
            .collect::<Vec<c_int>>()
    });

    assert_eq!(c_ret.len(), inputs.len());
    // Guard against a vacuous comparison: gotomach always logs at least the
    // "[INFO] Starting gotomach function" line, so an empty capture would mean
    // the stdout redirection silently failed.
    if !inputs.is_empty() {
        assert!(
            !c_out.is_empty(),
            "[{row}] stdout capture produced 0 bytes for {} inputs - capture is broken",
            inputs.len()
        );
    }
    for (i, a) in inputs.iter().enumerate() {
        assert_eq!(
            c_ret[i], r_ret[i],
            "[{row}] return mismatch for gotomach({}, {}, {}, {}): C={} Rust={}",
            a.iterations, a.seed, a.mode, a.threshold, c_ret[i], r_ret[i]
        );
    }
    if c_out != r_out {
        let first = c_out
            .iter()
            .zip(r_out.iter())
            .position(|(a, b)| a != b)
            .unwrap_or(c_out.len().min(r_out.len()));
        let lo = first.saturating_sub(120);
        panic!(
            "[{row}] stdout mismatch (C {} bytes, Rust {} bytes), first difference at byte {first}\n  C   ...{:?}\n  Rust...{:?}",
            c_out.len(),
            r_out.len(),
            String::from_utf8_lossy(&c_out[lo..(first + 120).min(c_out.len())]),
            String::from_utf8_lossy(&r_out[lo..(first + 120).min(r_out.len())]),
        );
    }
}

/// Differential test of one of the three exported `operation_fn`s.
pub fn diff_op_batch(row: &str, which: &str, inputs: &[(c_int, c_int, usize)]) {
    let c = c_impl();
    let r = rust_impl();
    let pick = |im: &Impl| -> OpFn {
        match which {
            "process_value" => im.process_value,
            "double_value" => im.double_value,
            "triple_value" => im.triple_value,
            _ => unreachable!(),
        }
    };
    let cf = pick(c);
    let rf = pick(r);
    let (c_ret, c_out) = capture_stdout(|| {
        inputs
            .iter()
            .map(|&(v, p, ctx)| unsafe { cf(v, p, ctx as *mut c_void) })
            .collect::<Vec<c_int>>()
    });
    let (r_ret, r_out) = capture_stdout(|| {
        inputs
            .iter()
            .map(|&(v, p, ctx)| unsafe { rf(v, p, ctx as *mut c_void) })
            .collect::<Vec<c_int>>()
    });
    for (i, &(v, p, ctx)) in inputs.iter().enumerate() {
        assert_eq!(
            c_ret[i], r_ret[i],
            "[{row}] {which}({v}, {p}, {ctx:#x}) mismatch: C={} Rust={}",
            c_ret[i], r_ret[i]
        );
    }
    assert_eq!(
        c_out, r_out,
        "[{row}] {which} must not print anything; C wrote {} bytes, Rust {} bytes",
        c_out.len(),
        r_out.len()
    );
}

/// Byte-compare helper used by the error-path tests: returns
/// `(return value, stdout bytes)` of a single `gotomach` call for one impl.
pub fn call_gotomach(im: &Impl, a: Args) -> (c_int, Vec<u8>) {
    capture_stdout(|| unsafe { (im.gotomach)(a.iterations, a.seed, a.mode, a.threshold) })
}

/// Asserts both impls return `expected` for `a`, and that their logs match.
pub fn assert_error_row(row: &str, a: Args, expected: c_int, expect_log_contains: &[&str]) {
    let (c_ret, c_out) = call_gotomach(c_impl(), a);
    let (r_ret, r_out) = call_gotomach(rust_impl(), a);
    assert_eq!(
        c_ret, expected,
        "[{row}] C returned {c_ret}, table says {expected} for {a:?}"
    );
    assert_eq!(
        r_ret, c_ret,
        "[{row}] Rust returned {r_ret}, C returned {c_ret} for {a:?}"
    );
    assert_eq!(
        String::from_utf8_lossy(&c_out),
        String::from_utf8_lossy(&r_out),
        "[{row}] log mismatch for {a:?}"
    );
    let text = String::from_utf8_lossy(&c_out).to_string();
    for needle in expect_log_contains {
        assert!(
            text.contains(needle),
            "[{row}] expected log to contain {needle:?}, got {text:?}"
        );
    }
}

/// Formats a NUL-terminated C string pointer (used by the symbol tests).
pub fn cstr(p: *const c_char) -> String {
    unsafe { std::ffi::CStr::from_ptr(p).to_string_lossy().to_string() }
}
