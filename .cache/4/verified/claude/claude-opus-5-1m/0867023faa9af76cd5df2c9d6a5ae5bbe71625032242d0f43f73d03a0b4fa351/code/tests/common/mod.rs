//! Shared differential-test harness.
//!
//! Both the C reference `.so` (`c_src/build/libsodium.so`) and the Rust
//! `.so` (`target/<profile>/liblibsodium.so`) are loaded with `libloading`
//! and driven **only** through their exported symbols, exactly as an external
//! `dlopen` consumer would. No Rust function is ever called directly, so the
//! `#[no_mangle]` / `extern "C"` export wrappers are part of what is tested.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::c_void;
use std::path::PathBuf;
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// library loading
// ---------------------------------------------------------------------------

static C_LIB: OnceLock<Library> = OnceLock::new();
static R_LIB: OnceLock<Library> = OnceLock::new();

fn c_so_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("c_src/build/libsodium.so")
}

fn r_so_path() -> PathBuf {
    // current_exe = <target>/<profile>/deps/<testbin>
    let exe = std::env::current_exe().expect("current_exe");
    let deps = exe.parent().expect("deps dir");
    let profile = deps.parent().expect("profile dir");
    for cand in [
        profile.join("liblibsodium.so"),
        deps.join("liblibsodium.so"),
    ] {
        if cand.exists() {
            return cand;
        }
    }
    panic!(
        "Rust cdylib liblibsodium.so not found next to {:?}; run `cargo build` first",
        exe
    );
}

pub fn c_lib() -> &'static Library {
    C_LIB.get_or_init(|| {
        let p = c_so_path();
        assert!(
            p.exists(),
            "C shared library missing at {p:?}. Build it with:\n  \
             cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build ."
        );
        unsafe { Library::new(&p) }.unwrap_or_else(|e| panic!("dlopen {p:?}: {e}"))
    })
}

pub fn r_lib() -> &'static Library {
    R_LIB.get_or_init(|| {
        let p = r_so_path();
        unsafe { Library::new(&p) }.unwrap_or_else(|e| panic!("dlopen {p:?}: {e}"))
    })
}

/// Look a symbol up in one library, transmuting it to the requested fn type.
pub fn sym<T: Copy>(lib: &'static Library, name: &str) -> T {
    let s: Symbol<T> = unsafe { lib.get(name.as_bytes()) }
        .unwrap_or_else(|e| panic!("symbol `{name}` not found: {e}"));
    *s
}

/// Look the same symbol up in **both** libraries: `(c_fn, rust_fn)`.
pub fn pair<T: Copy>(name: &str) -> (T, T) {
    (sym::<T>(c_lib(), name), sym::<T>(r_lib(), name))
}

/// `true` when the symbol is present in both libraries.
pub fn has_sym(name: &str) -> bool {
    unsafe { c_lib().get::<*const c_void>(name.as_bytes()) }.is_ok()
        && unsafe { r_lib().get::<*const c_void>(name.as_bytes()) }.is_ok()
}

// ---------------------------------------------------------------------------
// deterministic PRNG for the *test driver* (reproducible inputs)
// ---------------------------------------------------------------------------

/// splitmix64 — tiny, fixed, reproducible.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed.wrapping_add(0x9E37_79B9_7F4A_7C15))
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
    /// Uniform in `[0, n)`.
    pub fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            return 0;
        }
        (self.next_u64() % n as u64) as usize
    }
    /// Uniform in `[lo, hi]`.
    pub fn range(&mut self, lo: usize, hi: usize) -> usize {
        assert!(lo <= hi);
        lo + self.below(hi - lo + 1)
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
    pub fn bool(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }
    /// Pick one element of a slice.
    pub fn pick<'a, T>(&mut self, s: &'a [T]) -> &'a T {
        &s[self.below(s.len())]
    }
}

// ---------------------------------------------------------------------------
// deterministic `randombytes` implementation installed into BOTH libraries
// ---------------------------------------------------------------------------

#[repr(C)]
pub struct RandombytesImpl {
    pub implementation_name: Option<unsafe extern "C" fn() -> *const i8>,
    pub random: Option<unsafe extern "C" fn() -> u32>,
    pub stir: Option<unsafe extern "C" fn()>,
    pub uniform: Option<unsafe extern "C" fn(u32) -> u32>,
    pub buf: Option<unsafe extern "C" fn(*mut c_void, usize)>,
    pub close: Option<unsafe extern "C" fn() -> i32>,
}
unsafe impl Sync for RandombytesImpl {}

// Two *independent* PRNG states, one per loaded library, so that the C side
// and the Rust side each observe the identical byte stream when they are given
// the identical sequence of requests (see `reset_rngs`).
//
// The states are **thread-local**. The test harness runs the tests of one
// binary concurrently on several threads, and the usual usage pattern is
//
//     reset_rngs(s); c_fn(...);        // draw from the C-side stream
//     reset_rngs(s); rust_fn(...);     // draw from the Rust-side stream
//
// With process-global state a second test resetting the seed between those two
// halves would silently give the two libraries different streams and produce a
// spurious mismatch. Per-thread state makes the pattern race-free without any
// locking, because the library invokes these callbacks synchronously on the
// calling thread.
thread_local! {
    static STATE_C: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
    static STATE_R: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

const NAME: &[u8] = b"difftest-deterministic\0";

fn sm64_step(s: u64) -> (u64, u64) {
    let s = s.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = s;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    (s, z ^ (z >> 31))
}

unsafe extern "C" fn name_fn() -> *const i8 {
    NAME.as_ptr() as *const i8
}

macro_rules! det_impl {
    ($state:ident, $rnd:ident, $buf:ident, $stir:ident, $close:ident, $impl:ident) => {
        unsafe extern "C" fn $rnd() -> u32 {
            $state.with(|c| {
                let (s, v) = sm64_step(c.get());
                c.set(s);
                (v >> 32) as u32
            })
        }
        unsafe extern "C" fn $buf(p: *mut c_void, n: usize) {
            $state.with(|c| {
                let mut st = c.get();
                let mut i = 0usize;
                while i < n {
                    let (s, v) = sm64_step(st);
                    st = s;
                    let w = v.to_le_bytes();
                    let take = core::cmp::min(8, n - i);
                    unsafe {
                        core::ptr::copy_nonoverlapping(
                            w.as_ptr(),
                            (p as *mut u8).add(i),
                            take,
                        );
                    }
                    i += take;
                }
                c.set(st);
            })
        }
        unsafe extern "C" fn $stir() {}
        unsafe extern "C" fn $close() -> i32 {
            0
        }
        pub static $impl: RandombytesImpl = RandombytesImpl {
            implementation_name: Some(name_fn),
            random: Some($rnd),
            stir: Some($stir),
            uniform: None,
            buf: Some($buf),
            close: Some($close),
        };
    };
}

det_impl!(STATE_C, c_random, c_buf, c_stir, c_close, IMPL_C);
det_impl!(STATE_R, r_random, r_buf, r_stir, r_close, IMPL_R);

pub const RNG_SEED_BASE: u64 = 0x5EED_1234_ABCD_0001;

/// Rewind **this thread's** copy of both libraries' deterministic RNG streams
/// to the same seed. Call it immediately before each of the two calls in a pair
/// whose result depends on `randombytes_*`, so both sides consume the identical
/// byte stream.
pub fn reset_rngs(seed: u64) {
    STATE_C.with(|c| c.set(seed));
    STATE_R.with(|c| c.set(seed));
}

type SetImpl = unsafe extern "C" fn(*const RandombytesImpl) -> i32;
type Init = unsafe extern "C" fn() -> i32;

static SETUP: OnceLock<()> = OnceLock::new();

/// Install the deterministic RNG into both libraries and run `sodium_init()`
/// on both. Idempotent; every test should call it first.
pub fn setup() {
    SETUP.get_or_init(|| {
        let (c_set, r_set) = pair::<SetImpl>("randombytes_set_implementation");
        unsafe {
            assert_eq!(c_set(&raw const IMPL_C), 0);
            assert_eq!(r_set(&raw const IMPL_R), 0);
        }
        reset_rngs(RNG_SEED_BASE);
        let (c_init, r_init) = pair::<Init>("sodium_init");
        unsafe {
            let a = c_init();
            let b = r_init();
            assert!(a >= 0, "C sodium_init failed: {a}");
            assert!(b >= 0, "Rust sodium_init failed: {b}");
        }
    });
}

// ---------------------------------------------------------------------------
// comparison helpers
// ---------------------------------------------------------------------------

pub fn hex(b: &[u8]) -> String {
    let mut s = String::with_capacity(b.len() * 2);
    for x in b {
        s.push_str(&format!("{x:02x}"));
    }
    s
}

/// Assert two byte buffers are identical, printing a readable diff.
#[track_caller]
pub fn eq_bytes(what: &str, c: &[u8], r: &[u8]) {
    if c != r {
        let at = c
            .iter()
            .zip(r.iter())
            .position(|(a, b)| a != b)
            .unwrap_or(c.len().min(r.len()));
        panic!(
            "{what}: byte mismatch at offset {at} (len C={} R={})\n  C = {}\n  R = {}",
            c.len(),
            r.len(),
            hex(&c[..c.len().min(512)]),
            hex(&r[..r.len().min(512)]),
        );
    }
}

#[track_caller]
pub fn eq_i32(what: &str, c: i32, r: i32) {
    assert_eq!(c, r, "{what}: return value mismatch (C={c}, Rust={r})");
}

#[track_caller]
pub fn eq_usize(what: &str, c: usize, r: usize) {
    assert_eq!(c, r, "{what}: return value mismatch (C={c}, Rust={r})");
}

/// A canary-filled output buffer: lets us detect writes past the expected end
/// and compare "what was written" including untouched regions.
pub fn canary(n: usize) -> Vec<u8> {
    vec![0xA5u8; n]
}

// ---------------------------------------------------------------------------
// out-of-process execution (for paths where the C code `abort()`s:
// `assert()` failures and `sodium_misuse()`)
// ---------------------------------------------------------------------------

/// Re-executes the current test binary with `DIFFTEST_CHILD=<tag>` set and a
/// filter selecting `child_test_name`. Returns the child's raw exit status.
///
/// Used for differential testing of abort/misuse paths: the same helper runs
/// once for the C library and once for the Rust library, and the two exit
/// statuses must match.
pub fn run_child(child_test_name: &str, which: &str, tag: &str) -> std::process::Output {
    let exe = std::env::current_exe().unwrap();
    std::process::Command::new(exe)
        .arg("--exact")
        .arg(child_test_name)
        .arg("--nocapture")
        .arg("--test-threads=1")
        .env("DIFFTEST_CHILD", tag)
        .env("DIFFTEST_WHICH", which)
        .output()
        .expect("spawn child")
}

pub fn child_tag() -> Option<String> {
    std::env::var("DIFFTEST_CHILD").ok()
}

pub fn child_which() -> String {
    std::env::var("DIFFTEST_WHICH").unwrap_or_default()
}

/// In the child process: pick the library named by `DIFFTEST_WHICH`.
pub fn child_lib() -> &'static Library {
    if child_which() == "c" { c_lib() } else { r_lib() }
}

/// Compare two child outcomes: same exit code and same termination signal.
#[track_caller]
pub fn eq_status(what: &str, c: &std::process::Output, r: &std::process::Output) {
    use std::os::unix::process::ExitStatusExt;
    let (cc, cs) = (c.status.code(), c.status.signal());
    let (rc, rs) = (r.status.code(), r.status.signal());
    assert_eq!(
        (cc, cs),
        (rc, rs),
        "{what}: process outcome mismatch\n  C: code={cc:?} signal={cs:?}\n  \
         R: code={rc:?} signal={rs:?}\n  C stderr: {}\n  R stderr: {}",
        String::from_utf8_lossy(&c.stderr),
        String::from_utf8_lossy(&r.stderr),
    );
}

// ---------------------------------------------------------------------------
// aligned, opaque state buffers
// ---------------------------------------------------------------------------

/// A 64-byte-aligned heap buffer, used for the library's opaque `*_state`
/// structures (whose real alignment requirement is up to 64 bytes for the
/// BLAKE2b / Keccak / AES-GCM states).
pub struct State {
    ptr: *mut u8,
    layout: std::alloc::Layout,
    len: usize,
}

impl State {
    pub fn new(len: usize) -> State {
        let layout = std::alloc::Layout::from_size_align(len.max(1), 64).unwrap();
        let ptr = unsafe { std::alloc::alloc_zeroed(layout) };
        assert!(!ptr.is_null(), "alloc {len}");
        State { ptr, layout, len }
    }
    /// Allocate a state buffer sized by the library's own `*_statebytes()`.
    pub fn for_sym(statebytes_fn: &str) -> State {
        let f = sym::<unsafe extern "C" fn() -> usize>(c_lib(), statebytes_fn);
        State::new(unsafe { f() })
    }
    pub fn as_mut_ptr(&mut self) -> *mut u8 {
        self.ptr
    }
    pub fn as_ptr(&self) -> *const u8 {
        self.ptr
    }
    pub fn len(&self) -> usize {
        self.len
    }
    pub fn bytes(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.ptr, self.len) }
    }
}

impl Drop for State {
    fn drop(&mut self) {
        unsafe { std::alloc::dealloc(self.ptr, self.layout) }
    }
}

/// Split `total` into a randomised list of chunk sizes summing to `total`.
pub fn chunks(rng: &mut Rng, total: usize, style: u32) -> Vec<usize> {
    match style {
        0 => {
            if total == 0 {
                vec![]
            } else {
                vec![total]
            }
        }
        1 => vec![1; total],                        // one byte at a time
        2 => {
            // two chunks, split at a random point (including 0 and total)
            let a = rng.below(total + 1);
            vec![a, total - a]
        }
        _ => {
            // random walk
            let mut v = Vec::new();
            let mut left = total;
            while left > 0 {
                let cap = 1 + rng.below(70);
                let n = rng.range(1, left.min(cap).max(1));
                v.push(n);
                left -= n;
            }
            v
        }
    }
}

// ---------------------------------------------------------------------------
// sodium_misuse() observation
//
// `sodium_misuse()` calls the installed handler and *then* `abort()`s, so a
// handler that prints the observable state and `exit()`s turns an otherwise
// opaque SIGABRT into a precisely comparable outcome: the same exit code AND
// the same side effects (out-parameters written before the abort).
// ---------------------------------------------------------------------------

use std::sync::atomic::{AtomicPtr, AtomicUsize, Ordering};

/// Exit code the misuse handler uses. Distinct from any libsodium value.
pub const MISUSE_EXIT: i32 = 77;

static OBS_PTR: AtomicPtr<u8> = AtomicPtr::new(std::ptr::null_mut());
static OBS_LEN: AtomicUsize = AtomicUsize::new(0);

/// Register a byte range whose contents the misuse handler will print.
pub fn set_observation(p: *const u8, len: usize) {
    OBS_PTR.store(p as *mut u8, Ordering::SeqCst);
    OBS_LEN.store(len, Ordering::SeqCst);
}

pub unsafe extern "C" fn misuse_handler() {
    let p = OBS_PTR.load(Ordering::SeqCst);
    let n = OBS_LEN.load(Ordering::SeqCst);
    let s = if p.is_null() || n == 0 {
        String::new()
    } else {
        hex(unsafe { std::slice::from_raw_parts(p, n) })
    };
    println!("MISUSE obs={s}");
    use std::io::Write;
    let _ = std::io::stdout().flush();
    std::process::exit(MISUSE_EXIT);
}

type SetMisuse = unsafe extern "C" fn(Option<unsafe extern "C" fn()>) -> i32;

/// Install the observing misuse handler into one library.
pub fn install_misuse_handler(lib: &'static Library) {
    let f = sym::<SetMisuse>(lib, "sodium_set_misuse_handler");
    let rc = unsafe { f(Some(misuse_handler)) };
    assert_eq!(rc, 0, "sodium_set_misuse_handler");
}

/// Compare two child processes: identical exit status AND identical stdout.
#[track_caller]
pub fn eq_child(what: &str, c: &std::process::Output, r: &std::process::Output) {
    eq_status(what, c, r);
    let (co, ro) = (
        String::from_utf8_lossy(&c.stdout).to_string(),
        String::from_utf8_lossy(&r.stdout).to_string(),
    );
    let pick = |s: &str| -> String {
        s.lines()
            .filter(|l| l.starts_with("MISUSE ") || l.starts_with("OBS "))
            .collect::<Vec<_>>()
            .join("\n")
    };
    assert_eq!(
        pick(&co),
        pick(&ro),
        "{what}: observed side effects differ\n  C  stdout: {co}\n  R  stdout: {ro}"
    );
}

/// Body of the `#[test]` that a parent re-execs. Returns the tag to run, or
/// `None` when this process is the parent (in which case the test is a no-op).
pub fn child_case() -> Option<(String, &'static Library)> {
    let tag = child_tag()?;
    let lib = child_lib();
    setup();
    install_misuse_handler(lib);
    Some((tag, lib))
}
