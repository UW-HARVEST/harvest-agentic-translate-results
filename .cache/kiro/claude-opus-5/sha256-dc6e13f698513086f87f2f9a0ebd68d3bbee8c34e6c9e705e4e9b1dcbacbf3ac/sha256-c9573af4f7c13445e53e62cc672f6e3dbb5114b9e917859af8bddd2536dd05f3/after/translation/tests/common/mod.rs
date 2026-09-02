//! Shared harness for the C-vs-Rust differential tests.
//!
//! Both libraries are loaded with `libloading` and driven **only** through
//! their exported `.so` symbols, so the `#[no_mangle]` wrappers are part of
//! what is under test. The Rust crate is never linked directly.
//!
//! `driver`/`print_foo` return `void` and communicate exclusively by writing to
//! the process-wide, libc-buffered `stdout`. So the "output" of a call is the
//! bytes it appends to fd 1. We capture those by temporarily `dup2`-ing a temp
//! file over fd 1, flushing every libc stream (`fflush(NULL)`) after the call,
//! and reading the file back.

#![allow(dead_code)]

use std::ffi::{c_char, c_int, c_uint, c_void};
use std::io::{Read, Seek, SeekFrom};
use std::path::PathBuf;

use libloading::{Library, Symbol};

// ---------------------------------------------------------------------------
// libc bits we need for stdout capture
// ---------------------------------------------------------------------------

extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    /// `fflush(NULL)` flushes *every* open output stream in the process,
    /// including the ones the C `.so` and the Rust `.so` write through.
    fn fflush(stream: *mut c_void) -> c_int;
    fn fork() -> c_int;
    fn waitpid(pid: c_int, status: *mut c_int, options: c_int) -> c_int;
    fn _exit(code: c_int) -> !;
}

const STDOUT_FILENO: c_int = 1;

// ---------------------------------------------------------------------------
// The struct under test
// ---------------------------------------------------------------------------

/// The 8 raw bytes of the C `foo_t`:
///
/// ```c
/// typedef struct { unsigned int x:2; unsigned int y:3; bool b:1; int z; } foo_t;
/// ```
///
/// gcc x86-64: `sizeof == 8`, `alignof == 4`; byte 0 holds the bit-fields,
/// bytes 1..=3 are padding, bytes 4..=7 hold `z` (little-endian).
pub const FOO_SIZE: usize = 8;

/// Build the raw bytes for a `foo_t` from a byte-0 bit-field unit, three
/// padding bytes and a signed `z`.
pub fn foo_bytes(byte0: u8, pad: [u8; 3], z: i32) -> [u8; FOO_SIZE] {
    let mut b = [0u8; FOO_SIZE];
    b[0] = byte0;
    b[1] = pad[0];
    b[2] = pad[1];
    b[3] = pad[2];
    b[4..8].copy_from_slice(&z.to_le_bytes());
    b
}

/// Pack `x`, `y`, `b` the way the C `driver` does: `x & 3`, `(y & 7) << 2`,
/// `(b & 1) << 5`.
pub fn pack_byte0(x: u32, y: u32, b: u8) -> u8 {
    ((x as u8) & 0x03) | (((y as u8) & 0x07) << 2) | ((b & 0x01) << 5)
}

// ---------------------------------------------------------------------------
// Locating and loading the two shared objects
// ---------------------------------------------------------------------------

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// `<workdir>/c_src/build/libdriver.so`, built by cmake.
pub fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("C_DRIVER_SO") {
        return PathBuf::from(p);
    }
    manifest_dir()
        .parent()
        .expect("manifest dir has a parent")
        .join("c_src/build/libdriver.so")
}

/// The Rust `cdylib`, in whichever target profile directory this test binary
/// itself lives in (`target/<profile>/deps/<test>` -> `target/<profile>`).
pub fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_DRIVER_SO") {
        return PathBuf::from(p);
    }
    let exe = std::env::current_exe().expect("current_exe");
    let profile_dir = exe
        .parent()
        .and_then(|deps| deps.parent())
        .expect("target/<profile>/deps/<test>");
    let candidate = profile_dir.join("libdriver.so");
    if candidate.exists() {
        return candidate;
    }
    for p in ["target/debug/libdriver.so", "target/release/libdriver.so"] {
        let c = manifest_dir().join(p);
        if c.exists() {
            return c;
        }
    }
    candidate
}

/// The two libraries, loaded simultaneously into this process.
pub struct Libs {
    pub c: Library,
    pub rust: Library,
}

pub type DriverFn = unsafe extern "C" fn(c_uint, c_uint, u8, c_int);
pub type PrintFooFn = unsafe extern "C" fn(*const u8);

impl Libs {
    pub fn load() -> Libs {
        let c_path = c_so_path();
        let rust_path = rust_so_path();
        assert!(
            c_path.exists(),
            "C shared library not found at {c_path:?}. Build it with:\n  \
             cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build ."
        );
        assert!(
            rust_path.exists(),
            "Rust shared library not found at {rust_path:?}. Build it with `cargo build`."
        );
        unsafe {
            Libs {
                c: Library::new(&c_path).expect("dlopen C libdriver.so"),
                rust: Library::new(&rust_path).expect("dlopen Rust libdriver.so"),
            }
        }
    }

    pub fn c_driver(&self) -> Symbol<'_, DriverFn> {
        unsafe { self.c.get(b"driver\0").expect("C exports `driver`") }
    }
    pub fn rust_driver(&self) -> Symbol<'_, DriverFn> {
        unsafe { self.rust.get(b"driver\0").expect("Rust exports `driver`") }
    }
    pub fn c_print_foo(&self) -> Symbol<'_, PrintFooFn> {
        unsafe { self.c.get(b"print_foo\0").expect("C exports `print_foo`") }
    }
    pub fn rust_print_foo(&self) -> Symbol<'_, PrintFooFn> {
        unsafe {
            self.rust
                .get(b"print_foo\0")
                .expect("Rust exports `print_foo`")
        }
    }
}

/// Process-wide lock: stdout redirection is a global mutation, so only one
/// capture may be in flight at a time.
static CAPTURE_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Run `f` with fd 1 redirected to a temp file, then return the bytes written.
///
/// Any libc stream buffering is defeated with `fflush(NULL)` before the fd is
/// restored, so a fully-buffered redirected stdout still yields its bytes.
pub fn capture_stdout<F: FnOnce()>(f: F) -> Vec<u8> {
    let _guard = CAPTURE_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    let mut tmp = tempfile();

    unsafe {
        // Flush anything already pending so it does not land in our capture.
        // Two separate buffers must be drained: libc's `FILE *stdout` (used by
        // both `.so`s) and Rust's own `std::io::Stdout` LineWriter (used by the
        // test harness for its progress output, which has no trailing newline
        // and would otherwise be flushed into our redirected fd).
        let _ = std::io::Write::flush(&mut std::io::stdout());
        fflush(std::ptr::null_mut());
        let saved = dup(STDOUT_FILENO);
        assert!(saved >= 0, "dup(1) failed");
        assert!(
            dup2(file_fd(&tmp), STDOUT_FILENO) >= 0,
            "dup2 onto stdout failed"
        );

        f();

        fflush(std::ptr::null_mut());
        assert!(dup2(saved, STDOUT_FILENO) >= 0, "restoring stdout failed");
        close(saved);
    }

    tmp.seek(SeekFrom::Start(0)).expect("seek temp file");
    let mut out = Vec::new();
    tmp.read_to_end(&mut out).expect("read temp file");
    out
}

fn file_fd(f: &std::fs::File) -> c_int {
    use std::os::unix::io::AsRawFd;
    f.as_raw_fd()
}

/// A unique, immediately-unlinked temp file.
fn tempfile() -> std::fs::File {
    use std::os::unix::fs::OpenOptionsExt;
    static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!(
        "driver-diff-{}-{}-{}.out",
        std::process::id(),
        n,
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.subsec_nanos())
            .unwrap_or(0)
    ));
    let f = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&path)
        .expect("create temp file");
    // Unlink immediately; the open fd keeps it alive.
    let _ = std::fs::remove_file(&path);
    f
}

// ---------------------------------------------------------------------------
// Differential helpers
// ---------------------------------------------------------------------------

/// Call `driver` in both libraries with identical arguments and return
/// `(c_output, rust_output)`.
pub fn diff_driver(libs: &Libs, x: u32, y: u32, b: u8, z: i32) -> (Vec<u8>, Vec<u8>) {
    let cf = libs.c_driver();
    let rf = libs.rust_driver();
    let c_out = capture_stdout(|| unsafe { cf(x, y, b, z) });
    let r_out = capture_stdout(|| unsafe { rf(x, y, b, z) });
    (c_out, r_out)
}

/// Assert that both `driver` implementations produce byte-identical output.
#[track_caller]
pub fn assert_driver_eq(libs: &Libs, x: u32, y: u32, b: u8, z: i32) {
    let (c_out, r_out) = diff_driver(libs, x, y, b, z);
    if c_out != r_out {
        panic!(
            "driver({x}, {y}, {b}, {z}) diverged\n  C   : {:?}\n  Rust: {:?}",
            String::from_utf8_lossy(&c_out),
            String::from_utf8_lossy(&r_out),
        );
    }
}

/// Call `print_foo` in both libraries on the identical 8 raw bytes.
pub fn diff_print_foo(libs: &Libs, raw: &[u8; FOO_SIZE]) -> (Vec<u8>, Vec<u8>) {
    let cf = libs.c_print_foo();
    let rf = libs.rust_print_foo();
    let c_out = capture_stdout(|| unsafe { cf(raw.as_ptr()) });
    let r_out = capture_stdout(|| unsafe { rf(raw.as_ptr()) });
    (c_out, r_out)
}

#[track_caller]
pub fn assert_print_foo_eq(libs: &Libs, raw: &[u8; FOO_SIZE]) {
    let (c_out, r_out) = diff_print_foo(libs, raw);
    if c_out != r_out {
        panic!(
            "print_foo({raw:02x?}) diverged\n  C   : {:?}\n  Rust: {:?}",
            String::from_utf8_lossy(&c_out),
            String::from_utf8_lossy(&r_out),
        );
    }
}

// ---------------------------------------------------------------------------
// Crash-parity helper (for the null-pointer / UB rows)
// ---------------------------------------------------------------------------

/// Outcome of running a call in a forked child.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Outcome {
    Exited(c_int),
    Signalled(c_int),
}

/// Run `f` in a forked child process and report how the child terminated.
///
/// Used for the rows whose "expected C result" is a fatal signal: we must
/// observe that the Rust behaves the same way without taking down the test
/// runner. `fork` in a child that only performs one FFI call and then `_exit`s
/// is safe here (no allocation, no locks taken across the fork in the child
/// path other than the one call under test).
pub fn run_in_child<F: FnOnce()>(f: F) -> Outcome {
    unsafe {
        // Flush first so buffered parent output is not duplicated by the child.
        fflush(std::ptr::null_mut());
        let pid = fork();
        assert!(pid >= 0, "fork failed");
        if pid == 0 {
            // The child's own printf output is irrelevant here (only its
            // termination status is under test) and would otherwise leak onto
            // the test runner's console.
            let devnull = std::fs::File::create("/dev/null");
            if let Ok(f) = devnull.as_ref() {
                dup2(file_fd(f), STDOUT_FILENO);
            }
            f();
            fflush(std::ptr::null_mut());
            _exit(0);
        }
        let mut status: c_int = 0;
        let r = waitpid(pid, &mut status, 0);
        assert_eq!(r, pid, "waitpid failed");
        // WIFEXITED / WEXITSTATUS / WTERMSIG, open-coded for glibc.
        if status & 0x7f == 0x7f {
            // stopped; treat as signalled by the stop signal
            Outcome::Signalled((status >> 8) & 0xff)
        } else if status & 0x7f == 0 {
            Outcome::Exited((status >> 8) & 0xff)
        } else {
            Outcome::Signalled(status & 0x7f)
        }
    }
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (fixed seed => reproducible property-style tests)
// ---------------------------------------------------------------------------

/// SplitMix64. Small, fast, no dependencies, and fully reproducible.
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
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    pub fn next_i32(&mut self) -> i32 {
        self.next_u32() as i32
    }
    pub fn next_u8(&mut self) -> u8 {
        (self.next_u64() >> 56) as u8
    }
    /// Uniform in `0..n` (n > 0).
    pub fn below(&mut self, n: u32) -> u32 {
        (((self.next_u32() as u64) * (n as u64)) >> 32) as u32
    }
    pub fn fill(&mut self, buf: &mut [u8]) {
        for b in buf.iter_mut() {
            *b = self.next_u8();
        }
    }
}

/// Keeps the unused-import lint quiet for `c_char` (used by callers that
/// declare their own printf-alike signatures).
pub type CChar = c_char;

// ---------------------------------------------------------------------------
// Batched differential execution
// ---------------------------------------------------------------------------

/// One call to make against a library.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Op {
    /// `driver(x, y, b, z)`
    Driver { x: u32, y: u32, b: u8, z: i32 },
    /// `print_foo(&raw)` on these exact 8 bytes.
    PrintFoo { raw: [u8; FOO_SIZE] },
    /// `print_foo` on a deliberately misaligned pointer to these 8 bytes.
    PrintFooMisaligned { raw: [u8; FOO_SIZE] },
}

impl Op {
    fn run(self, driver: &DriverFn, print_foo: &PrintFooFn) {
        match self {
            Op::Driver { x, y, b, z } => unsafe { driver(x, y, b, z) },
            Op::PrintFoo { raw } => unsafe { print_foo(raw.as_ptr()) },
            Op::PrintFooMisaligned { raw } => {
                // Place the 8 bytes at offset 1 of an over-aligned buffer so the
                // `int z` load at +4 is 1 mod 4.
                #[repr(align(8))]
                struct Aligned([u8; 16]);
                let mut buf = Aligned([0u8; 16]);
                buf.0[1..9].copy_from_slice(&raw);
                unsafe { print_foo(buf.0.as_ptr().add(1)) }
            }
        }
    }
}

/// Run a whole batch of ops against one library inside a single stdout capture.
fn run_batch(libs: &Libs, ops: &[Op], rust: bool) -> Vec<u8> {
    let d: DriverFn = if rust {
        *libs.rust_driver()
    } else {
        *libs.c_driver()
    };
    let p: PrintFooFn = if rust {
        *libs.rust_print_foo()
    } else {
        *libs.c_print_foo()
    };
    capture_stdout(|| {
        for op in ops {
            op.run(&d, &p);
        }
    })
}

/// Run `ops` against BOTH libraries (each in one capture) and assert the two
/// output streams are byte-identical. On divergence, re-runs the batch one op at
/// a time to report the first offending op precisely.
#[track_caller]
pub fn assert_batch_eq(libs: &Libs, ops: &[Op], label: &str) {
    let c_out = run_batch(libs, ops, false);
    let r_out = run_batch(libs, ops, true);
    if c_out == r_out {
        return;
    }
    // Pinpoint.
    for op in ops {
        let one = [*op];
        let c1 = run_batch(libs, &one, false);
        let r1 = run_batch(libs, &one, true);
        if c1 != r1 {
            panic!(
                "[{label}] divergence on {op:?}\n  C   : {:?}\n  Rust: {:?}",
                String::from_utf8_lossy(&c1),
                String::from_utf8_lossy(&r1),
            );
        }
    }
    panic!(
        "[{label}] batch output differed although every individual op matched \
         (ordering/buffering divergence)\n  C   len {}\n  Rust len {}",
        c_out.len(),
        r_out.len()
    );
}

/// Chunked variant, so a huge case count still uses a bounded number of
/// captures and bounded temp-file size.
#[track_caller]
pub fn assert_batch_eq_chunked(libs: &Libs, ops: &[Op], label: &str) {
    const CHUNK: usize = 4096;
    for (i, chunk) in ops.chunks(CHUNK).enumerate() {
        assert_batch_eq(libs, chunk, &format!("{label} chunk {i}"));
    }
}
