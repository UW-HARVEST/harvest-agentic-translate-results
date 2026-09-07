// Shared differential-test harness.
//
// Both the C `.so` and the Rust `.so` are loaded with `libloading` and driven
// ONLY through their exported C symbols, so the `#[no_mangle]`/`extern "C"`
// wrappers are part of what is under test.
#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::io::{Read, Seek, SeekFrom};
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, OnceLock};

pub type FnPtrArg = unsafe extern "C" fn(*const libc::c_int);
pub type FnVoid = unsafe extern "C" fn();
pub type FnIntArg = unsafe extern "C" fn(libc::c_int);

pub struct Libs {
    pub c: Library,
    pub rs: Library,
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("C_DRIVER_SO") {
        return PathBuf::from(p);
    }
    manifest_dir()
        .parent()
        .unwrap()
        .join("c_src/build/libdriver.so")
}

fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_DRIVER_SO") {
        return PathBuf::from(p);
    }
    for profile in ["release", "debug"] {
        let p = manifest_dir().join(format!("target/{profile}/libdriver.so"));
        if p.exists() {
            return p;
        }
    }
    panic!("Rust cdylib not found; run `cargo build --release` first");
}

pub fn libs() -> &'static Libs {
    static LIBS: OnceLock<Libs> = OnceLock::new();
    LIBS.get_or_init(|| {
        let c = c_so_path();
        let rs = rust_so_path();
        assert!(c.exists(), "missing C .so at {}", c.display());
        assert!(rs.exists(), "missing Rust .so at {}", rs.display());
        unsafe {
            Libs {
                c: Library::new(&c).expect("dlopen C .so"),
                rs: Library::new(&rs).expect("dlopen Rust .so"),
            }
        }
    })
}

/// stdout redirection is process-global, so capturing tests serialize on this.
pub fn stdout_lock() -> MutexGuard<'static, ()> {
    static M: OnceLock<Mutex<()>> = OnceLock::new();
    M.get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}

pub fn sym<T: Copy>(lib: &'static Library, name: &[u8]) -> T {
    unsafe {
        let s: Symbol<T> = lib.get(name).unwrap_or_else(|e| {
            panic!(
                "symbol {:?} not exported: {e}",
                String::from_utf8_lossy(name)
            )
        });
        *s
    }
}

pub fn c_fn<T: Copy>(name: &[u8]) -> T {
    sym(&libs().c, name)
}
pub fn rs_fn<T: Copy>(name: &[u8]) -> T {
    sym(&libs().rs, name)
}

fn tmp_path(tag: &str) -> PathBuf {
    let dir = std::env::var("TMPDIR").unwrap_or_else(|_| "/tmp".into());
    let mut p = PathBuf::from(dir);
    p.push(format!(
        "difftest-{}-{}-{:?}.out",
        tag,
        std::process::id(),
        std::thread::current().id()
    ));
    p
}

/// Open a fresh scratch file and return (File, raw fd).
fn scratch(tag: &str) -> (std::fs::File, libc::c_int) {
    let path = tmp_path(tag);
    let f = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(&path)
        .expect("open scratch file");
    let _ = std::fs::remove_file(&path); // unlink: file lives only via the fd
    let fd = {
        use std::os::unix::io::AsRawFd;
        f.as_raw_fd()
    };
    (f, fd)
}

/// Run `f` with libc `stdout` (fd 1) redirected to a scratch file, and return
/// every byte the callee wrote. `fflush(NULL)` is used so that whatever libc
/// buffering the `.so` performs is observed identically for C and Rust.
pub fn capture<F: FnOnce()>(f: F) -> Vec<u8> {
    unsafe {
        libc::fflush(std::ptr::null_mut());
        let (mut file, fd) = scratch("cap");
        let saved = libc::dup(1);
        assert!(saved >= 0, "dup(1) failed");
        assert!(libc::dup2(fd, 1) >= 0, "dup2 failed");

        f();

        libc::fflush(std::ptr::null_mut());
        assert!(libc::dup2(saved, 1) >= 0, "restore dup2 failed");
        libc::close(saved);

        file.seek(SeekFrom::Start(0)).unwrap();
        let mut buf = Vec::new();
        file.read_to_end(&mut buf).unwrap();
        buf
    }
}

/// Outcome of running a (possibly crashing) closure in a forked child.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct ChildOutcome {
    /// `Some(sig)` if killed by a signal.
    pub signal: Option<i32>,
    /// `Some(code)` if it exited normally.
    pub exit: Option<i32>,
    pub stdout: Vec<u8>,
}

impl ChildOutcome {
    pub fn crashed(&self) -> bool {
        self.signal.is_some()
    }
}

/// Fork, redirect the child's stdout to a scratch file, run `f` in the child,
/// then `_exit(0)`. Returns how the child died plus the bytes it produced.
///
/// The caller MUST have already resolved (dlopen'd) every symbol it intends to
/// use, so the child performs no allocation-heavy work post-fork.
pub fn run_in_child<F: FnOnce()>(f: F) -> ChildOutcome {
    unsafe {
        libc::fflush(std::ptr::null_mut());
        let (mut file, fd) = scratch("child");

        let pid = libc::fork();
        assert!(pid >= 0, "fork failed");
        if pid == 0 {
            // ---- child ----
            libc::dup2(fd, 1);
            f();
            libc::fflush(std::ptr::null_mut());
            libc::_exit(0);
        }

        let mut status: libc::c_int = 0;
        let w = libc::waitpid(pid, &mut status, 0);
        assert_eq!(w, pid, "waitpid failed");

        file.seek(SeekFrom::Start(0)).unwrap();
        let mut buf = Vec::new();
        file.read_to_end(&mut buf).unwrap();

        let signal = if libc::WIFSIGNALED(status) {
            Some(libc::WTERMSIG(status))
        } else {
            None
        };
        let exit = if libc::WIFEXITED(status) {
            Some(libc::WEXITSTATUS(status))
        } else {
            None
        };
        ChildOutcome {
            signal,
            exit,
            stdout: buf,
        }
    }
}

/// Deterministic SplitMix64 PRNG — fixed seed => reproducible test vectors.
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
        self.next_u64() as u32 as i32
    }
    pub fn next_u8(&mut self) -> u8 {
        self.next_u64() as u8
    }
    /// Uniform in `0..n`.
    pub fn below(&mut self, n: u64) -> u64 {
        self.next_u64() % n
    }
}

/// The interesting `i32` boundary values the `%d` conversion distinguishes.
pub const INT_BOUNDARIES: &[i32] = &[
    i32::MIN,
    i32::MIN + 1,
    -2_000_000_000,
    -1_000_000_000,
    -100_000,
    -1000,
    -100,
    -10,
    -9,
    -1,
    0,
    1,
    9,
    10,
    99,
    100,
    999,
    1000,
    65_535,
    65_536,
    1_000_000_000,
    2_000_000_000,
    i32::MAX - 1,
    i32::MAX,
];

/// Assert that `out` is exactly one `printf("%d\n", ...)` line: a decimal
/// `int`, optionally signed, followed by a single trailing newline. Used for the
/// CWE-457 path where the printed VALUE is indeterminate but the SHAPE is not.
pub fn assert_single_decimal_line(out: &[u8], ctx: &str) {
    let s = std::str::from_utf8(out).unwrap_or_else(|e| panic!("{ctx}: non-UTF8 output: {e}"));
    assert!(
        s.ends_with('\n'),
        "{ctx}: missing trailing newline: {:?}",
        show(out)
    );
    let body = &s[..s.len() - 1];
    assert!(
        !body.contains('\n'),
        "{ctx}: expected exactly one line, got {:?}",
        show(out)
    );
    body.parse::<i32>()
        .unwrap_or_else(|e| panic!("{ctx}: not a decimal int ({e}): {:?}", show(out)));
}

pub fn show(b: &[u8]) -> String {
    String::from_utf8_lossy(b).escape_debug().to_string()
}
