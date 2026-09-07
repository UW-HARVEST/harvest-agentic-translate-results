//! Shared differential-test harness.
//!
//! Loads BOTH shared libraries through `libloading` — the C `libdriver.so` built
//! by CMake and the Rust `libdriver.so` built by `cargo build --release` — and
//! calls the exported `driver` symbol in each. The Rust side is *never* called
//! as a Rust function; it is always reached through its `.so` export, so the
//! `#[no_mangle] extern "C"` wrapper is under test too.
//!
//! `driver` returns `void` and communicates only by writing to stdout, so the
//! harness redirects fd 1 to a temporary file around each call and compares the
//! captured bytes.

#![allow(dead_code)]

use std::ffi::{c_char, CString};
use std::io::{Read, Seek, SeekFrom};
use std::path::PathBuf;
use std::sync::OnceLock;

use libloading::{Library, Symbol};

pub type DriverFn = unsafe extern "C" fn(*const c_char, *const c_char);

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ has a parent")
        .to_path_buf()
}

pub fn c_so_path() -> PathBuf {
    workspace_root().join("c_src/build/libdriver.so")
}

pub fn rust_so_path() -> PathBuf {
    // Tests are run for the `test` profile, but we deliberately load the
    // release cdylib: that is the artifact an external consumer links against.
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for profile in ["release", "debug"] {
        let p = root.join("target").join(profile).join("libdriver.so");
        if p.exists() {
            return p;
        }
    }
    panic!("no libdriver.so found under translation/target/{{release,debug}} — run `cargo build --release` first");
}

pub struct Pair {
    _c_lib: Library,
    _rust_lib: Library,
    pub c: DriverFn,
    pub rust: DriverFn,
}

impl Pair {
    fn load() -> Pair {
        let c_path = c_so_path();
        let rust_path = rust_so_path();
        assert!(
            c_path.exists(),
            "C shared library missing at {c_path:?}; build it with cmake first"
        );

        // SAFETY: both paths point at shared objects we built ourselves; loading
        // them runs their (empty) initialisers.
        unsafe {
            let c_lib = Library::new(&c_path).expect("dlopen C libdriver.so");
            let rust_lib = Library::new(&rust_path).expect("dlopen Rust libdriver.so");

            let c_sym: Symbol<DriverFn> = c_lib.get(b"driver\0").expect("C `driver` symbol");
            let rust_sym: Symbol<DriverFn> =
                rust_lib.get(b"driver\0").expect("Rust `driver` symbol");

            let c = *c_sym;
            let rust = *rust_sym;

            Pair {
                _c_lib: c_lib,
                _rust_lib: rust_lib,
                c,
                rust,
            }
        }
    }
}

static PAIR: OnceLock<Pair> = OnceLock::new();

/// The loaded C/Rust function pair. Loaded once per test binary.
pub fn pair() -> &'static Pair {
    PAIR.get_or_init(Pair::load)
}

/// Serialises fd-1 redirection: `capture_stdout` and `run_in_child` both swap the
/// process-global stdout descriptor, so two tests doing it concurrently would
/// steal each other's output. Tests may therefore still run in parallel.
static STDOUT_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Captures everything written to fd 1 (stdout) while `f` runs.
///
/// Both implementations write with glibc `printf`, so the capture must flush
/// stdio, swap the underlying fd for a temp file, run the call, flush again, and
/// restore. This is the only way to observe a `void`-returning printing API
/// byte-for-byte.
fn capture_stdout<F: FnOnce()>(f: F) -> Vec<u8> {
    use std::fs::File;
    use std::os::unix::io::{AsRawFd, FromRawFd};

    let _guard = STDOUT_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    // The libtest harness writes its progress line ("test foo ... ") through
    // Rust's `std::io::Stdout`, which is line-buffered — a partial line without a
    // trailing newline would otherwise still be sitting in that buffer and get
    // flushed into OUR capture file. Flush both buffering layers before swapping
    // the descriptor. (Tests must also run with `--test-threads=1` so the harness
    // cannot write concurrently from another thread — `verify.sh` always passes
    // it.)
    {
        use std::io::Write;
        let _ = std::io::stdout().flush();
        let _ = std::io::stderr().flush();
    }

    // SAFETY: plain libc fd bookkeeping; every fd we create is closed below.
    unsafe {
        libc::fflush(std::ptr::null_mut());

        let mut tmp = tempfile();
        let saved = libc::dup(1);
        assert!(saved >= 0, "dup(1) failed");
        assert!(libc::dup2(tmp.as_raw_fd(), 1) >= 0, "dup2 onto stdout failed");

        f();

        libc::fflush(std::ptr::null_mut());
        assert!(libc::dup2(saved, 1) >= 0, "restoring stdout failed");
        libc::close(saved);

        tmp.seek(SeekFrom::Start(0)).expect("rewind capture file");
        let mut buf = Vec::new();
        tmp.read_to_end(&mut buf).expect("read capture file");
        // `tmp` is dropped (and closed) here.
        let _ = File::from_raw_fd; // keep the import meaningful under all cfgs
        buf
    }
}

fn tempfile() -> std::fs::File {
    use std::os::unix::io::FromRawFd;
    let template = std::ffi::CString::new(
        std::env::temp_dir()
            .join("driver-diff-XXXXXX")
            .to_string_lossy()
            .as_bytes()
            .to_vec(),
    )
    .expect("temp path has no interior NUL");
    let mut raw = template.into_bytes_with_nul();
    // SAFETY: `raw` is a NUL-terminated mutable buffer ending in exactly six X's.
    let fd = unsafe { libc::mkstemp(raw.as_mut_ptr() as *mut c_char) };
    assert!(fd >= 0, "mkstemp failed");
    // Unlink immediately: we only need the fd.
    // SAFETY: `raw` is still a valid NUL-terminated path.
    unsafe {
        libc::unlink(raw.as_ptr() as *const c_char);
        FromRawFd::from_raw_fd(fd)
    }
}

/// Runs BOTH implementations on the same raw pointers and returns
/// `(c_stdout, rust_stdout)`.
///
/// # Safety
/// The pointers are passed through verbatim, so the caller carries whatever
/// contract `driver` imposes. Tests that deliberately pass invalid pointers must
/// use [`run_both_in_child`] instead.
pub unsafe fn run_both_raw(s1: *const c_char, s2: *const c_char) -> (Vec<u8>, Vec<u8>) {
    let p = pair();
    let c_out = capture_stdout(|| unsafe { (p.c)(s1, s2) });
    let rust_out = capture_stdout(|| unsafe { (p.rust)(s1, s2) });
    (c_out, rust_out)
}

/// Asserts the two implementations produce byte-identical stdout for the given
/// NUL-terminated byte strings, and returns the shared output.
pub fn assert_same(s1: &[u8], s2: &[u8], label: &str) -> Vec<u8> {
    let c1 = CString::new(s1).expect("s1 must not contain an interior NUL");
    let c2 = CString::new(s2).expect("s2 must not contain an interior NUL");
    assert_same_cstr(c1.as_bytes_with_nul(), c2.as_bytes_with_nul(), label)
}

/// Like [`assert_same`] but takes buffers that ALREADY include their terminator,
/// so callers can embed interior NULs on purpose.
pub fn assert_same_cstr(s1_with_nul: &[u8], s2_with_nul: &[u8], label: &str) -> Vec<u8> {
    assert_eq!(s1_with_nul.last(), Some(&0), "{label}: s1 must end in NUL");
    assert_eq!(s2_with_nul.last(), Some(&0), "{label}: s2 must end in NUL");

    // SAFETY: both buffers are NUL-terminated and outlive the calls.
    let (c_out, rust_out) = unsafe {
        run_both_raw(
            s1_with_nul.as_ptr() as *const c_char,
            s2_with_nul.as_ptr() as *const c_char,
        )
    };

    assert_eq!(
        c_out,
        rust_out,
        "{label}: stdout diverged\n  s1={:?}\n  s2={:?}\n  C   ={:?}\n  Rust={:?}",
        Trunc(s1_with_nul),
        Trunc(s2_with_nul),
        String::from_utf8_lossy(&c_out),
        String::from_utf8_lossy(&rust_out),
    );
    c_out
}

/// Convenience: assert both agree AND that the printed value is `expected`.
pub fn assert_same_and_value(s1: &[u8], s2: &[u8], expected: usize, label: &str) {
    let out = assert_same(s1, s2, label);
    let want = format!("{expected}\n");
    assert_eq!(
        String::from_utf8_lossy(&out),
        want,
        "{label}: both agreed but printed the wrong value"
    );
}

struct Trunc<'a>(&'a [u8]);
impl std::fmt::Debug for Trunc<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let n = self.0.len();
        if n <= 64 {
            write!(f, "{:?}", self.0)
        } else {
            write!(f, "{:?}... ({n} bytes)", &self.0[..64])
        }
    }
}

// ---------------------------------------------------------------------------
// Out-of-process execution, for the undefined-behaviour rows of ERRORS.md.
// ---------------------------------------------------------------------------

/// How a child process that ran one implementation ended.
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Outcome {
    /// Returned normally with this exit status and this stdout.
    Exited { code: i32, stdout: Vec<u8> },
    /// Died on a signal (e.g. `SIGSEGV` == 11).
    Signalled { signal: i32 },
}

/// Which implementation to run in the child.
#[derive(Copy, Clone, Debug)]
pub enum Which {
    C,
    Rust,
}

/// Runs `driver` in a forked child so that a crash (the expected outcome for the
/// UB rows) is observable instead of taking the test harness down with it.
///
/// # Safety
/// `build` runs in the child and produces the two pointers to pass to `driver`;
/// they may be deliberately invalid.
pub unsafe fn run_in_child<F>(which: Which, build: F) -> Outcome
where
    F: FnOnce() -> (*const c_char, *const c_char),
{
    let p = pair();
    let f = match which {
        Which::C => p.c,
        Which::Rust => p.rust,
    };

    let mut tmp_path_buf = {
        let template = std::ffi::CString::new(
            std::env::temp_dir()
                .join("driver-child-XXXXXX")
                .to_string_lossy()
                .as_bytes()
                .to_vec(),
        )
        .unwrap();
        template.into_bytes_with_nul()
    };
    // SAFETY: NUL-terminated mutable template ending in six X's.
    let out_fd = unsafe { libc::mkstemp(tmp_path_buf.as_mut_ptr() as *mut c_char) };
    assert!(out_fd >= 0, "mkstemp failed");

    // SAFETY: fork/exec-free child that only calls async-signal-safe-ish code we
    // control; it always terminates via `_exit` or a fatal signal.
    unsafe {
        let _guard = STDOUT_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        {
            use std::io::Write;
            let _ = std::io::stdout().flush();
            let _ = std::io::stderr().flush();
        }
        libc::fflush(std::ptr::null_mut());
        let pid = libc::fork();
        assert!(pid >= 0, "fork failed");

        if pid == 0 {
            // Child: point stdout at the temp file, run the call, exit 0.
            libc::dup2(out_fd, 1);
            let (a, b) = build();
            f(a, b);
            libc::fflush(std::ptr::null_mut());
            libc::_exit(0);
        }

        let mut status: i32 = 0;
        let waited = libc::waitpid(pid, &mut status, 0);
        assert_eq!(waited, pid, "waitpid failed");

        let outcome = if libc::WIFSIGNALED(status) {
            Outcome::Signalled {
                signal: libc::WTERMSIG(status),
            }
        } else {
            let mut file: std::fs::File = {
                use std::os::unix::io::FromRawFd;
                FromRawFd::from_raw_fd(libc::dup(out_fd))
            };
            file.seek(SeekFrom::Start(0)).unwrap();
            let mut buf = Vec::new();
            file.read_to_end(&mut buf).unwrap();
            Outcome::Exited {
                code: libc::WEXITSTATUS(status),
                stdout: buf,
            }
        };

        libc::close(out_fd);
        libc::unlink(tmp_path_buf.as_ptr() as *const c_char);
        outcome
    }
}

/// Asserts the C and Rust implementations reach the SAME outcome (same signal,
/// or same exit code AND same stdout) for a deliberately invalid input.
///
/// `build` is called once per child, so it must be cloneable in spirit: it is
/// taken as a `Fn`.
pub fn assert_same_outcome<F>(build: F, label: &str) -> Outcome
where
    F: Fn() -> (*const c_char, *const c_char),
{
    // SAFETY: each call happens in its own forked child.
    let c = unsafe { run_in_child(Which::C, &build) };
    let rust = unsafe { run_in_child(Which::Rust, &build) };
    assert_eq!(
        c, rust,
        "{label}: C and Rust outcomes diverged\n  C   ={c:?}\n  Rust={rust:?}"
    );
    c
}

// ---------------------------------------------------------------------------
// Deterministic PRNG — fixed seed, so every failure is reproducible.
// ---------------------------------------------------------------------------

pub const SEED: u64 = 0x5EED_1234_ABCD_0001;

/// xorshift64*, chosen for being short and exactly reproducible.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
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

    /// Uniform-ish in `[0, n)`.
    pub fn below(&mut self, n: usize) -> usize {
        assert!(n > 0);
        (self.next_u64() % n as u64) as usize
    }

    pub fn range(&mut self, lo: usize, hi_inclusive: usize) -> usize {
        assert!(lo <= hi_inclusive);
        lo + self.below(hi_inclusive - lo + 1)
    }

    /// A non-NUL byte from the full `0x01..=0xFF` range.
    pub fn byte_any(&mut self) -> u8 {
        (self.below(255) + 1) as u8
    }

    /// A printable ASCII byte.
    pub fn byte_ascii(&mut self) -> u8 {
        (self.range(0x20, 0x7E)) as u8
    }

    pub fn bytes_any(&mut self, len: usize) -> Vec<u8> {
        (0..len).map(|_| self.byte_any()).collect()
    }

    pub fn bytes_ascii(&mut self, len: usize) -> Vec<u8> {
        (0..len).map(|_| self.byte_ascii()).collect()
    }

    /// `bytes_any` with the length itself drawn from `[lo, hi]`. Exists so tests
    /// can avoid `rng.bytes_any(rng.range(..))`, which the borrow checker
    /// rejects.
    pub fn bytes_any_len(&mut self, lo: usize, hi_inclusive: usize) -> Vec<u8> {
        let n = self.range(lo, hi_inclusive);
        self.bytes_any(n)
    }

    pub fn bytes_ascii_len(&mut self, lo: usize, hi_inclusive: usize) -> Vec<u8> {
        let n = self.range(lo, hi_inclusive);
        self.bytes_ascii(n)
    }

    /// Bytes drawn from an inclusive value range, with a random length.
    pub fn bytes_in_len(
        &mut self,
        val_lo: u8,
        val_hi: u8,
        lo: usize,
        hi_inclusive: usize,
    ) -> Vec<u8> {
        let n = self.range(lo, hi_inclusive);
        (0..n)
            .map(|_| self.range(val_lo as usize, val_hi as usize) as u8)
            .collect()
    }
}

/// The reference result: length of the initial span of `s1` made of bytes not in
/// `s2`, matching C `strcspn` on NUL-terminated strings.
pub fn strcspn_ref(s1: &[u8], s2: &[u8]) -> usize {
    for (i, b) in s1.iter().enumerate() {
        if s2.contains(b) {
            return i;
        }
    }
    s1.len()
}
