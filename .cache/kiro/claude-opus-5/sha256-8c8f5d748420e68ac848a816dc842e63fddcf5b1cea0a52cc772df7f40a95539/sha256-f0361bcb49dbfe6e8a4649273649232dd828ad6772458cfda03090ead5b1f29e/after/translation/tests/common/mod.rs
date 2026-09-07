//! Shared differential-test harness.
//!
//! Loads BOTH shared objects — the C `libdriver.so` and the Rust `libdriver.so` —
//! through `libloading` and calls them only through their exported `extern "C"`
//! symbols. The Rust crate is never linked or called directly, so the
//! `#[unsafe(no_mangle)]` export wrappers are part of what is under test.
//!
//! Every public function in this library returns `void` and communicates solely
//! by writing to the process's `stdout` `FILE*`. The differential observation is
//! therefore the exact byte stream on file descriptor 1, captured by
//! temporarily `dup2`-ing a scratch file over it.

#![allow(dead_code)]

use std::ffi::{c_char, c_int, c_void};
use std::io::{Read, Seek, SeekFrom};
use std::os::fd::AsRawFd;
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, OnceLock};

use libloading::{Library, Symbol};

// ---------------------------------------------------------------------------
// libc pieces we need. Declared directly so the crate needs no `libc` dep.
// ---------------------------------------------------------------------------

unsafe extern "C" {
    /// `fflush(NULL)` flushes *all* open output streams (POSIX), which avoids
    /// needing to name glibc's `stdout` global.
    fn fflush(stream: *mut c_void) -> c_int;
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn write(fd: c_int, buf: *const c_void, count: usize) -> isize;
}

// ---------------------------------------------------------------------------
// Which implementation a test is talking to.
// ---------------------------------------------------------------------------

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

pub const BOTH: [Impl; 2] = [Impl::C, Impl::Rust];

// ---------------------------------------------------------------------------
// Library loading.
// ---------------------------------------------------------------------------

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR is <root>/translation
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest
        .parent()
        .expect("translation/ must have a parent")
        .to_path_buf()
}

fn c_so_path() -> PathBuf {
    workspace_root().join("c_src/build/libdriver.so")
}

pub fn rust_so_path() -> PathBuf {
    // Load the cdylib built for the profile the test itself was built with, so
    // `cargo test` and `cargo test --release` each exercise a fresh library.
    let dir = if cfg!(debug_assertions) { "debug" } else { "release" };
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("target/{dir}/libdriver.so"))
}

pub fn c_so_path_pub() -> PathBuf {
    c_so_path()
}

struct Libs {
    c: Library,
    rust: Library,
}

// SAFETY: the loaded libraries are only ever used behind `CALL_LOCK`, and the
// functions they export are re-entrant plain `puts` wrappers.
unsafe impl Send for Libs {}
unsafe impl Sync for Libs {}

fn libs() -> &'static Libs {
    static LIBS: OnceLock<Libs> = OnceLock::new();
    LIBS.get_or_init(|| {
        let c_path = c_so_path();
        let rust_path = rust_so_path();
        assert!(
            c_path.exists(),
            "C shared library not built: {}\nRun: cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            c_path.display()
        );
        assert!(
            rust_path.exists(),
            "Rust shared library not built: {}\nRun: cd translation && cargo build{}",
            rust_path.display(),
            if cfg!(debug_assertions) { "" } else { " --release" }
        );
        // SAFETY: both paths point at plain C-ABI shared objects with no
        // library initialisers of consequence.
        unsafe {
            Libs {
                c: Library::new(&c_path)
                    .unwrap_or_else(|e| panic!("dlopen {}: {e}", c_path.display())),
                rust: Library::new(&rust_path)
                    .unwrap_or_else(|e| panic!("dlopen {}: {e}", rust_path.display())),
            }
        }
    })
}

fn lib_for(which: Impl) -> &'static Library {
    match which {
        Impl::C => &libs().c,
        Impl::Rust => &libs().rust,
    }
}

/// `void f(const char *)`
type FnStr = unsafe extern "C" fn(*const c_char);
/// `void f(void)`
type FnVoid = unsafe extern "C" fn();

pub fn sym_print_line(which: Impl) -> Symbol<'static, FnStr> {
    // SAFETY: signature matches `void printLine(const char *)`.
    unsafe { lib_for(which).get(b"printLine\0") }
        .unwrap_or_else(|e| panic!("{} lib: printLine not found: {e}", which.name()))
}

pub fn sym_void(which: Impl, name: &str) -> Symbol<'static, FnVoid> {
    let mut key = name.as_bytes().to_vec();
    key.push(0);
    // SAFETY: signature matches `void f(void)`.
    unsafe { lib_for(which).get(&key) }
        .unwrap_or_else(|e| panic!("{} lib: {name} not found: {e}", which.name()))
}

/// Is `name` resolvable at all in `which`'s dynamic symbol table?
pub fn has_symbol(which: Impl, name: &str) -> bool {
    let mut key = name.as_bytes().to_vec();
    key.push(0);
    // SAFETY: we only test resolvability; the symbol is never called.
    unsafe { lib_for(which).get::<*const c_void>(&key) }.is_ok()
}

// ---------------------------------------------------------------------------
// stdout capture.
//
// fd 1 redirection is process-global, and the Rust test harness runs tests on
// multiple threads, so every capture must hold this lock.
// ---------------------------------------------------------------------------

fn call_lock() -> MutexGuard<'static, ()> {
    static LOCK: Mutex<()> = Mutex::new(());
    LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

/// Run `body`, returning every byte it wrote to fd 1.
///
/// Any C stream buffer is flushed both before redirecting (so unrelated output
/// does not leak into the capture) and after `body` returns (so buffered library
/// output is included).
pub fn capture<R>(body: impl FnOnce() -> R) -> (R, Vec<u8>) {
    let _guard = call_lock();
    capture_locked(body)
}

fn capture_locked<R>(body: impl FnOnce() -> R) -> (R, Vec<u8>) {
    // Flush anything already pending so it lands on the real stdout.
    let _ = std::io::Write::flush(&mut std::io::stdout());
    unsafe { fflush(std::ptr::null_mut()) };

    let mut scratch = tempfile();
    let saved = unsafe { dup(1) };
    assert!(saved >= 0, "dup(1) failed");
    assert!(
        unsafe { dup2(scratch.as_raw_fd(), 1) } >= 0,
        "dup2 onto fd 1 failed"
    );

    let result = body();

    // Flush the library's own buffered writes while fd 1 is still redirected.
    unsafe { fflush(std::ptr::null_mut()) };

    assert!(unsafe { dup2(saved, 1) } >= 0, "restoring fd 1 failed");
    unsafe { close(saved) };

    let mut bytes = Vec::new();
    scratch.seek(SeekFrom::Start(0)).expect("seek scratch");
    scratch.read_to_end(&mut bytes).expect("read scratch");
    (result, bytes)
}

/// Write raw bytes straight to fd 1, bypassing every buffer — used to test
/// interleaving with a consumer's own output.
pub fn raw_write_stdout(bytes: &[u8]) {
    let mut off = 0;
    while off < bytes.len() {
        let n = unsafe {
            write(
                1,
                bytes[off..].as_ptr() as *const c_void,
                bytes.len() - off,
            )
        };
        assert!(n > 0, "write to fd 1 failed");
        off += n as usize;
    }
}

fn tempfile() -> std::fs::File {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!(
        "driver_difftest_{}_{}_{}.out",
        std::process::id(),
        n,
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));
    let file = std::fs::OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(true)
        .open(&path)
        .expect("create scratch file");
    // Unlink immediately; the open fd keeps it alive and it cannot leak.
    let _ = std::fs::remove_file(&path);
    file
}

// ---------------------------------------------------------------------------
// Differential drivers.
// ---------------------------------------------------------------------------

fn describe(bytes: &[u8]) -> String {
    const LIMIT: usize = 160;
    let shown: String = bytes
        .iter()
        .take(LIMIT)
        .map(|&b| match b {
            b'\n' => "\\n".to_string(),
            b'\t' => "\\t".to_string(),
            0x20..=0x7e => (b as char).to_string(),
            _ => format!("\\x{b:02x}"),
        })
        .collect();
    if bytes.len() > LIMIT {
        format!("{} bytes: \"{shown}\"…", bytes.len())
    } else {
        format!("{} bytes: \"{shown}\"", bytes.len())
    }
}

/// Assert the two captures are byte-identical.
pub fn assert_same(context: &str, c_out: &[u8], rust_out: &[u8]) {
    if c_out != rust_out {
        let first_diff = c_out
            .iter()
            .zip(rust_out.iter())
            .position(|(a, b)| a != b)
            .unwrap_or_else(|| c_out.len().min(rust_out.len()));
        panic!(
            "stdout divergence [{context}]\n  first differing byte index: {first_diff}\n  \
             C   : {}\n  Rust: {}",
            describe(c_out),
            describe(rust_out)
        );
    }
}

/// Run the same closure against each implementation and compare the stdout
/// bytes. The closure receives the `Impl` so it can resolve its own symbols.
pub fn diff(context: &str, mut body: impl FnMut(Impl)) {
    let _guard = call_lock();
    let (_, c_out) = capture_locked(|| body(Impl::C));
    let (_, rust_out) = capture_locked(|| body(Impl::Rust));
    assert_same(context, &c_out, &rust_out);
}

/// `printLine(bytes.as_ptr())` differentially. `bytes` must contain its own
/// terminating NUL.
pub fn diff_print_line_raw(context: &str, bytes: &[u8]) {
    assert_eq!(
        bytes.last().copied(),
        Some(0u8),
        "diff_print_line_raw needs a NUL-terminated buffer"
    );
    diff(context, |which| {
        let f = sym_print_line(which);
        unsafe { f(bytes.as_ptr() as *const c_char) };
    });
}

/// `printLine` on a payload that carries no NUL of its own; one is appended.
pub fn diff_print_line(context: &str, payload: &[u8]) {
    assert!(
        !payload.contains(&0),
        "use diff_print_line_raw for payloads containing NUL"
    );
    let mut buf = payload.to_vec();
    buf.push(0);
    diff_print_line_raw(context, &buf);
}

/// `printLine(NULL)` differentially.
pub fn diff_print_line_null(context: &str) {
    diff(context, |which| {
        let f = sym_print_line(which);
        unsafe { f(std::ptr::null()) };
    });
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (SplitMix64) so every randomized row is reproducible.
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

    /// Uniform in `0..n`.
    pub fn below(&mut self, n: usize) -> usize {
        assert!(n > 0);
        (self.next_u64() % n as u64) as usize
    }

    /// Uniform in `lo..=hi`.
    pub fn range(&mut self, lo: usize, hi: usize) -> usize {
        assert!(lo <= hi);
        lo + self.below(hi - lo + 1)
    }

    /// A non-NUL byte, uniform over `0x01..=0xFF`.
    pub fn nonzero_byte(&mut self) -> u8 {
        (self.range(1, 255)) as u8
    }

    /// `len` arbitrary non-NUL bytes.
    pub fn bytes(&mut self, len: usize) -> Vec<u8> {
        (0..len).map(|_| self.nonzero_byte()).collect()
    }

    /// `len` printable-ASCII bytes (`0x20..=0x7e`).
    pub fn ascii(&mut self, len: usize) -> Vec<u8> {
        (0..len).map(|_| self.range(0x20, 0x7e) as u8).collect()
    }

    /// Arbitrary non-NUL bytes with a random length in `lo..=hi`.
    pub fn bytes_len(&mut self, lo: usize, hi: usize) -> Vec<u8> {
        let len = self.range(lo, hi);
        self.bytes(len)
    }

    /// Printable ASCII with a random length in `lo..=hi`.
    pub fn ascii_len(&mut self, lo: usize, hi: usize) -> Vec<u8> {
        let len = self.range(lo, hi);
        self.ascii(len)
    }
}
