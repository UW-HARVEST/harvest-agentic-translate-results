//! Differential-test harness.
//!
//! Loads BOTH shared libraries through `libloading` and never calls a Rust
//! function directly, so every assertion goes through the `#[no_mangle]`
//! `extern "C"` export wrappers exactly as an external C consumer would.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::path::PathBuf;
use std::sync::OnceLock;

pub struct Pair {
    pub c: Library,
    pub r: Library,
}

fn manifest() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_so() -> PathBuf {
    if let Ok(p) = std::env::var("C_SO") {
        return PathBuf::from(p);
    }
    manifest().join("../c_src/build/libsodium.so")
}

fn rust_so() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_SO") {
        return PathBuf::from(p);
    }
    let rel = manifest().join("target/release/liblibsodium.so");
    if rel.exists() {
        return rel;
    }
    manifest().join("target/debug/liblibsodium.so")
}

static PAIR: OnceLock<Pair> = OnceLock::new();

pub fn libs() -> &'static Pair {
    PAIR.get_or_init(|| {
        let c = unsafe { Library::new(c_so()).expect("load C .so") };
        let r = unsafe { Library::new(rust_so()).expect("load Rust .so") };
        // sodium_init() on both, as a real consumer does.
        unsafe {
            let ci: Symbol<unsafe extern "C" fn() -> i32> = c.get(b"sodium_init\0").unwrap();
            let ri: Symbol<unsafe extern "C" fn() -> i32> = r.get(b"sodium_init\0").unwrap();
            assert_eq!(ci(), 0, "C sodium_init");
            assert_eq!(ri(), 0, "Rust sodium_init");
        }
        Pair { c, r }
    })
}

/// Fetch the same symbol from both libraries, typed identically.
///
/// ```ignore
/// let (c, r) = sym::<unsafe extern "C" fn(*mut u8) -> i32>("crypto_box_keypair");
/// ```
pub fn sym<T: Copy>(name: &str) -> (T, T) {
    let l = libs();
    let mut n = name.as_bytes().to_vec();
    n.push(0);
    unsafe {
        let cs: Symbol<T> = l
            .c
            .get(&n)
            .unwrap_or_else(|e| panic!("C .so missing `{name}`: {e}"));
        let rs: Symbol<T> = l
            .r
            .get(&n)
            .unwrap_or_else(|e| panic!("Rust .so missing `{name}`: {e}"));
        (*cs, *rs)
    }
}

/// True if BOTH libraries export `name`.
pub fn has(name: &str) -> bool {
    let l = libs();
    let mut n = name.as_bytes().to_vec();
    n.push(0);
    unsafe {
        l.c.get::<*const ()>(&n).is_ok() && l.r.get::<*const ()>(&n).is_ok()
    }
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (splitmix64) — fixed seed => reproducible test inputs.
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed.wrapping_add(0x9E3779B97F4A7C15))
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
    pub fn byte(&mut self) -> u8 {
        (self.next_u64() >> 56) as u8
    }
    /// Uniform in `[0, n)`; `n == 0` yields 0.
    pub fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next_u64() % n as u64) as usize
        }
    }
    pub fn fill(&mut self, buf: &mut [u8]) {
        for b in buf.iter_mut() {
            *b = self.byte();
        }
    }
    pub fn bytes(&mut self, n: usize) -> Vec<u8> {
        let mut v = vec![0u8; n];
        self.fill(&mut v);
        v
    }
}

/// A canary-padded output buffer: detects over-writes past `len` and lets us
/// compare the *whole* allocation (payload + canary) byte-for-byte.
pub const CANARY: usize = 16;

pub fn out_buf(len: usize) -> Vec<u8> {
    let mut v = vec![0u8; len + CANARY];
    for (i, b) in v[len..].iter_mut().enumerate() {
        *b = 0xA5u8.wrapping_add(i as u8);
    }
    v
}

pub fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

/// Assert two byte buffers are identical, with a hex diff on failure.
#[track_caller]
pub fn eqb(what: &str, c: &[u8], r: &[u8]) {
    if c != r {
        let n = c.len().min(r.len());
        let mut first = n;
        for i in 0..n {
            if c[i] != r[i] {
                first = i;
                break;
            }
        }
        panic!(
            "{what}: buffers differ (len C={} R={}, first diff at {first})\n C={}\n R={}",
            c.len(),
            r.len(),
            hex(c),
            hex(r)
        );
    }
}

// ---------------------------------------------------------------------------
// Fork helper: some libsodium error paths call sodium_misuse() -> abort().
// Run the closure in a child process and report how it terminated, so the
// C and Rust abort/return behaviour can be compared without killing the test.
// ---------------------------------------------------------------------------

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Outcome {
    /// Returned normally; payload is the low 8 bits of the reported value.
    Returned(i32),
    /// Terminated by a signal (SIGABRT == 6).
    Signal(i32),
}

static NO_CORE: OnceLock<()> = OnceLock::new();

/// Hard cap on how long a forked child may run before `SIGALRM` kills it.
const CHILD_TIMEOUT_SECS: u32 = 30;

fn no_core_dumps() {
    NO_CORE.get_or_init(|| unsafe {
        let rl = libc::rlimit { rlim_cur: 0, rlim_max: 0 };
        libc::setrlimit(libc::RLIMIT_CORE, &rl);
    });
}

/// Runs `f` in a forked child. `f` returns an i32 which is passed out through
/// the exit status (only the low 8 bits survive, which is enough to
/// distinguish 0 / 1 / 255(-1)).
///
/// # Why the test runners pass `--test-threads=1`
///
/// `fork()` in a multithreaded process gives the child only the calling thread.
/// If any *other* test thread happens to hold glibc's malloc arena lock or the
/// dynamic-loader lock at that moment, the child deadlocks the first time it
/// allocates or calls `dlsym` — and the parent then blocks forever in
/// `waitpid`. The closures here do both (they resolve symbols and build
/// buffers), so `tools/run_tests.sh` and `tools/feature_matrix.sh` run the
/// suite with `--test-threads=1`, which removes the other threads entirely.
///
/// As a belt-and-braces measure the child also arms `alarm()`: a child that
/// wedges anyway is killed by `SIGALRM`, which surfaces as a loud outcome
/// mismatch instead of an indefinite hang.
pub fn fork_run<F: FnOnce() -> i32>(f: F) -> Outcome {
    no_core_dumps();
    unsafe {
        let pid = libc::fork();
        assert!(pid >= 0, "fork failed");
        if pid == 0 {
            // A libsodium abort() in a process with two ~1 MB shared objects
            // mapped writes a large core file; that dominates runtime and is
            // useless here. Suppress it, and silence the abort message.
            let rl = libc::rlimit { rlim_cur: 0, rlim_max: 0 };
            libc::setrlimit(libc::RLIMIT_CORE, &rl);
            let devnull = libc::open(b"/dev/null\0".as_ptr() as *const _, libc::O_WRONLY);
            if devnull >= 0 {
                libc::dup2(devnull, 2);
                libc::close(devnull);
            }
            libc::alarm(CHILD_TIMEOUT_SECS);
            let rc = f();
            libc::_exit((rc & 0xff) as i32);
        }
        let mut status: i32 = 0;
        let w = libc::waitpid(pid, &mut status, 0);
        assert_eq!(w, pid, "waitpid");
        if libc::WIFSIGNALED(status) {
            Outcome::Signal(libc::WTERMSIG(status))
        } else {
            Outcome::Returned(libc::WEXITSTATUS(status))
        }
    }
}

/// Differential check for a call that may abort: both sides must terminate the
/// same way (same return value, or both killed by the same signal).
#[track_caller]
pub fn same_outcome<FC, FR>(what: &str, cf: FC, rf: FR)
where
    FC: FnOnce() -> i32,
    FR: FnOnce() -> i32,
{
    let co = fork_run(cf);
    let ro = fork_run(rf);
    assert_eq!(co, ro, "{what}: C outcome {co:?} != Rust outcome {ro:?}");
}
