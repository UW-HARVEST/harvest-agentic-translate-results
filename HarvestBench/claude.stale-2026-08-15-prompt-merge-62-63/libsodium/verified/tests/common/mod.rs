//! Shared differential-test harness.
//!
//! Loads BOTH the C `libsodium.so` and the Rust `liblibsodium.so` via
//! `libloading` and calls every function through `dlsym`, exactly as an
//! external consumer would. Rust functions are NEVER called directly, so the
//! `#[no_mangle]` / `extern "C"` export wrappers are under test too.
#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::c_void;
use std::path::PathBuf;
use std::sync::OnceLock;

// ---------------------------------------------------------------- library load

pub struct Libs {
    pub c: Library,
    pub r: Library,
}

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn find_c_so() -> PathBuf {
    let p = crate_root().join("c_src/build/libsodium.so");
    assert!(
        p.exists(),
        "C shared library not found at {p:?}. Build it with:\n  \
         cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build ."
    );
    p
}

/// Prefer the profile the tests were built with, then fall back.
fn find_rust_so() -> PathBuf {
    let root = crate_root();
    let mut cands = vec![];
    if cfg!(debug_assertions) {
        cands.push(root.join("target/debug/liblibsodium.so"));
        cands.push(root.join("target/release/liblibsodium.so"));
    } else {
        cands.push(root.join("target/release/liblibsodium.so"));
        cands.push(root.join("target/debug/liblibsodium.so"));
    }
    for c in &cands {
        if c.exists() {
            assert_not_stale(c);
            return c.clone();
        }
    }
    panic!("Rust cdylib not found; tried {cands:?}. Run `cargo build`.");
}

/// `cargo test` does NOT rebuild a `cdylib`-only crate for integration tests
/// (there is no dependency edge from the test target to the `.so`), so a stale
/// `.so` would be silently tested and every assertion could pass vacuously.
/// Refuse to run if any `src/**.rs` is newer than the `.so`.
fn assert_not_stale(so: &std::path::Path) {
    let so_t = match so.metadata().and_then(|m| m.modified()) {
        Ok(t) => t,
        Err(_) => return,
    };
    let mut newest: Option<(std::time::SystemTime, PathBuf)> = None;
    let mut stack = vec![crate_root().join("src")];
    while let Some(d) = stack.pop() {
        let rd = match std::fs::read_dir(&d) {
            Ok(x) => x,
            Err(_) => continue,
        };
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
            } else if p.extension().is_some_and(|x| x == "rs") {
                if let Ok(t) = e.metadata().and_then(|m| m.modified()) {
                    if newest.as_ref().is_none_or(|(bt, _)| t > *bt) {
                        newest = Some((t, p));
                    }
                }
            }
        }
    }
    if let Some((t, p)) = newest {
        assert!(
            t <= so_t,
            "STALE Rust .so!\n  {} is newer than\n  {}\n\n\
             `cargo test` does not rebuild a cdylib-only crate for integration \
             tests. Run `cargo build` (or `cargo build --release`) first, \
             otherwise the tests silently run against old code.",
            p.display(),
            so.display()
        );
    }
}

static LIBS: OnceLock<Libs> = OnceLock::new();

pub fn libs() -> &'static Libs {
    LIBS.get_or_init(|| unsafe {
        let c = Library::new(find_c_so()).expect("dlopen C libsodium.so");
        let r = Library::new(find_rust_so()).expect("dlopen Rust liblibsodium.so");
        Libs { c, r }
    })
}

/// Look a symbol up in one library. Panics with the symbol name on failure so a
/// missing export is reported clearly rather than as an opaque dlsym error.
pub unsafe fn sym<T>(lib: &'static Library, name: &str) -> Symbol<'static, T> {
    let mut owned: Vec<u8> = Vec::with_capacity(name.len() + 1);
    owned.extend_from_slice(name.as_bytes());
    owned.push(0);
    lib.get::<T>(&owned)
        .unwrap_or_else(|e| panic!("symbol `{name}` not found: {e}"))
}

/// Get the same symbol from both libraries: `(c_fn, rust_fn)`.
pub unsafe fn pair<T>(name: &str) -> (Symbol<'static, T>, Symbol<'static, T>) {
    let l = libs();
    (sym::<T>(&l.c, name), sym::<T>(&l.r, name))
}

/// Convenience wrapper: fetch a function pair, typed.
#[macro_export]
macro_rules! fnpair {
    ($name:literal, $t:ty) => {{ unsafe { $crate::common::pair::<$t>($name) } }};
}

// ------------------------------------------------------------------- assertion

pub fn assert_eq_bytes(what: &str, c: &[u8], r: &[u8]) {
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
            "{what}: OUTPUT MISMATCH\n  C len={} rust len={}\n  first differing byte at {}\n  \
             C   ={}\n  rust={}",
            c.len(),
            r.len(),
            first,
            hexs(c),
            hexs(r)
        );
    }
}

pub fn hexs(b: &[u8]) -> String {
    const MAX: usize = 96;
    let mut s = String::new();
    for (i, x) in b.iter().enumerate() {
        if i == MAX {
            s.push_str(&format!("... (+{} more)", b.len() - MAX));
            break;
        }
        s.push_str(&format!("{x:02x}"));
    }
    if b.is_empty() {
        s.push_str("<empty>");
    }
    s
}

// ------------------------------------------------------- deterministic PRNG

/// SplitMix64 — reproducible, no external dependency.
pub struct Rng(pub u64);

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
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    pub fn byte(&mut self) -> u8 {
        (self.next_u64() >> 56) as u8
    }
    pub fn fill(&mut self, b: &mut [u8]) {
        for x in b.iter_mut() {
            *x = self.byte();
        }
    }
    pub fn bytes(&mut self, n: usize) -> Vec<u8> {
        let mut v = vec![0u8; n];
        self.fill(&mut v);
        v
    }
    /// Uniform-ish in `[0, n)`.
    pub fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next_u64() % n as u64) as usize
        }
    }
    pub fn pick<'a, T>(&mut self, xs: &'a [T]) -> &'a T {
        &xs[self.below(xs.len())]
    }
}

/// The fixed seed used across all property-style rows, so failures reproduce.
pub const SEED: u64 = 0x5EED_1234_5EED_1234;

// ------------------------------------------------ forked abort/misuse checking

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Outcome {
    /// The call returned normally with this value.
    Returned(i64),
    /// The process died from this signal (SIGABRT == 6 for `sodium_misuse()`).
    Signaled(i32),
}

/// Run `f` in a forked child so that a `sodium_misuse()` -> `abort()` can be
/// observed without killing the test runner. Used for every `misuse` row in
/// ERRORS.md: both the C and the Rust library must abort on the same input.
pub fn forked<F: FnOnce() -> i64>(f: F) -> Outcome {
    unsafe {
        // Flush so the child does not duplicate buffered output.
        libc::fflush(std::ptr::null_mut());
        let pid = libc::fork();
        assert!(pid >= 0, "fork failed");
        if pid == 0 {
            // Child: silence stderr so abort() messages do not spam the log.
            let devnull = libc::open(b"/dev/null\0".as_ptr() as *const libc::c_char, libc::O_WRONLY);
            if devnull >= 0 {
                libc::dup2(devnull, 2);
            }
            let v = f();
            // Encode a normal return in the low 7 bits + a marker.
            libc::_exit(if (0..=100).contains(&v) { v as i32 } else { 101 });
        }
        let mut status: i32 = 0;
        let w = libc::waitpid(pid, &mut status, 0);
        assert_eq!(w, pid, "waitpid failed");
        if libc::WIFSIGNALED(status) {
            Outcome::Signaled(libc::WTERMSIG(status))
        } else {
            Outcome::Returned(libc::WEXITSTATUS(status) as i64)
        }
    }
}

pub const SIGABRT: i32 = 6;

/// Assert that C and Rust behave identically for a branch that ends in
/// `sodium_misuse()` (or any other fatal path).
pub fn assert_same_fatal(what: &str, c: Outcome, r: Outcome) {
    assert_eq!(
        c, r,
        "{what}: C and Rust disagree on the fatal/abort path (C={c:?} rust={r:?})"
    );
}

// ---------------------------------------------------- injected randombytes RNG
//
// `struct randombytes_implementation` — field order and signatures taken
// verbatim from c_src/libsodium/include/sodium/randombytes.h.

#[repr(C)]
pub struct RandombytesImpl {
    pub implementation_name: Option<extern "C" fn() -> *const libc::c_char>,
    pub random: Option<extern "C" fn() -> u32>,
    pub stir: Option<extern "C" fn()>,
    pub uniform: Option<extern "C" fn(u32) -> u32>,
    pub buf: Option<extern "C" fn(*mut c_void, usize)>,
    pub close: Option<extern "C" fn() -> libc::c_int>,
}

// Two INDEPENDENT counter states so the C library and the Rust library each see
// the same deterministic stream instead of interleaving one shared counter.
static mut CTR_C: u64 = 0;
static mut CTR_R: u64 = 0;

const IMPL_NAME: &[u8] = b"difftest\0";

extern "C" fn name_cb() -> *const libc::c_char {
    IMPL_NAME.as_ptr() as *const libc::c_char
}
extern "C" fn stir_cb() {}
extern "C" fn close_cb() -> libc::c_int {
    0
}

#[inline]
fn step(ctr: &mut u64) -> u64 {
    *ctr = ctr.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = *ctr;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

extern "C" fn random_c() -> u32 {
    unsafe { (step(&mut *std::ptr::addr_of_mut!(CTR_C)) >> 32) as u32 }
}
extern "C" fn random_r() -> u32 {
    unsafe { (step(&mut *std::ptr::addr_of_mut!(CTR_R)) >> 32) as u32 }
}
extern "C" fn buf_c(p: *mut c_void, n: usize) {
    let s = unsafe { std::slice::from_raw_parts_mut(p as *mut u8, n) };
    for x in s.iter_mut() {
        *x = unsafe { (step(&mut *std::ptr::addr_of_mut!(CTR_C)) >> 56) as u8 };
    }
}
extern "C" fn buf_r(p: *mut c_void, n: usize) {
    let s = unsafe { std::slice::from_raw_parts_mut(p as *mut u8, n) };
    for x in s.iter_mut() {
        *x = unsafe { (step(&mut *std::ptr::addr_of_mut!(CTR_R)) >> 56) as u8 };
    }
}
/// A custom `uniform` so the delegation path (impl->uniform != NULL) is covered.
extern "C" fn uniform_c(ub: u32) -> u32 {
    if ub < 2 { 0 } else { random_c() % ub }
}
extern "C" fn uniform_r(ub: u32) -> u32 {
    if ub < 2 { 0 } else { random_r() % ub }
}

pub static mut IMPL_C: RandombytesImpl = RandombytesImpl {
    implementation_name: Some(name_cb),
    random: Some(random_c),
    stir: Some(stir_cb),
    uniform: None,
    buf: Some(buf_c),
    close: Some(close_cb),
};
pub static mut IMPL_R: RandombytesImpl = RandombytesImpl {
    implementation_name: Some(name_cb),
    random: Some(random_r),
    stir: Some(stir_cb),
    uniform: None,
    buf: Some(buf_r),
    close: Some(close_cb),
};
pub static mut IMPL_C_UNIF: RandombytesImpl = RandombytesImpl {
    implementation_name: Some(name_cb),
    random: Some(random_c),
    stir: Some(stir_cb),
    uniform: Some(uniform_c),
    buf: Some(buf_c),
    close: Some(close_cb),
};
pub static mut IMPL_R_UNIF: RandombytesImpl = RandombytesImpl {
    implementation_name: Some(name_cb),
    random: Some(random_r),
    stir: Some(stir_cb),
    uniform: Some(uniform_r),
    buf: Some(buf_r),
    close: Some(close_cb),
};

type SetImpl = unsafe extern "C" fn(*const RandombytesImpl) -> libc::c_int;

/// Install the deterministic RNG into BOTH libraries and reset both counters,
/// so that any `*_keygen` / `*_random` output is byte-comparable.
pub fn install_det_rng(with_uniform: bool) {
    let (c, r) = unsafe { pair::<SetImpl>("randombytes_set_implementation") };
    unsafe {
        CTR_C = 0;
        CTR_R = 0;
        if with_uniform {
            c(std::ptr::addr_of!(IMPL_C_UNIF));
            r(std::ptr::addr_of!(IMPL_R_UNIF));
        } else {
            c(std::ptr::addr_of!(IMPL_C));
            r(std::ptr::addr_of!(IMPL_R));
        }
    }
}

/// Reset both deterministic counters to the same point.
pub fn reset_det_rng() {
    unsafe {
        CTR_C = 0;
        CTR_R = 0;
    }
}

// ------------------------------------------------------------------ init once

static INIT: OnceLock<()> = OnceLock::new();

/// Call `sodium_init()` on both libraries exactly once per test process.
pub fn init_both() {
    INIT.get_or_init(|| {
        let (c, r) = unsafe { pair::<unsafe extern "C" fn() -> libc::c_int>("sodium_init") };
        let (rc, rr) = unsafe { (c(), r()) };
        assert_eq!(rc, rr, "sodium_init() first-call return differs");
        assert!(rc == 0 || rc == 1, "sodium_init() returned {rc}");
    });
}

// --------------------------------------------------------------- misc helpers

/// Standard length sweep used by most hash/stream rows.
pub const LENS: &[usize] = &[
    0, 1, 2, 3, 7, 8, 15, 16, 17, 31, 32, 33, 55, 56, 63, 64, 65, 71, 72, 73, 111, 112, 127, 128,
    129, 135, 136, 137, 143, 144, 167, 168, 169, 191, 192, 255, 256, 257, 271, 272, 336, 337, 383,
    384, 512, 513, 1000, 1024,
];

/// Interesting fixed byte patterns for keys/nonces.
pub fn patterns(n: usize, rng: &mut Rng) -> Vec<Vec<u8>> {
    vec![
        vec![0u8; n],
        vec![0xffu8; n],
        (0..n).map(|i| i as u8).collect(),
        rng.bytes(n),
        rng.bytes(n),
    ]
}
