//! Shared differential-test harness.
//!
//! Loads BOTH the C `libsodium.so` and the Rust `liblibsodium.so` via
//! `libloading` and exposes helpers to call the same exported symbol in both
//! and compare results byte-for-byte. Rust functions are NEVER called
//! directly — every call goes through the `.so` export table, so the
//! `#[no_mangle] extern "C"` wrappers are under test too.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::path::PathBuf;
use std::sync::OnceLock;

// ---------------------------------------------------------------- library load

pub struct Libs {
    pub c: Library,
    pub rs: Library,
}

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR = <root>/translation
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("manifest dir has a parent")
        .to_path_buf()
}

fn c_so_path() -> PathBuf {
    workspace_root().join("c_src/build/libsodium.so")
}

fn rust_so_path() -> PathBuf {
    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target");
    for p in ["release/liblibsodium.so", "debug/liblibsodium.so"] {
        let cand = base.join(p);
        if cand.exists() {
            return cand;
        }
    }
    base.join("release/liblibsodium.so")
}

static LIBS: OnceLock<Libs> = OnceLock::new();

pub fn libs() -> &'static Libs {
    LIBS.get_or_init(|| {
        let cp = c_so_path();
        let rp = rust_so_path();
        assert!(cp.exists(), "C .so not found at {cp:?} — build it first");
        assert!(rp.exists(), "Rust .so not found at {rp:?} — cargo build --release");
        let c = unsafe { Library::new(&cp) }.unwrap_or_else(|e| panic!("dlopen {cp:?}: {e}"));
        let rs = unsafe { Library::new(&rp) }.unwrap_or_else(|e| panic!("dlopen {rp:?}: {e}"));
        // sodium_init() on both, as a real consumer would.
        unsafe {
            let ci: Symbol<unsafe extern "C" fn() -> i32> = c.get(b"sodium_init\0").unwrap();
            let ri: Symbol<unsafe extern "C" fn() -> i32> = rs.get(b"sodium_init\0").unwrap();
            assert!(ci() >= 0);
            assert!(ri() >= 0);
        }
        Libs { c, rs }
    })
}

impl Libs {
    /// Fetch the same symbol from both libraries. Panics with a clear message
    /// if either side does not export it.
    pub fn pair<T>(&self, name: &str) -> (Symbol<'_, T>, Symbol<'_, T>) {
        let mut z = Vec::with_capacity(name.len() + 1);
        z.extend_from_slice(name.as_bytes());
        z.push(0);
        let c = unsafe { self.c.get::<T>(&z) }
            .unwrap_or_else(|e| panic!("C .so missing symbol {name}: {e}"));
        let r = unsafe { self.rs.get::<T>(&z) }
            .unwrap_or_else(|e| panic!("Rust .so missing symbol {name}: {e}"));
        (c, r)
    }

    pub fn has(&self, name: &str) -> bool {
        let mut z = Vec::with_capacity(name.len() + 1);
        z.extend_from_slice(name.as_bytes());
        z.push(0);
        unsafe { self.c.get::<*const ()>(&z) }.is_ok()
            && unsafe { self.rs.get::<*const ()>(&z) }.is_ok()
    }
}

/// Convenience: fetch a symbol pair from the process-wide library handles.
pub fn pair<T>(name: &str) -> (Symbol<'static, T>, Symbol<'static, T>) {
    libs().pair(name)
}

// -------------------------------------------------------------------- rng

/// xorshift64* — deterministic, identical across runs.
pub struct Rng(u64);

pub const SEED: u64 = 0x5EED_1234_ABCD_EF01;

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(if seed == 0 { 1 } else { seed })
    }
    pub fn seeded() -> Self {
        Rng::new(SEED)
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
    pub fn next_u8(&mut self) -> u8 {
        (self.next_u64() >> 56) as u8
    }
    /// Uniform-ish in [0, n)
    pub fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next_u64() % n as u64) as usize
        }
    }
    pub fn bytes(&mut self, n: usize) -> Vec<u8> {
        (0..n).map(|_| self.next_u8()).collect()
    }
    pub fn fill(&mut self, buf: &mut [u8]) {
        for b in buf.iter_mut() {
            *b = self.next_u8();
        }
    }
}

// ------------------------------------------------------------- comparisons

pub fn hex(b: &[u8]) -> String {
    let mut s = String::with_capacity(b.len() * 2);
    for x in b {
        s.push_str(&format!("{x:02x}"));
    }
    s
}

/// Assert two byte slices are identical, reporting the first differing index.
#[track_caller]
pub fn same_bytes(ctx: &str, c: &[u8], r: &[u8]) {
    if c == r {
        return;
    }
    assert_eq!(c.len(), r.len(), "{ctx}: output length differs");
    let i = c.iter().zip(r.iter()).position(|(a, b)| a != b).unwrap();
    panic!(
        "{ctx}: bytes differ at index {i} (C=0x{:02x} Rust=0x{:02x})\n  C   = {}\n  Rust= {}",
        c[i],
        r[i],
        hex(c),
        hex(r)
    );
}

#[track_caller]
pub fn same_ret(ctx: &str, c: i32, r: i32) {
    assert_eq!(c, r, "{ctx}: return value differs (C={c} Rust={r})");
}

/// A canary byte used to pre-fill output buffers so that we also detect
/// "wrote fewer/more bytes than C" divergences.
pub const CANARY: u8 = 0xA5;

pub fn buf(n: usize) -> Vec<u8> {
    vec![CANARY; n]
}

// --------------------------------------------------- abort/misuse comparison

/// Outcome of running a closure in a forked child.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Outcome {
    Exited(i32),
    Signaled(i32),
}

/// Run `f` in a forked child process and report how the child terminated.
/// Used for `sodium_misuse()` rows, which `abort()` unconditionally.
pub fn run_forked<F: FnOnce()>(f: F) -> Outcome {
    unsafe {
        let pid = libc::fork();
        assert!(pid >= 0, "fork failed");
        if pid == 0 {
            // Child: suppress core dumps (they dominate the runtime of these
            // tests) and silence the abort message, then run.
            let lim = libc::rlimit {
                rlim_cur: 0,
                rlim_max: 0,
            };
            libc::setrlimit(libc::RLIMIT_CORE, &lim);
            let devnull = libc::open(b"/dev/null\0".as_ptr() as *const libc::c_char, libc::O_WRONLY);
            if devnull >= 0 {
                libc::dup2(devnull, 2);
            }
            f();
            libc::_exit(0);
        }
        let mut status: libc::c_int = 0;
        let w = libc::waitpid(pid, &mut status, 0);
        assert_eq!(w, pid, "waitpid failed");
        if libc::WIFSIGNALED(status) {
            Outcome::Signaled(libc::WTERMSIG(status))
        } else {
            Outcome::Exited(libc::WEXITSTATUS(status))
        }
    }
}

/// errno access, for the `errno = EINVAL / EFBIG / ENOSYS / ERANGE / ENOMEM`
/// rows in ERRORS.md.
pub fn errno() -> i32 {
    std::io::Error::last_os_error().raw_os_error().unwrap_or(0)
}

pub fn set_errno(v: i32) {
    unsafe {
        *libc::__errno_location() = v;
    }
}

/// Call `f` with errno cleared, returning `(ret, errno_after)`.
pub fn with_errno<R, F: FnOnce() -> R>(f: F) -> (R, i32) {
    set_errno(0);
    let r = f();
    (r, errno())
}
