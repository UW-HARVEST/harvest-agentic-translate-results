// Shared harness for the C-vs-Rust differential tests.
//
// Both libraries are loaded as shared objects through `libloading` and called
// only through their exported `driver` symbol, exactly as an external C caller
// would.  The Rust implementation is NEVER called directly, so the
// `#[no_mangle] extern "C"` export wrapper is part of what is tested.

#![allow(dead_code)]

use libloading::Library;
use std::ffi::{c_int, c_void};
use std::os::fd::AsRawFd;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn pipe(fds: *mut c_int) -> c_int;
    fn read(fd: c_int, buf: *mut u8, count: usize) -> isize;
    // `fflush(NULL)` flushes every open output stream of the process, which is
    // how the bytes written by the dynamically loaded libraries (they share the
    // single process-wide libc `stdout`) are forced out before we read them.
    fn fflush(stream: *mut c_void) -> c_int;
}

// ---------------------------------------------------------------------------
// Library locations
// ---------------------------------------------------------------------------

pub fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Directory holding the freshly built Rust artifacts (`target/<profile>/`).
fn rust_artifact_dir() -> PathBuf {
    // current_exe() is target/<profile>/deps/<test-binary>
    let exe = std::env::current_exe().expect("current_exe");
    exe.parent()
        .and_then(Path::parent)
        .expect("target/<profile>")
        .to_path_buf()
}

pub fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("DRIVER_RUST_SO") {
        return PathBuf::from(p);
    }
    let p = rust_artifact_dir().join("libdriver.so");
    assert!(
        p.exists(),
        "Rust cdylib not found at {p:?}; run `cargo build` first"
    );
    p
}

pub fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("DRIVER_C_SO") {
        return PathBuf::from(p);
    }
    let p = manifest_dir().join("c_src/build/libdriver.so");
    assert!(
        p.exists(),
        "C shared library not found at {p:?}; build it with:\n  \
         cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build ."
    );
    p
}

// ---------------------------------------------------------------------------
// Loaded libraries
// ---------------------------------------------------------------------------

pub struct Impl {
    pub name: &'static str,
    /// `void driver(int)` as declared in `c_src/include/driver.h`.
    pub driver: extern "C" fn(c_int),
    /// Same code address, but described to the compiler as taking a 64-bit
    /// argument.  Used to check that garbage in the upper half of the argument
    /// register is ignored identically by both implementations.
    pub driver_wide: extern "C" fn(i64),
    pub addr: usize,
}

fn load(name: &'static str, src: &Path) -> Impl {
    // dlopen de-duplicates by device+inode, and the CMake-built C object even
    // carries `DT_SONAME = libdriver.so` (the Rust cdylib carries none).  Copy
    // each object to a distinct file so the two handles can never alias; the
    // `addr` values are asserted to differ below.
    // The staging directory is per-process: copying over a file that another
    // test process has mmap'd and is executing from would make that process die
    // with SIGBUS.
    let stage = rust_artifact_dir().join(format!("difftest-libs-{}", std::process::id()));
    std::fs::create_dir_all(&stage).expect("create staging dir");
    let dst = stage.join(format!("libdriver_{name}.so"));
    std::fs::copy(src, &dst).unwrap_or_else(|e| panic!("copy {src:?} -> {dst:?}: {e}"));

    // Leaked so the `Library` outlives every borrowed symbol for the whole
    // process lifetime (the libraries stay loaded on purpose).
    let lib: &'static Library = Box::leak(Box::new(unsafe {
        Library::new(&dst).unwrap_or_else(|e| panic!("dlopen {dst:?}: {e}"))
    }));

    let driver: extern "C" fn(c_int) = unsafe {
        *lib.get::<extern "C" fn(c_int)>(b"driver\0")
            .unwrap_or_else(|e| panic!("dlsym driver in {dst:?}: {e}"))
    };
    let addr = driver as usize;
    let driver_wide: extern "C" fn(i64) = unsafe { std::mem::transmute(addr) };

    Impl {
        name,
        driver,
        driver_wide,
        addr,
    }
}

pub struct Impls {
    pub c: Impl,
    pub rust: Impl,
}

pub fn impls() -> &'static Impls {
    static IMPLS: OnceLock<Impls> = OnceLock::new();
    IMPLS.get_or_init(|| {
        let c = load("c", &c_so_path());
        let rust = load("rust", &rust_so_path());
        assert_ne!(
            c.addr, rust.addr,
            "the C and Rust `driver` symbols resolved to the SAME address: \
             dlopen aliased the two shared objects, so the comparison would be \
             vacuous"
        );
        Impls { c, rust }
    })
}

// ---------------------------------------------------------------------------
// stdout capture
// ---------------------------------------------------------------------------

static CAPTURE_LOCK: Mutex<()> = Mutex::new(());
static COUNTER: AtomicU64 = AtomicU64::new(0);

/// Restores fd 1 even if the body panics, so a failing assertion inside a
/// capture window can never leave the process with a hijacked stdout.
struct FdRestore(c_int);

impl Drop for FdRestore {
    fn drop(&mut self) {
        unsafe {
            fflush(std::ptr::null_mut());
            dup2(self.0, 1);
            close(self.0);
        }
    }
}

/// NOTE on isolation: fd 1 is process-global, so while it is redirected any
/// *other* thread writing to stdout would land in our capture file.  The libtest
/// harness does exactly that when several `#[test]`s run in parallel, which is
/// why each capturing test binary contains exactly ONE `#[test]` (see
/// [`RowRunner`]); the harness then writes nothing between the start and the end
/// of that test — with ONE exception: after 60 s libtest's main thread prints
///
/// ```text
/// test <name> has been running for over 60 seconds
/// ```
///
/// to stdout, which lands in whatever capture is open at that moment.  Tests
/// that can run longer than 60 s must therefore treat a capture whose length
/// disagrees with the expected length as POLLUTED and retry it (see
/// `tests/exhaustive.rs`); the fast Phase B/C suites finish in seconds and never
/// hit it.
///
/// Runs `f` with file descriptor 1 redirected into a fresh regular file and
/// returns everything that was written to it.
pub fn capture_stdout<F: FnOnce()>(f: F) -> Vec<u8> {
    let guard = CAPTURE_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    // Push out anything already buffered (libc streams *and* Rust's own
    // `stdout`) so it cannot leak into the capture file.
    use std::io::Write;
    let _ = std::io::stdout().flush();
    unsafe { fflush(std::ptr::null_mut()) };

    let id = COUNTER.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("driver_cap_{}_{id}.out", std::process::id()));
    let file = std::fs::File::create(&path).expect("create capture file");

    let saved = unsafe { dup(1) };
    assert!(saved >= 0, "dup(1) failed");
    assert!(unsafe { dup2(file.as_raw_fd(), 1) } >= 0, "dup2 failed");
    let restore = FdRestore(saved);

    f();

    drop(restore); // flushes + restores fd 1
    drop(file);

    let bytes = std::fs::read(&path).expect("read capture file");
    let _ = std::fs::remove_file(&path);
    drop(guard);
    bytes
}

/// Same as [`capture_stdout`], but fd 1 is a **pipe** instead of a regular
/// file.  A pipe and a file give libc's `stdout` different buffering
/// characteristics, so this exercises the stream-state axis as well.
pub fn capture_stdout_via_pipe<F: FnOnce()>(f: F) -> Vec<u8> {
    let guard = CAPTURE_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    use std::io::Write;
    let _ = std::io::stdout().flush();
    unsafe { fflush(std::ptr::null_mut()) };

    let mut fds = [0 as c_int; 2];
    assert!(unsafe { pipe(fds.as_mut_ptr()) } == 0, "pipe() failed");
    let (rd, wr) = (fds[0], fds[1]);

    let saved = unsafe { dup(1) };
    assert!(saved >= 0, "dup(1) failed");
    assert!(unsafe { dup2(wr, 1) } >= 0, "dup2 failed");

    {
        let _restore = FdRestore(saved);
        f();
    }
    unsafe { close(wr) };

    let mut out = Vec::new();
    let mut buf = [0u8; 4096];
    loop {
        let n = unsafe { read(rd, buf.as_mut_ptr(), buf.len()) };
        if n <= 0 {
            break;
        }
        out.extend_from_slice(&buf[..n as usize]);
    }
    unsafe { close(rd) };

    drop(guard);
    out
}

/// Like [`capture_stdout`] but writes into `path` (kept for inspection) instead
/// of returning the bytes; useful for very large transcripts.
pub fn capture_stdout_to_path<F: FnOnce()>(path: &std::path::Path, f: F) -> u64 {
    let guard = CAPTURE_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    use std::io::Write;
    let _ = std::io::stdout().flush();
    unsafe { fflush(std::ptr::null_mut()) };

    let file = std::fs::File::create(path).expect("create capture file");
    let saved = unsafe { dup(1) };
    assert!(saved >= 0, "dup(1) failed");
    assert!(unsafe { dup2(file.as_raw_fd(), 1) } >= 0, "dup2 failed");
    {
        let _restore = FdRestore(saved);
        f();
    }
    let len = file.metadata().map(|m| m.len()).unwrap_or(0);
    drop(file);
    drop(guard);
    len
}

/// FNV-1a over the captured stdout, computed by a reader thread draining a
/// pipe, so arbitrarily large transcripts can be compared without buffering
/// them in memory or on disk.  Returns `(hash, byte_count)`.
pub fn hash_stdout<F: FnOnce()>(f: F) -> (u64, u64) {
    let guard = CAPTURE_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    use std::io::Write;
    let _ = std::io::stdout().flush();
    unsafe { fflush(std::ptr::null_mut()) };

    let mut fds = [0 as c_int; 2];
    assert!(unsafe { pipe(fds.as_mut_ptr()) } == 0, "pipe() failed");
    let (rd, wr) = (fds[0], fds[1]);

    // Drain + hash concurrently; otherwise a full pipe would deadlock the
    // writer.
    let reader = std::thread::spawn(move || {
        let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
        let mut count: u64 = 0;
        let mut buf = vec![0u8; 1 << 16];
        loop {
            let n = unsafe { read(rd, buf.as_mut_ptr(), buf.len()) };
            if n <= 0 {
                break;
            }
            for &b in &buf[..n as usize] {
                hash ^= b as u64;
                hash = hash.wrapping_mul(0x100_0000_01b3);
            }
            count += n as u64;
        }
        unsafe { close(rd) };
        (hash, count)
    });

    let saved = unsafe { dup(1) };
    assert!(saved >= 0, "dup(1) failed");
    assert!(unsafe { dup2(wr, 1) } >= 0, "dup2 failed");
    // fd 1 is now the only write end; closing it below signals EOF.
    unsafe { close(wr) };

    {
        let _restore = FdRestore(saved);
        f();
    }

    let out = reader.join().expect("reader thread");
    drop(guard);
    out
}

/// FNV-1a of an arbitrary byte slice, matching [`hash_stdout`].
pub fn fnv1a(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for &b in bytes {
        hash ^= b as u64;
        hash = hash.wrapping_mul(0x100_0000_01b3);
    }
    hash
}

// ---------------------------------------------------------------------------
// Differential assertions
// ---------------------------------------------------------------------------

fn show(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).escape_debug().to_string()
}

/// Calls `driver(x)` in both libraries and asserts byte-identical stdout.
/// Returns the (shared) output bytes.
pub fn assert_same(x: i32) -> Vec<u8> {
    let im = impls();
    let out_c = capture_stdout(|| (im.c.driver)(x));
    let out_r = capture_stdout(|| (im.rust.driver)(x));
    assert_eq!(
        out_c,
        out_r,
        "driver({x}) [0x{:08x}] diverged:\n  C    = \"{}\"\n  Rust = \"{}\"",
        x as u32,
        show(&out_c),
        show(&out_r)
    );
    // The C code is the ground truth; also pin down the exact expected text so
    // a silent change on BOTH sides cannot pass unnoticed.
    let expected = format!("{}\n", x.wrapping_mul(2).wrapping_add(300));
    assert_eq!(
        String::from_utf8_lossy(&out_c),
        expected,
        "C output for driver({x}) is not the expected line"
    );
    out_c
}

/// Same as [`assert_same`] but through the wide-argument (`i64`) caller
/// signature, so the upper 32 bits of the argument register carry `hi`.
pub fn assert_same_wide(raw: i64) -> Vec<u8> {
    let im = impls();
    let out_c = capture_stdout(|| (im.c.driver_wide)(raw));
    let out_r = capture_stdout(|| (im.rust.driver_wide)(raw));
    assert_eq!(
        out_c,
        out_r,
        "driver_wide(0x{:016x}) diverged:\n  C    = \"{}\"\n  Rust = \"{}\"",
        raw as u64,
        show(&out_c),
        show(&out_r)
    );
    out_c
}

/// Calls `driver` for every value in `xs` back-to-back inside a single capture
/// (per implementation) and compares the whole transcript.
pub fn assert_same_transcript(xs: &[i32]) -> Vec<u8> {
    let im = impls();
    let out_c = capture_stdout(|| {
        for &x in xs {
            (im.c.driver)(x);
        }
    });
    let out_r = capture_stdout(|| {
        for &x in xs {
            (im.rust.driver)(x);
        }
    });
    assert_eq!(
        out_c,
        out_r,
        "transcript of {} calls diverged:\n  C    = \"{}\"\n  Rust = \"{}\"",
        xs.len(),
        show(&out_c),
        show(&out_r)
    );
    let expected: String = xs
        .iter()
        .map(|x| format!("{}\n", x.wrapping_mul(2).wrapping_add(300)))
        .collect();
    assert_eq!(String::from_utf8_lossy(&out_c), expected);
    out_c
}

// ---------------------------------------------------------------------------
// Row runner
// ---------------------------------------------------------------------------

/// Runs the rows of CONFIGS.md / ERRORS.md as sub-cases of a SINGLE `#[test]`
/// so that no other libtest thread can write to the redirected stdout, while
/// still reporting every failing row instead of stopping at the first one.
pub struct RowRunner {
    label: &'static str,
    ok: Vec<String>,
    failures: Vec<String>,
}

impl RowRunner {
    pub fn new(label: &'static str) -> Self {
        // Silence the default panic hook's stderr spew for the rows we catch;
        // the collected messages are reported by `finish()` instead.
        RowRunner {
            label,
            ok: Vec::new(),
            failures: Vec::new(),
        }
    }

    pub fn row<F: FnOnce()>(&mut self, name: &'static str, f: F) {
        let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
        match res {
            Ok(()) => self.ok.push(name.to_string()),
            Err(payload) => {
                let msg = if let Some(s) = payload.downcast_ref::<&str>() {
                    (*s).to_string()
                } else if let Some(s) = payload.downcast_ref::<String>() {
                    s.clone()
                } else {
                    "<non-string panic payload>".to_string()
                };
                self.failures.push(format!("[{name}] {msg}"));
            }
        }
    }

    pub fn finish(self) {
        println!(
            "{}: {} rows passed: {}",
            self.label,
            self.ok.len(),
            self.ok.join(", ")
        );
        if !self.failures.is_empty() {
            panic!(
                "{}: {} of {} rows FAILED:\n\n{}",
                self.label,
                self.failures.len(),
                self.ok.len() + self.failures.len(),
                self.failures.join("\n\n")
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (SplitMix64) — fixed seeds keep failures reproducible
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

    pub fn next_i32(&mut self) -> i32 {
        (self.next_u64() >> 32) as u32 as i32
    }

    /// Uniform-ish value in the inclusive range `[lo, hi]`.
    pub fn in_range(&mut self, lo: i64, hi: i64) -> i32 {
        assert!(lo <= hi);
        let span = (hi - lo) as u64 + 1;
        let v = lo + (self.next_u64() % span) as i64;
        v as i32
    }
}
