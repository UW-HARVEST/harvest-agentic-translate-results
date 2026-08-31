//! Shared differential-test harness.
//!
//! Loads BOTH the reference C `libsodium.so` and the translated Rust
//! `liblibsodium.so` with `libloading` and exposes them as two opaque
//! libraries.  Every call in every test goes through `dlsym`, so the Rust
//! `#[no_mangle]` export wrappers are exercised exactly as an external C
//! consumer would exercise them.
#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, c_void};
use std::path::PathBuf;
use std::sync::OnceLock;

// ---------------------------------------------------------------- library load

fn crate_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("SODIUM_C_SO") {
        return PathBuf::from(p);
    }
    crate_dir()
        .parent()
        .unwrap()
        .join("c_src/build/libsodium.so")
}

fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("SODIUM_RUST_SO") {
        return PathBuf::from(p);
    }
    // The integration-test binary lives in target/<profile>/deps/, so the
    // cdylib built for the same profile is one directory up.
    if let Ok(exe) = std::env::current_exe() {
        if let Some(deps) = exe.parent() {
            if let Some(profile) = deps.parent() {
                let cand = profile.join("liblibsodium.so");
                if cand.exists() {
                    return cand;
                }
            }
        }
    }
    for p in ["target/release/liblibsodium.so", "target/debug/liblibsodium.so"] {
        let cand = crate_dir().join(p);
        if cand.exists() {
            return cand;
        }
    }
    panic!("cannot locate the Rust liblibsodium.so; run `cargo build` first");
}

/// `cargo test --test <name>` does NOT rebuild the `cdylib`, because the test
/// binaries reach it only through `dlsym` and so have no Cargo-level dependency
/// on it.  Without this check a stale `.so` can silently "pass" a suite that a
/// fresh one would fail.  Refuse to run instead.
fn assert_so_is_fresh(so: &std::path::Path) {
    let so_mtime = match std::fs::metadata(so).and_then(|m| m.modified()) {
        Ok(t) => t,
        Err(_) => return,
    };
    let src = crate_dir().join("src");
    let mut newest: Option<(std::path::PathBuf, std::time::SystemTime)> = None;
    let mut stack = vec![src];
    while let Some(d) = stack.pop() {
        let rd = match std::fs::read_dir(&d) {
            Ok(rd) => rd,
            Err(_) => continue,
        };
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
            } else if p.extension().map(|x| x == "rs").unwrap_or(false) {
                if let Ok(t) = e.metadata().and_then(|m| m.modified()) {
                    if newest.as_ref().map(|(_, n)| t > *n).unwrap_or(true) {
                        newest = Some((p, t));
                    }
                }
            }
        }
    }
    if let Some((p, t)) = newest {
        if t > so_mtime {
            panic!(
                "STALE cdylib: {} is newer than {}.\n\
                 `cargo test --test <name>` does not rebuild the cdylib; run\n\
                 `cargo build --offline` first (or set SODIUM_RUST_SO).",
                p.display(),
                so.display()
            );
        }
    }
}

struct Libs {
    c: Library,
    r: Library,
}

// The libraries are loaded once and never unloaded; every symbol handed out
// therefore has a `'static` lifetime.
static LIBS: OnceLock<Libs> = OnceLock::new();

fn libs() -> &'static Libs {
    LIBS.get_or_init(|| {
        let c = unsafe { Library::new(c_so_path()) }
            .unwrap_or_else(|e| panic!("loading {:?}: {e}", c_so_path()));
        let rp = rust_so_path();
        assert_so_is_fresh(&rp);
        let r = unsafe { Library::new(&rp) }.unwrap_or_else(|e| panic!("loading {rp:?}: {e}"));
        let l = Libs { c, r };
        unsafe { install_deterministic_rng(&l) };
        // sodium_init() after the RNG is installed, exactly like a real
        // consumer that swaps the RNG at startup.
        unsafe {
            let ci: Symbol<unsafe extern "C" fn() -> c_int> = l.c.get(b"sodium_init\0").unwrap();
            let ri: Symbol<unsafe extern "C" fn() -> c_int> = l.r.get(b"sodium_init\0").unwrap();
            assert_eq!(ci(), 0, "C sodium_init failed");
            assert_eq!(ri(), 0, "Rust sodium_init failed");
        }
        l
    })
}

pub fn c_lib() -> &'static Library {
    &libs().c
}
pub fn rust_lib() -> &'static Library {
    &libs().r
}

/// Look a symbol up in both libraries and return the (C, Rust) pair.
///
/// Panics with a clear message when the Rust `.so` does not export the symbol,
/// which is itself a translation-completeness failure.
pub fn both<T>(name: &str) -> (Symbol<'static, T>, Symbol<'static, T>) {
    let l = libs();
    let mut b: Vec<u8> = name.as_bytes().to_vec();
    b.push(0);
    unsafe {
        let c: Symbol<'static, T> = l
            .c
            .get(&b)
            .unwrap_or_else(|e| panic!("C .so is missing `{name}`: {e}"));
        let r: Symbol<'static, T> = l
            .r
            .get(&b)
            .unwrap_or_else(|e| panic!("Rust .so is missing `{name}`: {e}"));
        (c, r)
    }
}

/// `true` when both libraries export `name`.
pub fn has(name: &str) -> bool {
    let l = libs();
    let mut b: Vec<u8> = name.as_bytes().to_vec();
    b.push(0);
    unsafe {
        l.c.get::<*const c_void>(&b).is_ok() && l.r.get::<*const c_void>(&b).is_ok()
    }
}

// ------------------------------------------------------- deterministic RNG

#[repr(C)]
pub struct RandombytesImplementation {
    pub implementation_name: Option<unsafe extern "C" fn() -> *const c_char>,
    pub random: Option<unsafe extern "C" fn() -> u32>,
    pub stir: Option<unsafe extern "C" fn()>,
    pub uniform: Option<unsafe extern "C" fn(u32) -> u32>,
    pub buf: Option<unsafe extern "C" fn(*mut c_void, usize)>,
    pub close: Option<unsafe extern "C" fn() -> c_int>,
}

const RNG_SEED: u64 = 0x2545_F491_4F6C_DD1D;

// One independent stream per loaded library, so that the n-th random byte
// consumed by C equals the n-th random byte consumed by Rust.
//
// The state is THREAD-LOCAL: libtest runs the tests of one binary on several
// threads at once, and a process-global stream would let two tests interleave
// their draws, desynchronising the C stream from the Rust stream and producing
// spurious "divergences". Per-thread streams give every test its own lockstep
// pair regardless of `--test-threads`, and a fresh thread starts from RNG_SEED
// so a test that forgets `rng_reset()` is still reproducible.
thread_local! {
    static RNG_STATE: std::cell::Cell<[u64; 2]> = const { std::cell::Cell::new([RNG_SEED, RNG_SEED]) };
}

fn rng_next(i: usize) -> u64 {
    RNG_STATE.with(|c| {
        let mut s = c.get();
        let mut x = s[i];
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        s[i] = x;
        c.set(s);
        x
    })
}

/// Rewind both RNG streams (for the calling thread) so that the next C call and
/// the next Rust call see identical randomness.  Call this before any pair of
/// operations that consume randomness (`*_keypair`, `*_seal`, `randombytes_*`).
pub fn rng_reset() {
    RNG_STATE.with(|c| c.set([RNG_SEED, RNG_SEED]));
}

/// Rewind both RNG streams (for the calling thread) to an arbitrary non-zero seed.
pub fn rng_reseed(seed: u64) {
    let s = if seed == 0 { RNG_SEED } else { seed };
    RNG_STATE.with(|c| c.set([s, s]));
}

macro_rules! rng_impl {
    ($idx:expr, $name:ident, $rand:ident, $buf:ident, $stir:ident, $close:ident) => {
        unsafe extern "C" fn $name() -> *const c_char {
            b"difftest\0".as_ptr() as *const c_char
        }
        unsafe extern "C" fn $rand() -> u32 {
            rng_next($idx) as u32
        }
        unsafe extern "C" fn $buf(buf: *mut c_void, size: usize) {
            let mut p = buf as *mut u8;
            let mut n = size;
            while n >= 8 {
                let v = rng_next($idx).to_le_bytes();
                std::ptr::copy_nonoverlapping(v.as_ptr(), p, 8);
                p = p.add(8);
                n -= 8;
            }
            if n > 0 {
                let v = rng_next($idx).to_le_bytes();
                std::ptr::copy_nonoverlapping(v.as_ptr(), p, n);
            }
        }
        unsafe extern "C" fn $stir() {}
        unsafe extern "C" fn $close() -> c_int {
            0
        }
    };
}

rng_impl!(0, c_name, c_random, c_buf, c_stir, c_close);
rng_impl!(1, r_name, r_random, r_buf, r_stir, r_close);

unsafe fn install_deterministic_rng(l: &Libs) {
    // `uniform` is left NULL on purpose so that the library's own
    // randombytes_uniform() rejection-sampling loop is what gets tested.
    let c_impl: &'static RandombytesImplementation = Box::leak(Box::new(RandombytesImplementation {
        implementation_name: Some(c_name),
        random: Some(c_random),
        stir: Some(c_stir),
        uniform: None,
        buf: Some(c_buf),
        close: Some(c_close),
    }));
    let r_impl: &'static RandombytesImplementation = Box::leak(Box::new(RandombytesImplementation {
        implementation_name: Some(r_name),
        random: Some(r_random),
        stir: Some(r_stir),
        uniform: None,
        buf: Some(r_buf),
        close: Some(r_close),
    }));
    type SetImpl = unsafe extern "C" fn(*const RandombytesImplementation) -> c_int;
    let cs: Symbol<SetImpl> = l.c.get(b"randombytes_set_implementation\0").unwrap();
    let rs: Symbol<SetImpl> = l.r.get(b"randombytes_set_implementation\0").unwrap();
    assert_eq!(cs(c_impl as *const _), 0);
    assert_eq!(rs(r_impl as *const _), 0);
}

// ----------------------------------------------------------- test-input PRNG

/// Deterministic, reproducible generator for test inputs (splitmix64).
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed ^ 0x9E37_79B9_7F4A_7C15)
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
    pub fn byte(&mut self) -> u8 {
        (self.next_u64() >> 56) as u8
    }
    /// Uniform in `[0, n)`; returns 0 when `n == 0`.
    pub fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next_u64() % n as u64) as usize
        }
    }
    /// Uniform in `[lo, hi]`.
    pub fn range(&mut self, lo: usize, hi: usize) -> usize {
        assert!(lo <= hi);
        lo + self.below(hi - lo + 1)
    }
    pub fn bytes(&mut self, n: usize) -> Vec<u8> {
        let mut v = vec![0u8; n];
        self.fill(&mut v);
        v
    }
    pub fn fill(&mut self, out: &mut [u8]) {
        let mut i = 0;
        while i < out.len() {
            let w = self.next_u64().to_le_bytes();
            let k = std::cmp::min(8, out.len() - i);
            out[i..i + k].copy_from_slice(&w[..k]);
            i += k;
        }
    }
}

// ------------------------------------------------------------------- helpers

/// Byte-for-byte comparison with a readable failure message.
#[track_caller]
pub fn eqb(what: &str, c: &[u8], r: &[u8]) {
    if c != r {
        let n = std::cmp::min(c.len(), r.len());
        let mut first = n;
        for i in 0..n {
            if c[i] != r[i] {
                first = i;
                break;
            }
        }
        panic!(
            "{what}: byte mismatch (C len {}, Rust len {}, first diff at {first})\n  C   : {}\n  Rust: {}",
            c.len(),
            r.len(),
            hex(c),
            hex(r)
        );
    }
}

#[track_caller]
pub fn eqi(what: &str, c: c_int, r: c_int) {
    assert_eq!(c, r, "{what}: return code mismatch (C {c}, Rust {r})");
}

pub fn hex(b: &[u8]) -> String {
    const MAX: usize = 160;
    let mut s = String::new();
    for (i, x) in b.iter().enumerate() {
        if i == MAX {
            s.push_str("...");
            break;
        }
        s.push_str(&format!("{x:02x}"));
    }
    s
}

// ------------------------------------------------------------------- errno

extern "C" {
    fn __errno_location() -> *mut c_int;
    fn fork() -> c_int;
    fn waitpid(pid: c_int, status: *mut c_int, options: c_int) -> c_int;
    fn _exit(code: c_int) -> !;
    fn setrlimit(resource: c_int, rlim: *const RLimit) -> c_int;
}

#[repr(C)]
struct RLimit {
    cur: u64,
    max: u64,
}
const RLIMIT_CORE: c_int = 4;

pub fn errno() -> c_int {
    unsafe { *__errno_location() }
}
pub fn set_errno(v: c_int) {
    unsafe { *__errno_location() = v }
}

pub const EINVAL: c_int = 22;
pub const ERANGE: c_int = 34;
pub const ENOMEM: c_int = 12;
pub const EPERM: c_int = 1;

/// Run `f` in a forked child and return the raw `waitpid` status.
///
/// Used for the `sodium_misuse()` / `abort()` paths, which cannot be observed
/// in-process.  The child `_exit(0)`s if `f` returns normally, so a normal exit
/// and a fatal signal are clearly distinguishable.
pub fn in_child<F: FnOnce()>(f: F) -> c_int {
    unsafe {
        let pid = fork();
        assert!(pid >= 0, "fork failed");
        if pid == 0 {
            // No core dumps: the abort() paths are expected, and dumping cores
            // makes the error-path tests hundreds of times slower.
            setrlimit(RLIMIT_CORE, &RLimit { cur: 0, max: 0 });
            f();
            _exit(0);
        }
        let mut st: c_int = 0;
        assert!(waitpid(pid, &mut st, 0) == pid);
        st
    }
}

/// Human-readable classification of a `waitpid` status: `"exit:N"` or `"sig:N"`.
pub fn status_str(st: c_int) -> String {
    if st & 0x7f == 0 {
        format!("exit:{}", (st >> 8) & 0xff)
    } else {
        format!("sig:{}", st & 0x7f)
    }
}

/// Differentially compare the *process outcome* of a pair of calls that may
/// abort (`sodium_misuse`).
#[track_caller]
pub fn eq_abort<FC: FnOnce(), FR: FnOnce()>(what: &str, c: FC, r: FR) {
    let sc = in_child(c);
    let sr = in_child(r);
    assert_eq!(
        status_str(sc),
        status_str(sr),
        "{what}: process outcome mismatch"
    );
}

/// A guard byte pattern used to prove neither implementation writes out of
/// bounds; allocate `len + PAD` and check the tail afterwards.
pub const PAD: usize = 32;

pub fn padded(len: usize) -> Vec<u8> {
    let mut v = vec![0u8; len + PAD];
    for (i, b) in v[len..].iter_mut().enumerate() {
        *b = 0xA5u8.wrapping_add(i as u8);
    }
    v
}

#[track_caller]
pub fn check_pad(what: &str, v: &[u8], len: usize) {
    for (i, b) in v[len..].iter().enumerate() {
        assert_eq!(
            *b,
            0xA5u8.wrapping_add(i as u8),
            "{what}: out-of-bounds write at +{i} past {len}"
        );
    }
}
