//! Shared differential-test harness.
//!
//! Both the C `.so` and the Rust `.so` are loaded with `libloading` and called
//! **only** through their exported `driver` symbol — never by linking the Rust
//! crate directly. That way the `#[no_mangle] extern "C"` wrapper is part of
//! what is under test, exactly as an external consumer would see it.
//!
//! `driver` communicates solely by writing to the C runtime's `stdout`, so the
//! observable output is captured at the file-descriptor level: fd 1 is
//! temporarily redirected, the library is invoked, all C streams are flushed
//! with `fflush(NULL)`, and fd 1 is restored. Redirection is process-global, so
//! every capture is serialised on `FD_LOCK`.

#![allow(dead_code)]

use std::ffi::c_void;
use std::io::Read;
use std::os::fd::AsRawFd;
use std::os::raw::c_int;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    /// `fflush(NULL)` flushes *every* open C output stream, which covers the
    /// `stdout` used by both shared objects.
    fn fflush(stream: *mut c_void) -> c_int;
}

const STDOUT_FD: c_int = 1;

/// Serialises the process-global fd-1 redirection used by every capture.
pub fn fd_lock() -> &'static Mutex<()> {
    static L: OnceLock<Mutex<()>> = OnceLock::new();
    L.get_or_init(|| Mutex::new(()))
}

// ---------------------------------------------------------------------------
// Library handles
// ---------------------------------------------------------------------------

/// A dynamically loaded `libdriver.so` (either the C one or the Rust one).
pub struct Lib {
    lib: libloading::Library,
    pub path: PathBuf,
}

impl Lib {
    pub fn open(path: &Path) -> Lib {
        assert!(
            path.exists(),
            "shared object not found: {}\n\
             build the C lib with:  cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .\n\
             build the Rust lib with:  cd translation && cargo build --release",
            path.display()
        );
        // SAFETY: loading a plain C shared object with no initialisers that
        // could run arbitrary code beyond libc's own.
        let lib = unsafe { libloading::Library::new(path) }
            .unwrap_or_else(|e| panic!("dlopen({}) failed: {e}", path.display()));
        Lib {
            lib,
            path: path.to_path_buf(),
        }
    }

    /// Call the exported `void driver(int)`.
    pub fn driver(&self, x: i32) {
        // SAFETY: signature matches `void driver(int)` from `driver.h`.
        unsafe {
            let f: libloading::Symbol<unsafe extern "C" fn(c_int)> = self
                .lib
                .get(b"driver\0")
                .unwrap_or_else(|e| panic!("dlsym(driver) in {}: {e}", self.path.display()));
            f(x as c_int);
        }
    }

    /// Call the exported `driver` through a *64-bit* argument type so the high
    /// half of the argument register carries garbage. Used by `ERRORS.md` row
    /// E7 to check that both callees truncate to `int` identically.
    pub fn driver_dirty_arg(&self, packed: i64) {
        // SAFETY: deliberate ABI-level test; the callee reads only the low 32
        // bits of the first integer argument register in the SysV ABI, which is
        // what both implementations do.
        unsafe {
            let f: libloading::Symbol<unsafe extern "C" fn(i64)> = self
                .lib
                .get(b"driver\0")
                .unwrap_or_else(|e| panic!("dlsym(driver) in {}: {e}", self.path.display()));
            f(packed);
        }
    }

    /// Resolve `driver` to a bare function pointer.
    ///
    /// Needed by the `fork`-based tests: `dlsym` must not be called in a forked
    /// child (it can take the loader lock), so the address is resolved in the
    /// parent and only the raw pointer is used afterwards.
    pub fn raw_driver(&self) -> unsafe extern "C" fn(c_int) {
        // SAFETY: signature matches `void driver(int)`.
        unsafe {
            let f: libloading::Symbol<unsafe extern "C" fn(c_int)> = self
                .lib
                .get(b"driver\0")
                .unwrap_or_else(|e| panic!("dlsym(driver) in {}: {e}", self.path.display()));
            *f
        }
    }

    /// True when the given symbol name is exported by this `.so`.
    pub fn has_symbol(&self, name: &[u8]) -> bool {
        let mut z = name.to_vec();
        z.push(0);
        // SAFETY: only probing for presence; the pointer is never called.
        unsafe {
            self.lib
                .get::<*const c_void>(&z)
                .map(|_| true)
                .unwrap_or(false)
        }
    }
}

// ---------------------------------------------------------------------------
// Locating the two shared objects
// ---------------------------------------------------------------------------

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

pub fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("C_DRIVER_SO") {
        return PathBuf::from(p);
    }
    manifest_dir()
        .parent()
        .expect("manifest dir has a parent")
        .join("c_src/build/libdriver.so")
}

pub fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_DRIVER_SO") {
        return PathBuf::from(p);
    }
    manifest_dir().join("target/release/libdriver.so")
}

/// The C and Rust libraries, loaded once per test binary.
pub struct Pair {
    pub c: Lib,
    pub rust: Lib,
}

pub fn pair() -> &'static Pair {
    static P: OnceLock<Pair> = OnceLock::new();
    P.get_or_init(|| Pair {
        c: Lib::open(&c_so_path()),
        rust: Lib::open(&rust_so_path()),
    })
}

// ---------------------------------------------------------------------------
// stdout capture
// ---------------------------------------------------------------------------

/// Redirect fd 1 into a fresh temp file, run `body`, flush all C streams,
/// restore fd 1, and return the bytes written.
///
/// The caller must already hold [`fd_lock`].
fn capture_to_regular_file<F: FnOnce()>(body: F) -> Vec<u8> {
    let path = std::env::temp_dir().join(format!(
        "driver_capture_{}_{:?}.out",
        std::process::id(),
        std::thread::current().id()
    ));
    let file = std::fs::File::create(&path).expect("create temp capture file");

    // SAFETY: raw fd juggling; every fd obtained is closed on the way out.
    let saved = unsafe {
        fflush(std::ptr::null_mut());
        let saved = dup(STDOUT_FD);
        assert!(saved >= 0, "dup(1) failed");
        assert!(dup2(file.as_raw_fd(), STDOUT_FD) >= 0, "dup2 failed");
        saved
    };

    // Run the library call with fd 1 pointing at the temp file. Panics are
    // caught so fd 1 is always restored before the panic is re-raised.
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(body));

    // SAFETY: restore fd 1 and release the duplicate.
    unsafe {
        fflush(std::ptr::null_mut());
        assert!(dup2(saved, STDOUT_FD) >= 0, "dup2 restore failed");
        close(saved);
    }

    let bytes = std::fs::read(&path).expect("read temp capture file");
    let _ = std::fs::remove_file(&path);

    if let Err(p) = outcome {
        std::panic::resume_unwind(p);
    }
    bytes
}

/// Redirect fd 1 into a pipe, run `body`, and return the bytes. libc gives a
/// pipe different default buffering than a regular file, so this exercises the
/// other buffering mode (`CONFIGS.md` row C12). A reader thread drains the pipe
/// so large outputs cannot deadlock on the 64 KiB pipe capacity.
///
/// The caller must already hold [`fd_lock`].
fn capture_to_pipe<F: FnOnce()>(body: F) -> Vec<u8> {
    let (mut rx, tx) = std::io::pipe().expect("create pipe");

    // SAFETY: raw fd juggling, mirrored by the restore below.
    let saved = unsafe {
        fflush(std::ptr::null_mut());
        let saved = dup(STDOUT_FD);
        assert!(saved >= 0, "dup(1) failed");
        assert!(dup2(tx.as_raw_fd(), STDOUT_FD) >= 0, "dup2 failed");
        saved
    };

    let reader = std::thread::spawn(move || {
        let mut buf = Vec::new();
        let _ = rx.read_to_end(&mut buf);
        buf
    });

    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(body));

    // SAFETY: restore fd 1, then drop the write end so the reader sees EOF.
    unsafe {
        fflush(std::ptr::null_mut());
        assert!(dup2(saved, STDOUT_FD) >= 0, "dup2 restore failed");
        close(saved);
    }
    drop(tx);

    let bytes = reader.join().expect("pipe reader thread");

    if let Err(p) = outcome {
        std::panic::resume_unwind(p);
    }
    bytes
}

/// Which stdout backing the capture should use.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Sink {
    RegularFile,
    Pipe,
}

/// Capture the stdout bytes produced by `body` using the requested sink.
pub fn capture<F: FnOnce()>(sink: Sink, body: F) -> Vec<u8> {
    match sink {
        Sink::RegularFile => capture_to_regular_file(body),
        Sink::Pipe => capture_to_pipe(body),
    }
}

// ---------------------------------------------------------------------------
// Differential assertions
// ---------------------------------------------------------------------------

fn show(b: &[u8]) -> String {
    let s = String::from_utf8_lossy(b);
    if s.len() <= 400 {
        return s.into_owned();
    }
    format!("{}…[{} bytes total]…{}", &s[..200], b.len(), &s[s.len() - 100..])
}

fn first_difference(a: &[u8], b: &[u8]) -> Option<usize> {
    let n = a.len().min(b.len());
    (0..n).find(|&i| a[i] != b[i]).or({
        if a.len() == b.len() {
            None
        } else {
            Some(n)
        }
    })
}

/// Run `driver(x)` in both libraries under the given sink and assert the
/// captured stdout bytes are identical.
pub fn assert_same(x: i32, sink: Sink, ctx: &str) -> Vec<u8> {
    let p = pair();
    let _g = fd_lock().lock().unwrap();
    let c_out = capture(sink, || p.c.driver(x));
    let r_out = capture(sink, || p.rust.driver(x));
    drop(_g);
    compare(&c_out, &r_out, &format!("{ctx}: driver({x}) via {sink:?}"));
    c_out
}

/// Assert the `driver_dirty_arg` variants agree (`ERRORS.md` row E7).
pub fn assert_same_dirty(packed: i64, ctx: &str) {
    let p = pair();
    let _g = fd_lock().lock().unwrap();
    let c_out = capture(Sink::RegularFile, || p.c.driver_dirty_arg(packed));
    let r_out = capture(Sink::RegularFile, || p.rust.driver_dirty_arg(packed));
    drop(_g);
    compare(
        &c_out,
        &r_out,
        &format!("{ctx}: driver(packed=0x{packed:016x})"),
    );
}

/// Run an arbitrary sequence of calls against both libraries and compare the
/// concatenated output. Used for statelessness / interleaving rows.
pub fn assert_same_sequence(xs: &[i32], sink: Sink, ctx: &str) {
    let p = pair();
    let _g = fd_lock().lock().unwrap();
    let c_out = capture(sink, || {
        for &x in xs {
            p.c.driver(x);
        }
    });
    let r_out = capture(sink, || {
        for &x in xs {
            p.rust.driver(x);
        }
    });
    drop(_g);
    compare(&c_out, &r_out, &format!("{ctx}: sequence {xs:?} via {sink:?}"));
}

/// The library only ever emits lines of the form `"<int> <int>\n"`. Anything
/// else in a capture means foreign bytes leaked into the redirected fd — in
/// practice libtest's own progress output when test threads run in parallel.
/// Detecting that explicitly avoids misreporting it as a translation divergence.
fn contamination(b: &[u8]) -> Option<String> {
    if b.is_empty() {
        return None;
    }
    if !b.ends_with(b"\n") {
        return Some(format!("capture does not end with a newline: {}", show(b)));
    }
    for line in b[..b.len() - 1].split(|&c| c == b'\n') {
        let ok = std::str::from_utf8(line).is_ok_and(|s| {
            let mut it = s.split(' ');
            match (it.next(), it.next(), it.next()) {
                (Some(a), Some(c), None) => {
                    !a.is_empty()
                        && !c.is_empty()
                        && a.parse::<i32>().is_ok()
                        && c.parse::<i32>().is_ok()
                }
                _ => false,
            }
        });
        if !ok {
            return Some(format!(
                "unexpected line in capture: {:?}",
                String::from_utf8_lossy(line)
            ));
        }
    }
    None
}

pub fn compare(c_out: &[u8], r_out: &[u8], ctx: &str) {
    for (who, bytes) in [("C", c_out), ("Rust", r_out)] {
        if let Some(why) = contamination(bytes) {
            panic!(
                "CAPTURE CONTAMINATED ({who} side) — {ctx}\n  {why}\n  \
                 fd 1 is process-global; run the tests serially \
                 (RUST_TEST_THREADS=1 or `-- --test-threads=1`). \
                 translation/.cargo/config.toml sets this automatically."
            );
        }
    }
    if c_out == r_out {
        return;
    }
    let at = first_difference(c_out, r_out);
    panic!(
        "DIVERGENCE — {ctx}\n  \
         first differing byte offset: {at:?}\n  \
         C   ({} bytes): {}\n  \
         Rust({} bytes): {}",
        c_out.len(),
        show(c_out),
        r_out.len(),
        show(r_out),
    );
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (fixed seed => reproducible property-style tests)
// ---------------------------------------------------------------------------

/// SplitMix64. Fixed seed, so every run draws the same sequence.
pub struct Rng(u64);

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
    /// Uniform in `lo..=hi`.
    pub fn range_i32(&mut self, lo: i32, hi: i32) -> i32 {
        debug_assert!(lo <= hi);
        let span = (hi as i64 - lo as i64 + 1) as u64;
        (lo as i64 + (self.next_u64() % span) as i64) as i32
    }
    pub fn next_i32(&mut self) -> i32 {
        self.next_u64() as u32 as i32
    }
}
