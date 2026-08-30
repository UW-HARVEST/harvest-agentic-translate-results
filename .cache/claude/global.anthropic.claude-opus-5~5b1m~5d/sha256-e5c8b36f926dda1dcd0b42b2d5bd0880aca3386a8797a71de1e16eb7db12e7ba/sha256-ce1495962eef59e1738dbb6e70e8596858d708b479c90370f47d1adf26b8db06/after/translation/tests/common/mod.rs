// Shared differential-test harness.
//
// Loads BOTH shared libraries through `libloading` and calls the exported
// `driver` symbol only through the dynamic-symbol boundary — never a direct
// Rust call into the crate — so the `#[no_mangle] extern "C"` wrapper itself is
// under test.
//
// The library's only observable effect is bytes written to `stdout` via the C
// `printf` in libc, so the harness captures fd 1 into a temporary file around
// each invocation. fd-1 redirection is process-global, therefore every capture
// is serialised behind a global mutex (cargo runs tests on many threads).

#![allow(dead_code)]

use std::ffi::c_int;
use std::fs::File;
use std::os::unix::io::AsRawFd;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    /// `fflush(NULL)` flushes *all* open C output streams, which is exactly the
    /// libc `stdout` buffer both libraries write into.
    fn fflush(stream: *mut std::ffi::c_void) -> c_int;
}

pub type DriverFn = unsafe extern "C" fn(f32);

/// Which implementation to drive.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Impl {
    C,
    Rust,
}

impl Impl {
    pub fn name(self) -> &'static str {
        match self {
            Impl::C => "C",
            Impl::Rust => "Rust",
        }
    }
}

struct Libs {
    c_driver: DriverFn,
    rust_driver: DriverFn,
    // Keep the `Library` handles alive for the whole process lifetime.
    _c: &'static libloading::Library,
    _rust: &'static libloading::Library,
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// `<workdir>/c_src/build/libdriver.so`
pub fn c_so_path() -> PathBuf {
    manifest_dir()
        .parent()
        .expect("translation/ must have a parent")
        .join("c_src/build/libdriver.so")
}

/// `<manifest>/target/<profile>/libdriver.so`, derived from the running test
/// executable (`target/<profile>/deps/<test>-<hash>`) so it works for both the
/// debug and release profiles.
pub fn rust_so_path() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    let profile_dir = exe
        .parent() // deps/
        .and_then(|p| p.parent()) // <profile>/
        .expect("test exe should live in target/<profile>/deps/")
        .to_path_buf();
    profile_dir.join("libdriver.so")
}

/// Newest modification time among the files that determine the `.so`'s content.
fn newest_source_mtime() -> (std::time::SystemTime, PathBuf) {
    let mut newest = (std::time::SystemTime::UNIX_EPOCH, PathBuf::new());
    let mut consider = |p: PathBuf| {
        if let Ok(md) = std::fs::metadata(&p) {
            if let Ok(t) = md.modified() {
                if t > newest.0 {
                    newest = (t, p);
                }
            }
        }
    };

    consider(manifest_dir().join("Cargo.toml"));

    // Walk src/ recursively.
    let mut stack = vec![manifest_dir().join("src")];
    while let Some(dir) = stack.pop() {
        if let Ok(entries) = std::fs::read_dir(&dir) {
            for e in entries.flatten() {
                let p = e.path();
                if p.is_dir() {
                    stack.push(p);
                } else {
                    consider(p);
                }
            }
        }
    }
    newest
}

fn assert_so_is_fresh(so: &std::path::Path) {
    let so_mtime = std::fs::metadata(so)
        .and_then(|m| m.modified())
        .expect("stat the Rust .so");
    let (src_mtime, src_path) = newest_source_mtime();
    assert!(
        so_mtime >= src_mtime,
        "STALE ARTIFACT: {} is older than {}.\n\
         `cargo test` compiles the lib target as a test binary but does NOT \
         re-link the cdylib, so these tests would have loaded an out-of-date \
         library and passed vacuously.\n\
         Run `cargo build --release` (or use ./run_all_features.sh) before \
         `cargo test`.",
        so.display(),
        src_path.display()
    );
}

fn libs() -> &'static Libs {
    static LIBS: OnceLock<Libs> = OnceLock::new();
    LIBS.get_or_init(|| {
        let c_path = c_so_path();
        let rust_path = rust_so_path();

        assert!(
            c_path.exists(),
            "C shared library not found at {}\nBuild it with:\n  cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            c_path.display()
        );
        assert!(
            rust_path.exists(),
            "Rust shared library not found at {}\nBuild it with:\n  cd translation && cargo build --release",
            rust_path.display()
        );

        // CRITICAL: `cargo test` compiles the lib target as a *test* binary but
        // does NOT re-link the `cdylib` artifact. Without this guard the tests
        // silently load a stale `libdriver.so` and pass no matter what the
        // current source says. Refuse to run unless the `.so` is newer than
        // every input that feeds it.
        assert_so_is_fresh(&rust_path);

        unsafe {
            let c_lib: &'static libloading::Library = Box::leak(Box::new(
                libloading::Library::new(&c_path)
                    .unwrap_or_else(|e| panic!("dlopen({}) failed: {e}", c_path.display())),
            ));
            let rust_lib: &'static libloading::Library = Box::leak(Box::new(
                libloading::Library::new(&rust_path)
                    .unwrap_or_else(|e| panic!("dlopen({}) failed: {e}", rust_path.display())),
            ));

            let c_sym: libloading::Symbol<DriverFn> = c_lib
                .get(b"driver\0")
                .expect("C .so does not export `driver`");
            let rust_sym: libloading::Symbol<DriverFn> = rust_lib
                .get(b"driver\0")
                .expect("Rust .so does not export `driver`");

            Libs {
                c_driver: *c_sym,
                rust_driver: *rust_sym,
                _c: c_lib,
                _rust: rust_lib,
            }
        }
    })
}

pub fn driver_fn(which: Impl) -> DriverFn {
    match which {
        Impl::C => libs().c_driver,
        Impl::Rust => libs().rust_driver,
    }
}

/// Force both libraries to be resolved (used by the symbol-parity test).
pub fn ensure_loaded() {
    let _ = libs();
}

fn capture_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

static CAPTURE_SEQ: AtomicU64 = AtomicU64::new(0);

/// Run `f` with fd 1 redirected to a fresh temporary file and return every byte
/// it wrote. The destination is a regular file, so libc treats `stdout` as fully
/// buffered — the case where a translation using its own private buffer would
/// reorder output relative to the C library.
pub fn capture<F: FnOnce()>(f: F) -> Vec<u8> {
    let guard = capture_lock().lock().unwrap_or_else(|e| e.into_inner());

    let seq = CAPTURE_SEQ.fetch_add(1, Ordering::SeqCst);
    let path = std::env::temp_dir().join(format!(
        "driver_diff_capture_{}_{}.bin",
        std::process::id(),
        seq
    ));

    let file = File::create(&path).expect("create capture file");
    let fd = file.as_raw_fd();

    // NOTE: deliberately do NOT flush Rust's own `std::io::stdout` here. If
    // libtest had a partial progress line ("test foo ... ") still buffered,
    // flushing would push it *into* the capture file. Leaving it buffered means
    // it reaches the real stdout after fd 1 is restored. `RUST_TEST_THREADS=1`
    // (.cargo/config.toml) keeps libtest from writing during the window.

    let result = unsafe {
        // Flush anything already pending so it lands on the real stdout.
        fflush(std::ptr::null_mut());
        let saved = dup(1);
        assert!(saved >= 0, "dup(1) failed");
        assert!(dup2(fd, 1) >= 0, "dup2(fd, 1) failed");

        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));

        // Flush the library's writes into the temp file before restoring fd 1.
        fflush(std::ptr::null_mut());
        assert!(dup2(saved, 1) >= 0, "restoring fd 1 failed");
        close(saved);
        r
    };

    drop(file);
    let bytes = std::fs::read(&path).expect("read capture file");
    let _ = std::fs::remove_file(&path);

    drop(guard);

    match result {
        Ok(()) => {
            // Defensive: the only thing that may write into a capture window is
            // the library under test, whose output alphabet is exactly
            // [0-9a-f\n]. Anything else means foreign bytes (e.g. libtest
            // progress output) contaminated the capture, which would silently
            // corrupt every comparison -- fail loudly instead.
            if let Some(bad) = bytes
                .iter()
                .position(|&b| !(b.is_ascii_digit() || (b'a'..=b'f').contains(&b) || b == b'\n'))
            {
                panic!(
                    "capture contaminated by foreign output at offset {bad} \
                     (byte {:#04x}); captured: {:?}",
                    bytes[bad],
                    String::from_utf8_lossy(&bytes)
                );
            }
            bytes
        }
        Err(payload) => std::panic::resume_unwind(payload),
    }
}

/// Call `driver` from `which` implementation once, returning its exact output.
pub fn run_one(which: Impl, x: f32) -> Vec<u8> {
    let f = driver_fn(which);
    capture(|| unsafe { f(x) })
}

/// Call `driver` from `which` implementation once for each value, in order,
/// inside a SINGLE capture. Batching keeps the differential comparison honest
/// about the shared `stdout` buffer while making large sweeps fast.
pub fn run_batch(which: Impl, xs: &[f32]) -> Vec<u8> {
    let f = driver_fn(which);
    capture(|| {
        for &x in xs {
            unsafe { f(x) }
        }
    })
}

/// Independent oracle: what the C source says the output must be for one call,
/// i.e. `sizeof(float)` bytes of the native object representation, each printed
/// with `%02x`, then `'\n'`.
pub fn oracle_one(x: f32) -> Vec<u8> {
    let mut out = Vec::with_capacity(9);
    for b in x.to_ne_bytes() {
        out.extend_from_slice(format!("{:02x}", b).as_bytes());
    }
    out.push(b'\n');
    out
}

pub fn oracle_batch(xs: &[f32]) -> Vec<u8> {
    let mut out = Vec::with_capacity(xs.len() * 9);
    for &x in xs {
        out.extend_from_slice(&oracle_one(x));
    }
    out
}

fn describe(x: f32) -> String {
    let bits = x.to_bits();
    format!(
        "bits=0x{:08x} class={} value={:?}",
        bits,
        match (bits >> 23) & 0xff {
            0 if bits & 0x7fffff == 0 => "zero",
            0 => "subnormal",
            0xff if bits & 0x7fffff == 0 => "inf",
            0xff => "nan",
            _ => "normal",
        },
        x
    )
}

/// Compare C vs Rust for a batch of inputs, and also compare both against the
/// independent oracle. Reports the first differing input precisely.
pub fn assert_batch_matches(row: &str, xs: &[f32]) {
    let c_out = run_batch(Impl::C, xs);
    let rust_out = run_batch(Impl::Rust, xs);

    if c_out != rust_out {
        // Locate the first differing 9-byte record for a useful message.
        let n = c_out.len().min(rust_out.len());
        let mut first = None;
        for i in 0..n {
            if c_out[i] != rust_out[i] {
                first = Some(i);
                break;
            }
        }
        let idx = first.unwrap_or(n);
        let rec = idx / 9;
        let ctx_start = rec.saturating_sub(1) * 9;
        let ctx_end = ((rec + 2) * 9).min(n);
        panic!(
            "[{row}] C and Rust output differ.\n\
             first differing byte offset: {idx} (record {rec} of {})\n\
             offending input: {}\n\
             C    bytes {ctx_start}..{ctx_end}: {:?}\n\
             Rust bytes {ctx_start}..{ctx_end}: {:?}\n\
             C len={} Rust len={}",
            xs.len(),
            xs.get(rec).map(|&v| describe(v)).unwrap_or_else(|| "<past end>".into()),
            String::from_utf8_lossy(&c_out[ctx_start..ctx_end]),
            String::from_utf8_lossy(&rust_out[ctx_start..ctx_end]),
            c_out.len(),
            rust_out.len()
        );
    }

    let expected = oracle_batch(xs);
    assert_eq!(
        c_out.len(),
        expected.len(),
        "[{row}] C produced {} bytes, oracle expects {} for {} inputs",
        c_out.len(),
        expected.len(),
        xs.len()
    );
    if c_out != expected {
        let idx = c_out
            .iter()
            .zip(expected.iter())
            .position(|(a, b)| a != b)
            .unwrap();
        let rec = idx / 9;
        panic!(
            "[{row}] C output disagrees with the oracle at record {rec}: input {}",
            describe(xs[rec])
        );
    }
}

/// Same as [`assert_batch_matches`] but each input also gets its own isolated
/// single-call capture, which is the strictest form (no batching can mask a
/// per-call difference in trailing newline placement).
pub fn assert_each_matches(row: &str, xs: &[f32]) {
    for &x in xs {
        let c_out = run_one(Impl::C, x);
        let rust_out = run_one(Impl::Rust, x);
        assert_eq!(
            c_out,
            rust_out,
            "[{row}] single-call divergence for {}\n  C   = {:?}\n  Rust= {:?}",
            describe(x),
            String::from_utf8_lossy(&c_out),
            String::from_utf8_lossy(&rust_out)
        );
        assert_eq!(
            c_out,
            oracle_one(x),
            "[{row}] C disagrees with oracle for {}",
            describe(x)
        );
    }
    assert_batch_matches(row, xs);
}

// ---------------------------------------------------------------------------
// Deterministic RNG (SplitMix64) — fixed seed, so every run is reproducible.
// ---------------------------------------------------------------------------

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

    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }

    /// Uniform in `[0, n)` for `n > 0`.
    pub fn below(&mut self, n: u32) -> u32 {
        assert!(n > 0);
        self.next_u32() % n
    }

    /// Uniform in `[lo, hi]`.
    pub fn range(&mut self, lo: u32, hi: u32) -> u32 {
        assert!(lo <= hi);
        lo + self.below(hi - lo + 1)
    }

    pub fn bit(&mut self) -> u32 {
        (self.next_u64() >> 63) as u32
    }
}

/// Assemble a `float` from its IEEE-754 binary32 fields.
pub fn from_fields(sign: u32, exponent: u32, mantissa: u32) -> f32 {
    debug_assert!(sign <= 1);
    debug_assert!(exponent <= 0xff);
    debug_assert!(mantissa <= 0x7f_ffff);
    f32::from_bits((sign << 31) | (exponent << 23) | mantissa)
}
