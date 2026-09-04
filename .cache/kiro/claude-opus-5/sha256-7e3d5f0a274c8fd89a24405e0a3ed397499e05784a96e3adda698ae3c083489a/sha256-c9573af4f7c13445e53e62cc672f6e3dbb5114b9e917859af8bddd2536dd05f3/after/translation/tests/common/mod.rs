//! Differential test harness.
//!
//! Loads BOTH shared libraries via `libloading` and exposes helpers to call the
//! same exported symbol in each and compare results byte-for-byte. The Rust
//! implementation is *never* called directly — always through its `.so`
//! exports, exactly as an external C consumer would, so the `#[no_mangle]`
//! wrappers are under test too.

#![allow(dead_code)]

pub mod accessors;

use libloading::{Library, Symbol};
use std::path::PathBuf;
use std::sync::OnceLock;

pub struct Duo {
    pub c: Library,
    pub r: Library,
}

fn manifest() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_so() -> PathBuf {
    manifest().join("../c_src/build/libsodium.so")
}

fn rust_so() -> PathBuf {
    // Prefer the profile the test binary itself was built into.
    let exe = std::env::current_exe().expect("current_exe");
    // .../target/<profile>/deps/<test>-<hash>
    let profile_dir = exe.parent().and_then(|p| p.parent());
    if let Some(d) = profile_dir {
        let p = d.join("liblibsodium.so");
        if p.exists() {
            return p;
        }
    }
    for prof in ["release", "debug"] {
        let p = manifest().join("target").join(prof).join("liblibsodium.so");
        if p.exists() {
            return p;
        }
    }
    panic!("liblibsodium.so not found; run `cargo build --release` first");
}

static DUO: OnceLock<Duo> = OnceLock::new();

/// Both libraries, loaded once per test binary with RTLD_LOCAL so their
/// identically-named symbols cannot interpose on one another.
pub fn duo() -> &'static Duo {
    DUO.get_or_init(|| {
        let c = unsafe { Library::new(c_so()) }
            .unwrap_or_else(|e| panic!("load C .so {:?}: {e}", c_so()));
        let r = unsafe { Library::new(rust_so()) }
            .unwrap_or_else(|e| panic!("load Rust .so {:?}: {e}", rust_so()));
        // Both libraries must be initialized before use.
        unsafe {
            let f: Symbol<unsafe extern "C" fn() -> i32> = c.get(b"sodium_init\0").unwrap();
            f();
            let f: Symbol<unsafe extern "C" fn() -> i32> = r.get(b"sodium_init\0").unwrap();
            f();
        }
        Duo { c, r }
    })
}

impl Duo {
    /// Fetch the same symbol from both libraries. Panics if either lacks it —
    /// that is itself a symbol-parity failure worth failing the test on.
    pub fn pair<T>(&'static self, name: &str) -> (Symbol<'static, T>, Symbol<'static, T>) {
        let mut z = name.as_bytes().to_vec();
        z.push(0);
        let cf: Symbol<T> = unsafe { self.c.get(&z) }
            .unwrap_or_else(|e| panic!("C .so missing symbol `{name}`: {e}"));
        let rf: Symbol<T> = unsafe { self.r.get(&z) }
            .unwrap_or_else(|e| panic!("Rust .so missing symbol `{name}`: {e}"));
        (cf, rf)
    }

    pub fn has(&self, name: &str) -> bool {
        let mut z = name.as_bytes().to_vec();
        z.push(0);
        unsafe { self.c.get::<*const ()>(&z).is_ok() && self.r.get::<*const ()>(&z).is_ok() }
    }
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (xoshiro256**) — fixed seed per test for reproducibility.
// ---------------------------------------------------------------------------

pub struct Rng {
    s: [u64; 4],
}

impl Rng {
    pub fn new(seed: u64) -> Self {
        // splitmix64 expansion
        let mut z = seed;
        let mut s = [0u64; 4];
        for v in s.iter_mut() {
            z = z.wrapping_add(0x9E37_79B9_7F4A_7C15);
            let mut x = z;
            x = (x ^ (x >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
            x = (x ^ (x >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
            *v = x ^ (x >> 31);
        }
        Rng { s }
    }

    pub fn next_u64(&mut self) -> u64 {
        let r = self.s[1].wrapping_mul(5).rotate_left(7).wrapping_mul(9);
        let t = self.s[1] << 17;
        self.s[2] ^= self.s[0];
        self.s[3] ^= self.s[1];
        self.s[1] ^= self.s[2];
        self.s[0] ^= self.s[3];
        self.s[2] ^= t;
        self.s[3] = self.s[3].rotate_left(45);
        r
    }

    pub fn u8(&mut self) -> u8 {
        (self.next_u64() >> 56) as u8
    }

    pub fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next_u64() % n as u64) as usize
        }
    }

    pub fn bytes(&mut self, n: usize) -> Vec<u8> {
        (0..n).map(|_| self.u8()).collect()
    }

    pub fn fill(&mut self, b: &mut [u8]) {
        for x in b.iter_mut() {
            *x = self.u8();
        }
    }
}

/// Standard length sweep: exercises empty, one, sub-block, exact-block and
/// cross-block shapes for the common 16/32/64/128-byte block sizes.
pub const LENS: &[usize] = &[
    0, 1, 2, 15, 16, 17, 31, 32, 33, 55, 56, 57, 63, 64, 65, 111, 112, 113, 127, 128, 129, 135,
    136, 137, 167, 168, 169, 255, 256, 511, 1000,
];

/// Shorter sweep for expensive operations.
pub const LENS_SHORT: &[usize] = &[0, 1, 15, 16, 17, 31, 32, 33, 63, 64, 65, 128, 129, 1000];

#[track_caller]
pub fn eq_bytes(what: &str, c: &[u8], r: &[u8]) {
    if c != r {
        let at = c.iter().zip(r).position(|(a, b)| a != b);
        panic!(
            "{what}: C/Rust output differs at byte {:?}\n  C   ={}\n  Rust={}",
            at,
            hex(c),
            hex(r)
        );
    }
}

#[track_caller]
pub fn eq_i32(what: &str, c: i32, r: i32) {
    assert_eq!(c, r, "{what}: C returned {c}, Rust returned {r}");
}

pub fn hex(b: &[u8]) -> String {
    let mut s = String::with_capacity(b.len() * 2);
    for x in b.iter().take(96) {
        s.push_str(&format!("{x:02x}"));
    }
    if b.len() > 96 {
        s.push_str("...");
    }
    s
}

// ---------------------------------------------------------------------------
// errno access (for the `errno = E...; return -1` paths in the C source)
// ---------------------------------------------------------------------------

pub fn errno() -> i32 {
    unsafe { *libc::__errno_location() }
}

pub fn set_errno(v: i32) {
    unsafe { *libc::__errno_location() = v }
}

/// Run `f`, returning `(ret, errno)` with errno cleared beforehand.
pub fn with_errno<R>(f: impl FnOnce() -> R) -> (R, i32) {
    set_errno(0);
    let r = f();
    (r, errno())
}

// ---------------------------------------------------------------------------
// Abort/misuse testing: sodium_misuse() ends in abort(), so the only way to
// observe it is in a forked child.
// ---------------------------------------------------------------------------

/// Outcome of running a closure in a forked child.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Fate {
    /// Returned normally with this exit code.
    Exited(i32),
    /// Killed by this signal (SIGABRT == 6 for `sodium_misuse`).
    Signaled(i32),
}

/// Fork and run `f` in the child. The child never returns to the test harness.
pub fn in_child(f: impl FnOnce()) -> Fate {
    unsafe {
        let pid = libc::fork();
        assert!(pid >= 0, "fork failed");
        if pid == 0 {
            // Silence the abort message so test output stays readable.
            let devnull = libc::open(b"/dev/null\0".as_ptr() as *const _, libc::O_WRONLY);
            if devnull >= 0 {
                libc::dup2(devnull, 2);
            }
            f();
            libc::_exit(0);
        }
        let mut status: i32 = 0;
        libc::waitpid(pid, &mut status, 0);
        if libc::WIFSIGNALED(status) {
            Fate::Signaled(libc::WTERMSIG(status))
        } else {
            Fate::Exited(libc::WEXITSTATUS(status))
        }
    }
}

/// Assert C and Rust have the same fate (same exit code, or same fatal signal)
/// when driven into the same invalid condition.
#[track_caller]
pub fn same_fate(what: &str, cf: impl FnOnce(), rf: impl FnOnce()) {
    let c = in_child(cf);
    let r = in_child(rf);
    assert_eq!(c, r, "{what}: C fate {c:?} != Rust fate {r:?}");
}
