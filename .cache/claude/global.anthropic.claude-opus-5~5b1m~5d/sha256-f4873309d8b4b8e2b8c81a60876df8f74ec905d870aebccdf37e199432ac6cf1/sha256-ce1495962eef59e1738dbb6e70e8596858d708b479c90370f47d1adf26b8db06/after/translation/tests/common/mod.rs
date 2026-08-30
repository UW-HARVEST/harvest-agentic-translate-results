//! Shared harness for the C-vs-Rust differential tests.
//!
//! Both shared libraries are loaded via `libloading` and driven **only** through
//! their exported `extern "C"` symbols — the Rust functions are never called
//! directly, so the `#[no_mangle]` export wrappers are under test too.
//!
//! Every function in this library returns `void` and communicates exclusively
//! through libc `printf`, so "output" means "the bytes written to fd 1". We
//! capture them by `dup2`-ing a temp file over fd 1. This works for BOTH
//! libraries because both funnel through the *same* libc `stdout` of the test
//! process (the Rust translation deliberately imports `printf` instead of using
//! `std::io::stdout`), which is exactly what makes byte-identical comparison
//! meaningful.

#![allow(dead_code)]

use libloading::Library;
use std::ffi::{c_char, c_int, CString};
use std::fs::File;
use std::os::unix::io::AsRawFd;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut core::ffi::c_void) -> c_int;
}

// ---------------------------------------------------------------------------
// The exported API surface (all 5 dynamic symbols of both `.so`s)
// ---------------------------------------------------------------------------

pub struct Api {
    pub name: &'static str,
    pub print_line: extern "C" fn(*const c_char),
    pub print_int_line: extern "C" fn(c_int),
    pub bad: extern "C" fn(f32),
    pub good: extern "C" fn(f32),
    pub driver: extern "C" fn(f32, f32),
}

fn load(name: &'static str, path: &Path) -> Api {
    assert!(
        path.exists(),
        "shared library not found: {}\n\
         Build it first:\n  C:    cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .\n  \
         Rust: cd translation && cargo build",
        path.display()
    );
    unsafe {
        // Leaked so the symbols' fn pointers stay valid for the whole run.
        let lib: &'static Library = Box::leak(Box::new(
            Library::new(path).unwrap_or_else(|e| panic!("dlopen {}: {e}", path.display())),
        ));
        macro_rules! sym {
            ($s:literal) => {
                *lib.get($s).unwrap_or_else(|e| {
                    panic!("symbol {} missing from {}: {e}", stringify!($s), name)
                })
            };
        }
        Api {
            name,
            print_line: sym!(b"printLine\0"),
            print_int_line: sym!(b"printIntLine\0"),
            bad: sym!(b"bad\0"),
            good: sym!(b"good\0"),
            driver: sym!(b"driver\0"),
        }
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// `target/<profile>/` — derived from the running test binary's own location
/// (`target/<profile>/deps/<test>`), so it is correct for debug and release.
fn target_profile_dir() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    exe.parent()
        .and_then(|deps| deps.parent())
        .expect("target/<profile>")
        .to_path_buf()
}

/// `cargo test` builds the crate as an *rlib* for the test binaries; it does
/// NOT produce or refresh the `cdylib` artifact we need to `dlopen`. So the
/// first test that needs it builds the cdylib itself (once per process).
///
/// STALENESS IS THE #1 HAZARD HERE. Two earlier versions of this function were
/// caught by mutation testing silently validating an OUT-OF-DATE `.so`, so that
/// a deliberately broken translation still passed every differential test:
///
///  1. Returning early when the file already existed (never rebuilt at all).
///  2. Building into the SAME target dir as the outer `cargo test`. Cargo's
///     fingerprints are mtime-based, and when `src/lib.rs` was rewritten within
///     the same wall-clock second as the previous build, cargo judged the crate
///     "fresh" and left the old `.so` in place.
///
/// So we build into a DEDICATED target dir that is wiped first. That removes
/// both the shared-fingerprint confusion and any lock contention with the outer
/// cargo invocation, and makes the artifact unconditionally fresh. The crate is
/// tiny and has no dependencies, so the full rebuild costs a fraction of a
/// second.
fn ensure_rust_so() -> PathBuf {
    static BUILT: OnceLock<PathBuf> = OnceLock::new();
    BUILT
        .get_or_init(|| {
            let release = target_profile_dir()
                .file_name()
                .map(|n| n == "release")
                .unwrap_or(false);
            let target_dir = manifest_dir().join("target/difftest-cdylib");
            // Wipe so cargo cannot possibly consider a previous build fresh.
            let _ = std::fs::remove_dir_all(&target_dir);

            let mut cmd = Command::new(env!("CARGO"));
            cmd.current_dir(manifest_dir())
                .arg("build")
                .arg("--offline")
                .arg("--lib")
                .arg("--target-dir")
                .arg(&target_dir);
            if release {
                cmd.arg("--release");
            }
            let out = cmd.output().expect("spawn cargo build for the cdylib");
            let so = target_dir
                .join(if release { "release" } else { "debug" })
                .join("libdriver.so");
            assert!(
                out.status.success() && so.exists(),
                "cargo build did not produce {}\nstdout:\n{}\nstderr:\n{}",
                so.display(),
                String::from_utf8_lossy(&out.stdout),
                String::from_utf8_lossy(&out.stderr)
            );
            // Freshness guard: the artifact must be at least as new as the source.
            let src_mtime = std::fs::metadata(manifest_dir().join("src/lib.rs"))
                .and_then(|m| m.modified())
                .expect("stat src/lib.rs");
            let so_mtime = std::fs::metadata(&so)
                .and_then(|m| m.modified())
                .expect("stat the built .so");
            assert!(
                so_mtime >= src_mtime,
                "the built {} is OLDER than src/lib.rs -- refusing to run \
                 differential tests against a stale artifact",
                so.display()
            );
            so
        })
        .clone()
}

/// Build the C shared library with CMake if it is not already present.
fn ensure_c_so() -> PathBuf {
    static BUILT: OnceLock<PathBuf> = OnceLock::new();
    BUILT
        .get_or_init(|| {
            let c_src = manifest_dir().join("../c_src");
            let build = c_src.join("build");
            let so = build.join("libdriver.so");
            if so.exists() {
                return so;
            }
            std::fs::create_dir_all(&build).expect("mkdir c_src/build");
            let cfg = Command::new("cmake")
                .current_dir(&build)
                .args(["..", "-DCMAKE_POSITION_INDEPENDENT_CODE=ON"])
                .output()
                .expect("run cmake configure");
            let bld = Command::new("cmake")
                .current_dir(&build)
                .args(["--build", "."])
                .output()
                .expect("run cmake build");
            assert!(
                so.exists(),
                "cmake did not produce {}\nconfigure:\n{}\n{}\nbuild:\n{}\n{}",
                so.display(),
                String::from_utf8_lossy(&cfg.stdout),
                String::from_utf8_lossy(&cfg.stderr),
                String::from_utf8_lossy(&bld.stdout),
                String::from_utf8_lossy(&bld.stderr),
            );
            so
        })
        .clone()
}

pub fn c_so_path() -> PathBuf {
    ensure_c_so()
}

pub fn rust_so_path() -> PathBuf {
    ensure_rust_so()
}

pub fn c_api() -> &'static Api {
    static C: OnceLock<Api> = OnceLock::new();
    C.get_or_init(|| load("C", &ensure_c_so()))
}

pub fn rust_api() -> &'static Api {
    static R: OnceLock<Api> = OnceLock::new();
    R.get_or_init(|| load("Rust", &ensure_rust_so()))
}

// ---------------------------------------------------------------------------
// stdout capture
// ---------------------------------------------------------------------------

/// fd 1 redirection is process-global, so captures must be serialized even
/// though cargo runs tests on multiple threads.
fn capture_lock() -> &'static Mutex<()> {
    static L: OnceLock<Mutex<()>> = OnceLock::new();
    L.get_or_init(|| Mutex::new(()))
}

fn temp_path() -> PathBuf {
    static N: AtomicU64 = AtomicU64::new(0);
    let n = N.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "driver_diff_{}_{}_{}.out",
        std::process::id(),
        n,
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.subsec_nanos())
            .unwrap_or(0)
    ))
}

/// Run `f`, returning every byte it wrote to fd 1.
pub fn capture<F: FnOnce()>(f: F) -> Vec<u8> {
    let guard = capture_lock().lock().unwrap_or_else(|e| e.into_inner());
    let path = temp_path();
    let bytes = unsafe {
        // Push out anything the harness itself has buffered, so it does not
        // end up inside our capture file.
        fflush(std::ptr::null_mut());
        let saved = dup(1);
        assert!(saved >= 0, "dup(1) failed");
        {
            let file = File::create(&path).expect("create capture temp file");
            assert!(dup2(file.as_raw_fd(), 1) >= 0, "dup2 onto fd 1 failed");
            // `file` drops here; fd 1 remains a valid dup of the same file.
        }
        f();
        // Flush the library's `printf` output before restoring fd 1.
        fflush(std::ptr::null_mut());
        assert!(dup2(saved, 1) >= 0, "dup2 restore of fd 1 failed");
        close(saved);
        std::fs::read(&path).expect("read capture temp file")
    };
    let _ = std::fs::remove_file(&path);
    drop(guard);
    bytes
}

// ---------------------------------------------------------------------------
// comparison helpers
// ---------------------------------------------------------------------------

pub fn show(bytes: &[u8]) -> String {
    let mut s = String::from("\"");
    for &b in bytes {
        match b {
            b'\n' => s.push_str("\\n"),
            b'\t' => s.push_str("\\t"),
            b'\\' => s.push_str("\\\\"),
            b'"' => s.push_str("\\\""),
            0x20..=0x7e => s.push(b as char),
            _ => s.push_str(&format!("\\x{b:02x}")),
        }
    }
    s.push('"');
    if bytes.len() > 400 {
        format!("{}... ({} bytes)", &s[..400.min(s.len())], bytes.len())
    } else {
        s
    }
}

/// Run one operation against both libraries and assert byte-identical stdout.
pub fn diff_one<F: Fn(&Api)>(label: &str, f: F) -> Vec<u8> {
    let c = capture(|| f(c_api()));
    let r = capture(|| f(rust_api()));
    assert_eq!(
        c,
        r,
        "\n[{label}] C and Rust stdout diverge\n  C    = {}\n  Rust = {}\n",
        show(&c),
        show(&r)
    );
    c
}

/// Batched differential run over many inputs.
///
/// Inputs are executed in chunks inside a single capture (which additionally
/// exercises repeated calls / buffering). If a chunk diverges we re-run its
/// samples one at a time to report the exact offending input.
pub fn diff_samples<T, F>(label: &str, samples: &[T], call: F)
where
    T: Copy + std::fmt::Debug,
    F: Fn(&Api, T),
{
    const CHUNK: usize = 64;
    for (ci, chunk) in samples.chunks(CHUNK).enumerate() {
        let c = capture(|| {
            let api = c_api();
            for &s in chunk {
                call(api, s);
            }
        });
        let r = capture(|| {
            let api = rust_api();
            for &s in chunk {
                call(api, s);
            }
        });
        if c == r {
            continue;
        }
        // Localize the divergence.
        for &s in chunk {
            let c1 = capture(|| call(c_api(), s));
            let r1 = capture(|| call(rust_api(), s));
            assert_eq!(
                c1,
                r1,
                "\n[{label}] divergence on input {s:?}\n  C    = {}\n  Rust = {}\n",
                show(&c1),
                show(&r1)
            );
        }
        panic!(
            "\n[{label}] chunk {ci} diverged but no single input reproduced it \
             (ordering/state-dependent bug!)\n  C    = {}\n  Rust = {}\n",
            show(&c),
            show(&r)
        );
    }
}

// ---------------------------------------------------------------------------
// deterministic RNG (xorshift64*), fixed seed for reproducibility
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new() -> Self {
        Rng(0x5EED_D1FF_1234_5678)
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
    pub fn next_i32(&mut self) -> i32 {
        self.next_u32() as i32
    }
    /// Uniform in `[0, n)`.
    pub fn below(&mut self, n: u32) -> u32 {
        self.next_u32() % n
    }
    /// A float built from random bits — hits NaNs, subnormals, infinities.
    pub fn next_f32_bits(&mut self) -> f32 {
        f32::from_bits(self.next_u32())
    }
    /// A "normal-ish" float spanning roughly 1e-20 .. 1e20, both signs.
    pub fn next_f32_decades(&mut self) -> f32 {
        let mantissa = (self.next_u32() as f64) / (u32::MAX as f64); // 0..1
        let exp = self.below(41) as i32 - 20; // -20 ..= 20
        let sign = if self.next_u32() & 1 == 0 { 1.0 } else { -1.0 };
        (sign * (0.1 + 0.9 * mantissa) * 10f64.powi(exp)) as f32
    }
}

// ---------------------------------------------------------------------------
// input corpora
// ---------------------------------------------------------------------------

/// Floats the C code treats specially (see CONFIGS.md axis 4 / ERRORS.md).
pub const INTERESTING_FLOATS: &[f32] = &[
    0.0,
    -0.0,
    1.0,
    -1.0,
    2.0,
    -2.0,
    3.0,
    -3.0,
    4.0,
    7.0,
    100.0,
    -100.0,
    0.5,
    -0.5,
    1e-6,               // ERRORS.md E12: (double)1e-6f < 1e-06, branch is FALSE
    1.0000001e-6,       // just above the threshold
    9.9e-7,             // just below
    1.1e-6,
    -1e-6,
    -1.1e-6,
    1e-7,
    1e-9,
    1e-30,
    -1e-30,
    1e-45,              // subnormal
    -1e-45,
    f32::MIN_POSITIVE,
    -f32::MIN_POSITIVE,
    f32::MAX,
    f32::MIN,
    1e3,
    1e6,
    1e30,
    -1e30,
    f32::INFINITY,
    f32::NEG_INFINITY,
    f32::NAN,
    -f32::NAN,
    // straddle the (int) overflow edge: 100.0/data == 2147483648.0 exactly
    // at data == 100.0/2147483648.0
    4.656_612_9e-8,
    4.656_613e-8,
    4.7e-8,
    4.6e-8,
];

/// A NUL-terminated C string from arbitrary bytes (interior NULs remapped,
/// since a C string cannot carry them).
pub fn cstring(bytes: &[u8]) -> CString {
    let cleaned: Vec<u8> = bytes.iter().map(|&b| if b == 0 { b'.' } else { b }).collect();
    CString::new(cleaned).expect("no interior NUL after cleaning")
}
