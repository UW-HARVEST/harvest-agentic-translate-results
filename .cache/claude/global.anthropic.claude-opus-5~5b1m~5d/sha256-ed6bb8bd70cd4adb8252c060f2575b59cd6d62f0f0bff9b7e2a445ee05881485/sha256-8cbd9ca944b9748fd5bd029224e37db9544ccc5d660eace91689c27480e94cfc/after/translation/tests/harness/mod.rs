//! Shared differential-test harness.
//!
//! Loads BOTH shared libraries with `libloading` and calls `driver` only
//! through the dynamic symbol table, exactly as an external C consumer would.
//! No Rust function is ever called directly, so the `#[no_mangle] extern "C"`
//! export wrapper is under test too.
//!
//! `driver` returns `void` and its only observable effect is bytes on stdout,
//! so the harness captures stdout at the *file-descriptor* level (dup2) and
//! flushes libc's stream buffers before reading the capture back.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::c_void;
use std::fs;
use std::io::{Read, Seek, SeekFrom};
use std::os::raw::c_int;
use std::os::unix::io::AsRawFd;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard, OnceLock};

/// fd 1 is process-global, so only one capture may be in flight at a time even
/// though the test harness runs test functions on multiple threads.
fn capture_lock() -> MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    match LOCK.get_or_init(|| Mutex::new(())).lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    }
}

extern "C" {
    /// `fflush(NULL)` flushes *every* open output stream, which is exactly what
    /// we need and avoids depending on the `stdout` data symbol's ABI.
    fn fflush(stream: *mut c_void) -> c_int;
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
}

pub type DriverFn = unsafe extern "C" fn(c_int);

const STDOUT_FILENO: c_int = 1;

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Path to the C shared library built by CMake.
fn c_lib_path() -> PathBuf {
    if let Ok(p) = std::env::var("C_DRIVER_SO") {
        return PathBuf::from(p);
    }
    let p = manifest_dir()
        .parent()
        .expect("crate has a parent dir")
        .join("c_src/build/libdriver.so");
    assert!(
        p.exists(),
        "C shared library not found at {p:?}. Build it with:\n  \
         cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build ."
    );
    p
}

/// Path to the Rust `cdylib`. Prefers the release artifact, falls back to debug.
fn rust_lib_path() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_DRIVER_SO") {
        return PathBuf::from(p);
    }
    let base = manifest_dir().join("target");
    for profile in ["release", "debug"] {
        let p = base.join(profile).join("libdriver.so");
        if p.exists() {
            return p;
        }
    }
    panic!(
        "Rust cdylib not found under {base:?}. Build it with:\n  \
         cd translation && cargo build --release"
    );
}

/// The two libraries plus their resolved `driver` symbols.
pub struct Libs {
    _c: Library,
    _rust: Library,
    pub c_driver: DriverFn,
    pub rust_driver: DriverFn,
}

impl Libs {
    pub fn load() -> Libs {
        unsafe {
            let c = Library::new(c_lib_path()).expect("dlopen C libdriver.so");
            let rust = Library::new(rust_lib_path()).expect("dlopen Rust libdriver.so");
            let c_sym: Symbol<DriverFn> =
                c.get(b"driver\0").expect("C .so must export `driver`");
            let rust_sym: Symbol<DriverFn> =
                rust.get(b"driver\0").expect("Rust .so must export `driver`");
            let c_driver = *c_sym;
            let rust_driver = *rust_sym;
            Libs {
                _c: c,
                _rust: rust,
                c_driver,
                rust_driver,
            }
        }
    }
}

/// Capture everything written to file descriptor 1 by `f`, including output
/// that goes through libc's `stdout` buffer.
pub fn capture_stdout<F: FnOnce()>(f: F) -> Vec<u8> {
    let _guard = capture_lock();
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    let path = std::env::temp_dir().join(format!(
        "driver_diff_{}_{}_{n}.out",
        std::process::id(),
        // keep names unique across threads too
        format!("{:?}", std::thread::current().id())
            .replace(|c: char| !c.is_ascii_alphanumeric(), "")
    ));

    let mut file = fs::OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(true)
        .open(&path)
        .expect("create capture file");

    let out = unsafe {
        // Drain anything already buffered so it is not misattributed to `f`.
        // Rust's `Stdout` keeps its own `LineWriter` buffer, which `fflush` does
        // not know about, so both must be flushed.
        let _ = std::io::Write::flush(&mut std::io::stdout());
        fflush(std::ptr::null_mut());
        let saved = dup(STDOUT_FILENO);
        assert!(saved >= 0, "dup(1) failed");
        assert!(
            dup2(file.as_raw_fd(), STDOUT_FILENO) >= 0,
            "dup2 onto stdout failed"
        );

        f();

        // Push the library's buffered bytes into the capture file *before*
        // restoring fd 1, otherwise they would land on the real stdout.
        fflush(std::ptr::null_mut());
        assert!(dup2(saved, STDOUT_FILENO) >= 0, "dup2 restore failed");
        close(saved);

        let mut buf = Vec::new();
        file.seek(SeekFrom::Start(0)).expect("seek capture file");
        file.read_to_end(&mut buf).expect("read capture file");
        buf
    };

    drop(file);
    let _ = fs::remove_file(&path);
    out
}

/// Run `driver(x)` in the C library and return its stdout bytes.
pub fn c_out(libs: &Libs, x: i32) -> Vec<u8> {
    capture_stdout(|| unsafe { (libs.c_driver)(x) })
}

/// Run `driver(x)` in the Rust library and return its stdout bytes.
pub fn rust_out(libs: &Libs, x: i32) -> Vec<u8> {
    capture_stdout(|| unsafe { (libs.rust_driver)(x) })
}

/// Assert C and Rust produce byte-identical stdout for a single input.
pub fn assert_same(libs: &Libs, x: i32, row: &str) {
    let c = c_out(libs, x);
    let r = rust_out(libs, x);
    if c != r {
        panic!(
            "[{row}] divergence for driver({x}) (0x{x:08x}):\n  \
             C   : {:?} ({:02x?})\n  Rust: {:?} ({:02x?})",
            String::from_utf8_lossy(&c),
            c,
            String::from_utf8_lossy(&r),
            r
        );
    }
    // The C output is `2*sizeof(int)` hex digits plus '\n'.
    assert_eq!(c.len(), 9, "[{row}] unexpected record length for {x}");
}

/// Batched comparison: run the whole input sequence through C in one capture
/// and through Rust in one capture, then compare the full transcripts. This is
/// far faster than per-input redirection for large input sets and additionally
/// checks record framing / ordering across a sequence of calls.
pub fn assert_same_batch(libs: &Libs, xs: &[i32], row: &str) {
    let c = capture_stdout(|| {
        for &x in xs {
            unsafe { (libs.c_driver)(x) }
        }
    });
    let r = capture_stdout(|| {
        for &x in xs {
            unsafe { (libs.rust_driver)(x) }
        }
    });
    if c != r {
        // Narrow the failure down to the first differing record.
        let cl: Vec<&[u8]> = c.split(|&b| b == b'\n').collect();
        let rl: Vec<&[u8]> = r.split(|&b| b == b'\n').collect();
        for (i, (a, b)) in cl.iter().zip(rl.iter()).enumerate() {
            if a != b {
                panic!(
                    "[{row}] divergence at record {i} (input {} / 0x{:08x}):\n  \
                     C   : {:?}\n  Rust: {:?}",
                    xs.get(i).copied().unwrap_or_default(),
                    xs.get(i).copied().unwrap_or_default(),
                    String::from_utf8_lossy(a),
                    String::from_utf8_lossy(b)
                );
            }
        }
        panic!(
            "[{row}] transcripts differ in length: C {} bytes / {} records, \
             Rust {} bytes / {} records",
            c.len(),
            cl.len(),
            r.len(),
            rl.len()
        );
    }
    assert_eq!(
        c.len(),
        xs.len() * 9,
        "[{row}] expected exactly one 9-byte record per call"
    );
}

/// Run a list of named rows STRICTLY SEQUENTIALLY inside a single `#[test]`.
///
/// This matters: `capture_stdout` redirects the process-wide file descriptor 1,
/// so nothing else in the process — including the libtest harness printing its
/// own `test foo ... ok` progress lines from another thread — may write to
/// stdout while a capture is in flight. Collapsing every row into one `#[test]`
/// removes intra-binary parallelism entirely, so the suite is correct under a
/// plain `cargo test` without needing `--test-threads=1`.
///
/// Each row runs under `catch_unwind` so one divergence does not hide the rest;
/// the aggregate result is asserted at the end.
pub fn run_rows(rows: &[(&str, fn(&Libs))]) {
    let libs = Libs::load();
    let mut failures: Vec<String> = Vec::new();
    for (name, f) in rows {
        let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| f(&libs)));
        match res {
            Ok(()) => eprintln!("  [PASS] {name}"),
            Err(e) => {
                let msg = if let Some(s) = e.downcast_ref::<String>() {
                    s.clone()
                } else if let Some(s) = e.downcast_ref::<&str>() {
                    (*s).to_string()
                } else {
                    "<non-string panic>".to_string()
                };
                eprintln!("  [FAIL] {name}: {msg}");
                failures.push(format!("{name}: {msg}"));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "{} of {} rows diverged:\n{}",
        failures.len(),
        rows.len(),
        failures.join("\n")
    );
}

/// Deterministic SplitMix64 PRNG so every randomized row is reproducible.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed)
    }
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E3779B97F4A7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
        z ^ (z >> 31)
    }
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    pub fn next_i32(&mut self) -> i32 {
        self.next_u32() as i32
    }
    /// An `i32` whose four native bytes are each drawn from `pool`.
    pub fn i32_from_byte_pool(&mut self, pool: &[u8]) -> i32 {
        let mut bytes = [0u8; 4];
        for b in bytes.iter_mut() {
            *b = pool[(self.next_u32() as usize) % pool.len()];
        }
        i32::from_ne_bytes(bytes)
    }
}

/// Build an `i32` from native-order bytes (index 0 == first byte `memcpy` sees).
pub fn from_native_bytes(b: [u8; 4]) -> i32 {
    i32::from_ne_bytes(b)
}

/// `x` with the byte at native offset `off` replaced by `v`.
pub fn with_byte(base: i32, off: usize, v: u8) -> i32 {
    let mut b = base.to_ne_bytes();
    b[off] = v;
    i32::from_ne_bytes(b)
}
