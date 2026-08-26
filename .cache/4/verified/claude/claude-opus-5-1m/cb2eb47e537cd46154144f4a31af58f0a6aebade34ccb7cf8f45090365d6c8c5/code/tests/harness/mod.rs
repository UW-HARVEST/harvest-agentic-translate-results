// Differential-test harness shared by the Phase B / C / D test binaries.
//
// Ground rule: the Rust implementation is NEVER called directly as a Rust
// function. Both the C `.so` and the Rust `.so` are loaded with `libloading`
// and driven purely through their exported C symbols, so the `#[no_mangle]`
// export wrappers are on trial too.

#![allow(dead_code)]

use libloading::os::unix::{Library, RTLD_LOCAL, RTLD_NOW};
use std::ffi::c_char;
use std::ffi::c_int;
use std::fs::File;
use std::io::Write as _;
use std::os::fd::AsRawFd;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::SystemTime;

// ---------------------------------------------------------------------------
// libc plumbing used only by the harness (fd juggling + stream flushing).
// These are test scaffolding, not part of the library under test.
// ---------------------------------------------------------------------------

unsafe extern "C" {
    /// `fflush(NULL)` flushes *every* open output stream, which is how the
    /// buffered libc `stdout` writes performed inside both `.so`s are forced
    /// out before the captured file is read back.
    fn fflush(stream: *mut std::ffi::c_void) -> c_int;
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
}

// ---------------------------------------------------------------------------
// Loading the two shared libraries
// ---------------------------------------------------------------------------

/// The four exported entry points of one shared library, resolved by symbol
/// name exactly as an external C consumer would resolve them.
pub struct DriverLib {
    /// Kept alive for the process lifetime; dropping it would unload the code.
    _lib: Library,
    pub which: &'static str,
    pub path: PathBuf,
    pub print_line: unsafe extern "C" fn(*const c_char),
    pub bad: unsafe extern "C" fn(),
    pub good: unsafe extern "C" fn(),
    pub driver: unsafe extern "C" fn(c_int),
}

// The handle is only ever used behind the global `FD_LOCK`, and dlsym'd code
// pointers are plain addresses.
unsafe impl Send for DriverLib {}
unsafe impl Sync for DriverLib {}

impl DriverLib {
    fn open(which: &'static str, path: PathBuf) -> DriverLib {
        assert!(
            path.exists(),
            "{which} shared library not found at {}.\n\
             Build the C side with:\n  \
             cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .\n\
             Build the Rust side with:\n  cargo build",
            path.display()
        );

        // RTLD_LOCAL (not RTLD_GLOBAL) is essential: both libraries export the
        // same four names, and the C `bad()` calls `printLine` through its PLT.
        // Keeping each object out of the global namespace guarantees the C
        // library's internal call resolves to the C `printLine` and never to
        // the Rust one, so the two implementations cannot contaminate each
        // other. RTLD_NOW surfaces any unresolved symbol immediately.
        let lib = unsafe { Library::open(Some(&path), RTLD_NOW | RTLD_LOCAL) }
            .unwrap_or_else(|e| panic!("dlopen({}) failed: {e}", path.display()));

        macro_rules! sym {
            ($name:literal, $ty:ty) => {{
                let s = unsafe { lib.get::<$ty>($name) }.unwrap_or_else(|e| {
                    panic!(
                        "{which} library {} does not export `{}`: {e}",
                        path.display(),
                        String::from_utf8_lossy(&$name[..$name.len() - 1]),
                    )
                });
                *s
            }};
        }

        let print_line = sym!(b"printLine\0", unsafe extern "C" fn(*const c_char));
        let bad = sym!(b"bad\0", unsafe extern "C" fn());
        let good = sym!(b"good\0", unsafe extern "C" fn());
        let driver = sym!(b"driver\0", unsafe extern "C" fn(c_int));

        DriverLib { _lib: lib, which, path, print_line, bad, good, driver }
    }

    // Thin, safe-to-call-under-capture wrappers.
    pub fn print_line(&self, nul_terminated: *const c_char) {
        unsafe { (self.print_line)(nul_terminated) }
    }
    pub fn call_bad(&self) {
        unsafe { (self.bad)() }
    }
    pub fn call_good(&self) {
        unsafe { (self.good)() }
    }
    pub fn call_driver(&self, use_good: c_int) {
        unsafe { (self.driver)(use_good) }
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_so_path() -> PathBuf {
    manifest_dir().join("c_src/build/libdriver.so")
}

/// Locate the Rust `cdylib` next to the running test executable
/// (`target/<profile>/deps/libdriver.so`), falling back to
/// `target/<profile>/libdriver.so`. Never assumes a hard-coded profile.
fn rust_so_path() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    let deps = exe.parent().expect("deps dir");
    let candidates = [
        deps.join("libdriver.so"),
        deps.parent().map(|p| p.join("libdriver.so")).unwrap_or_default(),
    ];
    for c in &candidates {
        if c.exists() {
            return c.clone();
        }
    }
    candidates[0].clone()
}

/// Newest modification time under `dir` (recursively), for files matching `ext`.
fn newest_mtime(dir: &Path, ext: &str) -> Option<SystemTime> {
    let mut newest: Option<SystemTime> = None;
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&d) else { continue };
        for entry in rd.flatten() {
            let p = entry.path();
            if p.is_dir() {
                stack.push(p);
            } else if p.extension().is_some_and(|e| e == ext) {
                if let Ok(m) = entry.metadata().and_then(|m| m.modified()) {
                    newest = Some(newest.map_or(m, |n| n.max(m)));
                }
            }
        }
    }
    newest
}

/// Refuse to run against a stale artifact.
///
/// `crate-type = ["cdylib"]` means integration tests never *link* the library,
/// so `cargo test` does **not** rebuild `libdriver.so`. Without this guard a run
/// after editing `src/` would silently re-test the previous binary and report a
/// vacuous pass. Verified empirically: touching `src/driver.rs` and running
/// `cargo test --no-run` leaves `target/*/deps/libdriver.so` untouched.
fn assert_artifacts_fresh(rust_so: &Path) {
    let so_time = match std::fs::metadata(rust_so).and_then(|m| m.modified()) {
        Ok(t) => t,
        Err(e) => panic!("cannot stat {}: {e}", rust_so.display()),
    };

    if let Some(src_time) = newest_mtime(&manifest_dir().join("src"), "rs") {
        assert!(
            src_time <= so_time,
            "\n\nSTALE ARTIFACT: {} is older than the newest file in src/.\n\
             `cargo test` does not rebuild a cdylib (integration tests never link \
             it), so this run would have tested the *previous* build and passed \
             vacuously.\n\nRun:\n\n    cargo build && RUST_TEST_THREADS=1 cargo test\n\n\
             or just ./verify.sh\n",
            rust_so.display()
        );
    }

    // The C reference must likewise be newer than c_src/.
    let c_so = c_so_path();
    if let (Ok(c_time), Some(c_src_time)) = (
        std::fs::metadata(&c_so).and_then(|m| m.modified()),
        newest_mtime(&manifest_dir().join("c_src/src"), "c"),
    ) {
        assert!(
            c_src_time <= c_time,
            "STALE ARTIFACT: {} is older than c_src/src; rebuild the C library",
            c_so.display()
        );
    }
}

static LIBS: OnceLock<(DriverLib, DriverLib)> = OnceLock::new();

/// `(c_library, rust_library)`.
pub fn libs() -> (&'static DriverLib, &'static DriverLib) {
    let (c, r) = LIBS.get_or_init(|| {
        let rust = rust_so_path();
        assert_artifacts_fresh(&rust);
        (
            DriverLib::open("C", c_so_path()),
            DriverLib::open("Rust", rust),
        )
    });
    (c, r)
}

pub fn c_so_file() -> PathBuf {
    c_so_path()
}
pub fn rust_so_file() -> PathBuf {
    rust_so_path()
}

// ---------------------------------------------------------------------------
// stdout capture
// ---------------------------------------------------------------------------

/// Serializes the fd-1 swap. Every capture in a test binary goes through this,
/// so concurrent captures cannot steal each other's output.
static FD_LOCK: Mutex<()> = Mutex::new(());
static CAPTURE_SEQ: AtomicU64 = AtomicU64::new(0);

/// Capturing works by temporarily pointing the *process's* file descriptor 1 at
/// a file, because that is the only way to observe what libc's buffered
/// `stdout` receives from inside a `.so`. libtest's own progress printer
/// (`test foo ... ok`) writes to that same descriptor from the runner thread,
/// so if tests ran concurrently those bytes would land in the capture and be
/// misreported as a divergence.
///
/// Rather than silently tolerate that, the harness *requires* serial execution
/// and fails loudly otherwise — a filtered/heuristic capture could hide a real
/// difference.
fn assert_serial_execution() {
    static CHECKED: OnceLock<()> = OnceLock::new();
    CHECKED.get_or_init(|| {
        let threads = std::env::var("RUST_TEST_THREADS").unwrap_or_default();
        assert_eq!(
            threads, "1",
            "\n\nThese differential tests redirect the process's file descriptor 1 \
             to capture what the C and Rust `.so`s write through libc's stdout, so \
             they must not run concurrently with libtest's own progress output.\n\
             Run them as:\n\n    \
             RUST_TEST_THREADS=1 cargo test\n\n\
             (got RUST_TEST_THREADS={threads:?})\n"
        );
    });
}

/// Run `f` with the process's file descriptor 1 redirected to a temporary
/// file, then return every byte it wrote.
///
/// Both libraries write through libc's buffered `stdout` (`puts`), so the
/// buffer is flushed *before* the swap (to keep unrelated output out) and again
/// *after* `f` (while fd 1 still points at the file) before restoring.
pub fn capture<F: FnOnce()>(f: F) -> Vec<u8> {
    assert_serial_execution();
    let guard = FD_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    // libtest prints `test <name> ... ` (no trailing newline) through Rust's
    // line-buffered `io::stdout()` before the test body runs, so those bytes are
    // still sitting in Rust's userspace buffer. Push them out to the *real* fd 1
    // now; otherwise the next flush would deposit them into the capture file.
    let _ = std::io::stdout().flush();

    let seq = CAPTURE_SEQ.fetch_add(1, Ordering::Relaxed);
    let mut path = std::env::temp_dir();
    path.push(format!("driver-diff-{}-{}.out", std::process::id(), seq));

    let file = File::create(&path).expect("create capture file");
    let file_fd = file.as_raw_fd();

    let out = unsafe {
        fflush(std::ptr::null_mut());
        let saved = dup(1);
        assert!(saved >= 0, "dup(1) failed");
        assert!(dup2(file_fd, 1) >= 0, "dup2 onto stdout failed");

        // Run the library code with stdout pointed at the file.
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));

        // Flush while fd 1 is still the file, then put the real stdout back
        // even if the call panicked.
        fflush(std::ptr::null_mut());
        dup2(saved, 1);
        close(saved);

        if let Err(payload) = result {
            drop(file);
            let _ = std::fs::remove_file(&path);
            drop(guard);
            std::panic::resume_unwind(payload);
        }

        let mut f = file;
        let _ = f.flush();
        drop(f);
        std::fs::read(&path).expect("read capture file")
    };

    let _ = std::fs::remove_file(&path);
    drop(guard);
    out
}

// ---------------------------------------------------------------------------
// The differential assertion
// ---------------------------------------------------------------------------

fn render(bytes: &[u8]) -> String {
    const LIMIT: usize = 160;
    let shown = &bytes[..bytes.len().min(LIMIT)];
    let mut s = String::new();
    for &b in shown {
        match b {
            b'\n' => s.push_str("\\n"),
            b'\r' => s.push_str("\\r"),
            b'\t' => s.push_str("\\t"),
            b'\\' => s.push_str("\\\\"),
            0x20..=0x7e => s.push(b as char),
            _ => s.push_str(&format!("\\x{b:02x}")),
        }
    }
    if bytes.len() > LIMIT {
        s.push_str(&format!("...(+{} bytes)", bytes.len() - LIMIT));
    }
    s
}

/// Drive the *same* closure against the C library and the Rust library, and
/// require the bytes they write to stdout to be identical.
pub fn diff<F: Fn(&DriverLib)>(label: &str, f: F) {
    let (c, r) = libs();
    let out_c = capture(|| f(c));
    let out_r = capture(|| f(r));

    if out_c != out_r {
        let first = out_c
            .iter()
            .zip(out_r.iter())
            .position(|(a, b)| a != b)
            .unwrap_or(out_c.len().min(out_r.len()));
        panic!(
            "DIVERGENCE [{label}]\n  \
             C   ({:>7} bytes): {}\n  \
             Rust({:>7} bytes): {}\n  \
             first difference at byte {first}",
            out_c.len(),
            render(&out_c),
            out_r.len(),
            render(&out_r),
        );
    }
}

/// Like [`diff`], but additionally pins the exact bytes both must produce.
///
/// Performs exactly two captures (one per library) so that rows driven with
/// thousands of randomized inputs stay fast.
pub fn diff_exact<F: Fn(&DriverLib)>(label: &str, expected: &[u8], f: F) {
    let (c, r) = libs();
    let out_c = capture(|| f(c));
    let out_r = capture(|| f(r));

    if out_c != out_r {
        let first = out_c
            .iter()
            .zip(out_r.iter())
            .position(|(a, b)| a != b)
            .unwrap_or(out_c.len().min(out_r.len()));
        panic!(
            "DIVERGENCE [{label}]\n  \
             C   ({:>7} bytes): {}\n  \
             Rust({:>7} bytes): {}\n  \
             first difference at byte {first}",
            out_c.len(),
            render(&out_c),
            out_r.len(),
            render(&out_r),
        );
    }

    assert_eq!(
        out_c,
        expected,
        "[{label}] C produced {} but the table says {}",
        render(&out_c),
        render(expected)
    );
    assert_eq!(
        out_r,
        expected,
        "[{label}] Rust produced {} but the table says {}",
        render(&out_r),
        render(expected)
    );
}

// ---------------------------------------------------------------------------
// Payload helpers
// ---------------------------------------------------------------------------

/// A NUL-terminated payload whose allocation has **no slack after the
/// terminator**, so any read past the NUL is a genuine heap over-read.
pub struct CBuf(Box<[u8]>);

impl CBuf {
    pub fn new(payload: &[u8]) -> CBuf {
        assert!(
            !payload.contains(&0),
            "payload must not contain an interior NUL"
        );
        let mut v = Vec::with_capacity(payload.len() + 1);
        v.extend_from_slice(payload);
        v.push(0);
        CBuf(v.into_boxed_slice())
    }
    pub fn as_ptr(&self) -> *const c_char {
        self.0.as_ptr().cast::<c_char>()
    }
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (SplitMix64) — fixed seeds keep every row reproducible.
// ---------------------------------------------------------------------------

pub const SEED: u64 = 0x5EED_1234_ABCD_EF01;

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
    /// Uniform in `0..n`.
    pub fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
    }
    /// Uniform in `lo..=hi`.
    pub fn range(&mut self, lo: usize, hi: usize) -> usize {
        lo + self.below(hi - lo + 1)
    }
    /// Random non-NUL byte (`0x01..=0xFF`).
    pub fn nonzero_byte(&mut self) -> u8 {
        1 + (self.next_u64() % 255) as u8
    }
    /// Random printable-ASCII byte (`0x20..=0x7E`).
    pub fn ascii_byte(&mut self) -> u8 {
        0x20 + (self.next_u64() % (0x7e - 0x20 + 1)) as u8
    }
    pub fn bytes_nonzero(&mut self, len: usize) -> Vec<u8> {
        (0..len).map(|_| self.nonzero_byte()).collect()
    }
    pub fn bytes_ascii(&mut self, len: usize) -> Vec<u8> {
        (0..len).map(|_| self.ascii_byte()).collect()
    }
}
