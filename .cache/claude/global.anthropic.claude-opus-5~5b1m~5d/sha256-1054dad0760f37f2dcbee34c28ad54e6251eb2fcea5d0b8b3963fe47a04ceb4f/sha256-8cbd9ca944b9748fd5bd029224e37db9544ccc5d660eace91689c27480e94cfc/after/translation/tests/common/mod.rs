//! Shared harness for the C-vs-Rust differential tests.
//!
//! BOTH libraries are loaded as shared objects with `libloading` and called
//! only through their exported `driver` symbol -- the Rust functions are never
//! called directly, so the `#[no_mangle] extern "C"` wrapper is under test too.
//!
//! `driver` returns `void` and its entire observable behaviour is what it
//! `printf`s to `stdout`, so the harness captures file descriptor 1 (the real
//! OS-level stdout that libc's `stdout` FILE stream writes to) around each
//! call and compares the captured bytes.

#![allow(dead_code)]

use std::ffi::c_int;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use libloading::{Library, Symbol};

extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    /// `fflush(NULL)` flushes *every* open output stream, which is what we need
    /// because the C `.so` and the Rust `.so` share the process' libc `stdout`.
    fn fflush(stream: *mut core::ffi::c_void) -> c_int;
}

/// fd 1 is process-global, so captures must not run concurrently.
static CAPTURE_LOCK: Mutex<()> = Mutex::new(());

// ---------------------------------------------------------------------------
// Locating the two shared objects
// ---------------------------------------------------------------------------

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// `c_src/build/libdriver.so`, built by
/// `cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .`
pub fn c_so_path() -> PathBuf {
    let p = manifest_dir().join("../c_src/build/libdriver.so");
    assert!(
        p.exists(),
        "C shared library not found at {}\nBuild it with:\n  cd c_src && mkdir -p build && cd build \\\n    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        p.display()
    );
    p
}

/// The Rust `cdylib`. Prefer the profile the tests themselves were built with,
/// then fall back to the other one.
pub fn rust_so_path() -> PathBuf {
    let target = std::env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| manifest_dir().join("target"));

    let preferred: &[&str] = if cfg!(debug_assertions) {
        &["debug", "release"]
    } else {
        &["release", "debug"]
    };

    for profile in preferred {
        let p = target.join(profile).join("libdriver.so");
        if p.exists() {
            return p;
        }
    }
    panic!(
        "Rust cdylib not found under {}\nBuild it with:  cd translation && cargo build --release",
        target.display()
    );
}

// ---------------------------------------------------------------------------
// Loading
// ---------------------------------------------------------------------------

/// `void driver(int)`
pub type DriverFn = unsafe extern "C" fn(c_int);

pub struct Lib {
    pub name: &'static str,
    pub path: PathBuf,
    lib: Library,
}

impl Lib {
    fn open(name: &'static str, path: PathBuf) -> Self {
        let lib = unsafe { Library::new(&path) }
            .unwrap_or_else(|e| panic!("dlopen({}) failed: {e}", path.display()));
        Lib { name, path, lib }
    }

    /// Resolve an exported symbol. Panics if the `.so` does not export it --
    /// which is exactly the Phase A / Phase D symbol-parity check, performed
    /// through a real `dlsym`.
    pub fn driver(&self) -> Symbol<'_, DriverFn> {
        unsafe { self.lib.get(b"driver\0") }.unwrap_or_else(|e| {
            panic!(
                "{} ({}) does not export symbol `driver`: {e}",
                self.name,
                self.path.display()
            )
        })
    }

    pub fn has_symbol(&self, sym: &[u8]) -> bool {
        let mut owned = sym.to_vec();
        owned.push(0);
        unsafe { self.lib.get::<*const ()>(&owned) }.is_ok()
    }
}

/// The pair of libraries under comparison.
pub struct Pair {
    pub c: Lib,
    pub rust: Lib,
}

/// Both `.so`s are opened once per test binary. `libloading`'s `Library::new`
/// uses `RTLD_LOCAL`, so each handle's `dlsym("driver")` resolves to that
/// library's *own* definition even though the name collides.
pub fn libs() -> &'static Pair {
    use std::sync::OnceLock;
    static PAIR: OnceLock<Pair> = OnceLock::new();
    PAIR.get_or_init(|| Pair {
        c: Lib::open("C .so", c_so_path()),
        rust: Lib::open("Rust .so", rust_so_path()),
    })
}

// ---------------------------------------------------------------------------
// stdout capture
// ---------------------------------------------------------------------------

fn tmp_path(tag: &str) -> PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "driver_diff_{}_{}_{}_{}.out",
        tag,
        std::process::id(),
        n,
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ))
}

fn read_and_remove(path: &Path) -> Vec<u8> {
    let mut buf = Vec::new();
    std::fs::File::open(path)
        .unwrap_or_else(|e| panic!("reopen {} failed: {e}", path.display()))
        .read_to_end(&mut buf)
        .unwrap_or_else(|e| panic!("read {} failed: {e}", path.display()));
    let _ = std::fs::remove_file(path);
    buf
}

/// Run `f`, capturing everything written to fd 1 (by either `.so`) while it
/// runs, and return the raw bytes.
pub fn capture_fd1<F: FnOnce()>(tag: &str, f: F) -> Vec<u8> {
    let _guard = CAPTURE_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    let path = tmp_path(tag);
    let file = std::fs::File::create(&path)
        .unwrap_or_else(|e| panic!("create {} failed: {e}", path.display()));

    unsafe {
        // Flush anything already pending so it lands on the *real* stdout.
        fflush(core::ptr::null_mut());

        let saved = dup(1);
        assert!(saved >= 0, "dup(1) failed");

        use std::os::unix::io::AsRawFd;
        assert!(dup2(file.as_raw_fd(), 1) >= 0, "dup2 -> fd 1 failed");

        f();

        // Force libc to drain its (now fd-1-backed) buffers before we restore.
        fflush(core::ptr::null_mut());

        assert!(dup2(saved, 1) >= 0, "restoring fd 1 failed");
        close(saved);
    }

    drop(file);
    read_and_remove(&path)
}

/// Call `driver(x)` in the given library and return exactly what it printed.
pub fn call_capture(lib: &Lib, x: i32) -> Vec<u8> {
    let f = lib.driver();
    capture_fd1("one", || unsafe { f(x as c_int) })
}

/// Call `driver` once per input, all inside a single capture, then split the
/// result back into one `Vec<u8>` per call. `driver` emits exactly one
/// `'\n'`-terminated line per call, so the split is unambiguous -- and this
/// keeps large randomized rows fast while preserving per-input attribution.
pub fn call_capture_batch(lib: &Lib, inputs: &[i32]) -> Vec<Vec<u8>> {
    let f = lib.driver();
    let raw = capture_fd1("batch", || {
        for &x in inputs {
            unsafe { f(x as c_int) }
        }
    });

    let lines = split_lines(&raw);
    assert_eq!(
        lines.len(),
        inputs.len(),
        "{}: expected one output line per call ({} inputs) but got {} lines; raw = {:?}",
        lib.name,
        inputs.len(),
        lines.len(),
        String::from_utf8_lossy(&raw)
    );
    lines
}

/// Split on `'\n'`, keeping the terminator on each piece so the comparison is
/// byte-exact (a missing or doubled newline shows up as a diff).
pub fn split_lines(raw: &[u8]) -> Vec<Vec<u8>> {
    let mut out = Vec::new();
    let mut cur = Vec::new();
    for &b in raw {
        cur.push(b);
        if b == b'\n' {
            out.push(std::mem::take(&mut cur));
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

fn show(b: &[u8]) -> String {
    format!("{:?} (len {})", String::from_utf8_lossy(b), b.len())
}

// ---------------------------------------------------------------------------
// Differential assertions
// ---------------------------------------------------------------------------

/// Row-level assertion: for each input, C and Rust must print identical bytes.
pub fn assert_same(row: &str, inputs: &[i32]) {
    let p = libs();

    let c_out = call_capture_batch(&p.c, inputs);
    let r_out = call_capture_batch(&p.rust, inputs);

    for (i, x) in inputs.iter().enumerate() {
        assert_eq!(
            c_out[i],
            r_out[i],
            "[{row}] divergence for driver({x}) (= {x:#010x}, input #{i})\n  C    : {}\n  Rust : {}",
            show(&c_out[i]),
            show(&r_out[i])
        );
    }
}

/// Same as [`assert_same`] but each call gets its OWN capture (one `dup2`
/// window per call). Slower, so it is used for the small hand-picked rows; it
/// additionally proves each single call flushes a complete line on its own.
pub fn assert_same_isolated(row: &str, inputs: &[i32]) {
    let p = libs();
    for (i, &x) in inputs.iter().enumerate() {
        let c = call_capture(&p.c, x);
        let r = call_capture(&p.rust, x);
        assert_eq!(
            c, r,
            "[{row}] divergence for driver({x}) (= {x:#010x}, input #{i}, isolated capture)\n  C    : {}\n  Rust : {}",
            show(&c),
            show(&r)
        );
    }
}

/// Both of the above, so a row is covered batched *and* isolated.
pub fn assert_same_both_ways(row: &str, inputs: &[i32]) {
    assert_same(row, inputs);
    assert_same_isolated(row, inputs);
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (xorshift64*) -- fixed seed => reproducible rows
// ---------------------------------------------------------------------------

pub const SEED: u64 = 0x0000_D3C0_DE15_EA5E;

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
    pub fn next_i32(&mut self) -> i32 {
        self.next_u32() as i32
    }
    /// Uniform in `0..n` (n > 0).
    pub fn below(&mut self, n: u32) -> u32 {
        self.next_u32() % n
    }
}
