//! Shared differential-test harness.
//!
//! Loads BOTH the C reference `libsodium.so` and the Rust `liblibsodium.so`
//! through `libloading` and exposes them as a pair, so every test calls both
//! implementations exactly the way an external consumer would (through the
//! dynamic-symbol table), never through Rust-internal calls.
#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::path::PathBuf;
use std::sync::OnceLock;

pub const C_ENV: &str = "SODIUM_C_SO";
pub const R_ENV: &str = "SODIUM_RUST_SO";

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var(C_ENV) {
        return PathBuf::from(p);
    }
    let root = crate_root();
    let ws = root.parent().expect("crate parent");
    let cand = ws.join("c_src/build/libsodium.so");
    assert!(
        cand.exists(),
        "C shared library not found at {cand:?}; build it first (see task instructions) \
         or set {C_ENV}"
    );
    cand
}

fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var(R_ENV) {
        return PathBuf::from(p);
    }
    let root = crate_root();
    // The test binary lives in target/<profile>/deps/<name>; walk up to the
    // profile dir so we pick the .so from the same profile we were built with.
    let exe = std::env::current_exe().expect("current_exe");
    let profile_dir = exe
        .parent()
        .and_then(|p| p.parent())
        .map(|p| p.to_path_buf());
    let mut cands: Vec<PathBuf> = Vec::new();
    if let Some(d) = profile_dir {
        cands.push(d.join("liblibsodium.so"));
    }
    cands.push(root.join("target/debug/liblibsodium.so"));
    cands.push(root.join("target/release/liblibsodium.so"));
    // `cargo test` does NOT rebuild a `cdylib`-only lib target for integration
    // tests, so several candidates may exist. Always take the most recently
    // built one, and shout if it is older than the newest source file.
    let mut best: Option<(PathBuf, std::time::SystemTime)> = None;
    for c in &cands {
        if let Ok(md) = std::fs::metadata(c) {
            let t = md.modified().unwrap_or(std::time::UNIX_EPOCH);
            if best.as_ref().map(|(_, bt)| t > *bt).unwrap_or(true) {
                best = Some((c.clone(), t));
            }
        }
    }
    let (path, built) = best.unwrap_or_else(|| {
        panic!("Rust cdylib not found; tried {cands:?}. Run `./dt.sh` (or `cargo build --release`) first, or set {R_ENV}")
    });
    if let Ok(rd) = std::fs::read_dir(root.join("src")) {
        for e in rd.flatten() {
            if let Ok(md) = e.metadata() {
                if md.modified().map(|t| t > built).unwrap_or(false) {
                    panic!(
                        "STALE Rust cdylib: {} is older than {}. \
                         Rebuild with `./dt.sh` (cargo build --release && cargo test).",
                        path.display(),
                        e.path().display()
                    );
                }
            }
        }
    }
    path
}

/// A pair of loaded libraries: `.0` is C (ground truth), `.1` is Rust.
pub struct Libs {
    pub c: &'static Library,
    pub rs: &'static Library,
}

static LIBS: OnceLock<Libs> = OnceLock::new();

/// The `randombytes_implementation` vtable, byte-compatible with
/// `randombytes.h`.
#[repr(C)]
pub struct RandombytesImpl {
    pub implementation_name: Option<extern "C" fn() -> *const std::os::raw::c_char>,
    pub random: Option<extern "C" fn() -> u32>,
    pub stir: Option<extern "C" fn()>,
    pub uniform: Option<extern "C" fn(u32) -> u32>,
    pub buf: Option<extern "C" fn(*mut std::os::raw::c_void, usize)>,
    pub close: Option<extern "C" fn() -> i32>,
}
unsafe impl Sync for RandombytesImpl {}

// ---------------------------------------------------------------------------
// Deterministic randombytes implementation, installed into BOTH libraries so
// that every entry point that internally draws randomness (keypair
// generation, `*_keygen`, `crypto_core_*_random`, ...) becomes reproducible
// and therefore differentially comparable byte-for-byte.
//
// It is a plain counter-seeded ChaCha-free xorshift stream: the exact bytes do
// not matter, only that the two libraries observe the SAME stream.
// ---------------------------------------------------------------------------
use std::sync::atomic::{AtomicU64, Ordering};

static DET_STATE: AtomicU64 = AtomicU64::new(0x2545_F491_4F6C_DD1D);

pub fn det_reseed(seed: u64) {
    DET_STATE.store(seed | 1, Ordering::SeqCst);
}

fn det_next_u64() -> u64 {
    // splitmix64
    let mut z = DET_STATE.fetch_add(0x9E37_79B9_7F4A_7C15, Ordering::SeqCst)
        .wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

const DET_NAME: &[u8] = b"harness_det\0";

extern "C" fn det_name() -> *const std::os::raw::c_char {
    DET_NAME.as_ptr() as *const _
}

extern "C" fn det_random() -> u32 {
    det_next_u64() as u32
}

extern "C" fn det_stir() {}

extern "C" fn det_buf(buf: *mut std::os::raw::c_void, size: usize) {
    if buf.is_null() || size == 0 {
        return;
    }
    let out = unsafe { std::slice::from_raw_parts_mut(buf as *mut u8, size) };
    let mut i = 0usize;
    while i < size {
        let v = det_next_u64().to_le_bytes();
        let n = core::cmp::min(8, size - i);
        out[i..i + n].copy_from_slice(&v[..n]);
        i += n;
    }
}

extern "C" fn det_close() -> i32 {
    0
}

pub static DET_IMPL: RandombytesImpl = RandombytesImpl {
    implementation_name: Some(det_name),
    random: Some(det_random),
    stir: Some(det_stir),
    uniform: None, // exercise the library's own rejection-sampling code
    buf: Some(det_buf),
    close: Some(det_close),
};

/// Load (once) and initialise both libraries.
pub fn libs() -> &'static Libs {
    LIBS.get_or_init(|| {
        let c = unsafe { Library::new(c_so_path()) }.expect("load C libsodium.so");
        let rs = unsafe { Library::new(rust_so_path()) }.expect("load Rust liblibsodium.so");
        let c: &'static Library = Box::leak(Box::new(c));
        let rs: &'static Library = Box::leak(Box::new(rs));
        for l in [c, rs] {
            let init: Symbol<unsafe extern "C" fn() -> i32> =
                unsafe { l.get(b"sodium_init\0") }.expect("sodium_init");
            let r = unsafe { init() };
            assert!(r == 0 || r == 1, "sodium_init returned {r}");
        }
        Libs { c, rs }
    })
}

/// Install the deterministic randombytes implementation in both libraries.
/// Idempotent; safe to call from every test.
pub fn install_det_random() {
    static DONE: OnceLock<()> = OnceLock::new();
    DONE.get_or_init(|| {
        let l = libs();
        for lib in [l.c, l.rs] {
            let f: Symbol<unsafe extern "C" fn(*const RandombytesImpl) -> i32> =
                unsafe { lib.get(b"randombytes_set_implementation\0") }.unwrap();
            let r = unsafe { f(&DET_IMPL as *const RandombytesImpl) };
            assert_eq!(r, 0, "randombytes_set_implementation failed");
        }
    });
}

/// Fetch the same symbol from both libraries, typed as `F`.
///
/// # Safety
/// The caller must supply a signature matching the C declaration.
pub unsafe fn pair<F>(name: &str) -> (Symbol<'static, F>, Symbol<'static, F>) {
    let l = libs();
    let mut n = name.as_bytes().to_vec();
    n.push(0);
    let a: Symbol<'static, F> = l
        .c
        .get(&n)
        .unwrap_or_else(|e| panic!("C symbol {name} missing: {e}"));
    let b: Symbol<'static, F> = l
        .rs
        .get(&n)
        .unwrap_or_else(|e| panic!("Rust symbol {name} missing: {e}"));
    (a, b)
}

/// True when the symbol exists in both libraries.
pub fn has_sym(name: &str) -> bool {
    let l = libs();
    let mut n = name.as_bytes().to_vec();
    n.push(0);
    unsafe {
        l.c.get::<*const ()>(&n).is_ok() && l.rs.get::<*const ()>(&n).is_ok()
    }
}

// ---------------------------------------------------------------------------
// Deterministic test RNG (for generating inputs, independent of the library)
// ---------------------------------------------------------------------------
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed ^ 0x9E37_79B9_7F4A_7C15)
    }
    pub fn next_u64(&mut self) -> u64 {
        // xorshift64*
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
    pub fn byte(&mut self) -> u8 {
        (self.next_u64() >> 56) as u8
    }
    pub fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next_u64() % n as u64) as usize
        }
    }
    pub fn range(&mut self, lo: usize, hi_incl: usize) -> usize {
        lo + self.below(hi_incl - lo + 1)
    }
    pub fn bytes(&mut self, n: usize) -> Vec<u8> {
        (0..n).map(|_| self.byte()).collect()
    }
    pub fn fill(&mut self, b: &mut [u8]) {
        for x in b.iter_mut() {
            *x = self.byte();
        }
    }
}

pub fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

/// Assert two byte slices are identical, printing a hex diff on failure.
#[track_caller]
pub fn eq_bytes(ctx: &str, c: &[u8], r: &[u8]) {
    if c != r {
        let at = c.iter().zip(r).position(|(a, b)| a != b);
        panic!(
            "{ctx}: C/Rust output differs (first diff at byte {at:?})\n  C   = {}\n  Rust= {}",
            hex(c),
            hex(r)
        );
    }
}

#[track_caller]
pub fn eq_i32(ctx: &str, c: i32, r: i32) {
    assert_eq!(c, r, "{ctx}: return value differs (C={c}, Rust={r})");
}

// ---------------------------------------------------------------------------
// errno access (both libraries share this thread's errno via glibc)
// ---------------------------------------------------------------------------
use std::os::raw::c_int;

fn errno_location() -> *mut c_int {
    // Cache only the FUNCTION pointer, never the address it returns: errno
    // lives in thread-local storage, so the address differs per thread (the
    // test harness runs every #[test] on its own thread).
    static F: OnceLock<usize> = OnceLock::new();
    let fp = *F.get_or_init(|| unsafe {
        let this = libloading::os::unix::Library::this();
        let f: libloading::os::unix::Symbol<unsafe extern "C" fn() -> *mut c_int> =
            this.get(b"__errno_location\0").expect("__errno_location");
        let fp: unsafe extern "C" fn() -> *mut c_int = *f;
        std::mem::forget(this);
        fp as usize
    });
    unsafe {
        let f: unsafe extern "C" fn() -> *mut c_int = std::mem::transmute(fp);
        f()
    }
}

pub fn errno() -> i32 {
    unsafe { *errno_location() }
}

pub fn set_errno(v: i32) {
    unsafe { *errno_location() = v }
}

pub const EPERM: i32 = 1;
pub const ENOMEM: i32 = 12;
pub const EINVAL: i32 = 22;
pub const ENOSYS: i32 = 38;
pub const ERANGE: i32 = 34;

// ---------------------------------------------------------------------------
// Abort-path differential testing.
//
// Several libsodium rejection paths do not return an error code: they call
// `sodium_misuse()` / `assert()` and kill the process. To compare those between
// C and Rust we re-exec THIS test binary in a child process, told (via env) to
// run one specific case against one specific library, and compare how the two
// children terminated (exit code / signal).
// ---------------------------------------------------------------------------
pub const CASE_ENV: &str = "SODIUM_DIFF_CASE";
pub const LIB_ENV: &str = "SODIUM_DIFF_LIB";
pub const CHILD_TEST: &str = "zz_abort_child";

#[derive(Debug, PartialEq, Eq)]
pub struct Term {
    pub code: Option<i32>,
    pub signal: Option<i32>,
}

fn run_child(case: &str, lib: &str) -> Term {
    use std::os::unix::process::ExitStatusExt;
    let exe = std::env::current_exe().expect("current_exe");
    let out = std::process::Command::new(exe)
        .arg("--exact")
        .arg(CHILD_TEST)
        .arg("--nocapture")
        .arg("--test-threads=1")
        .env(CASE_ENV, case)
        .env(LIB_ENV, lib)
        .env(C_ENV, c_so_path())
        .env(R_ENV, rust_so_path())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .output()
        .expect("spawn child");
    Term {
        code: out.status.code(),
        signal: out.status.signal(),
    }
}

/// Which library the current (child) process should exercise, if any.
pub fn child_case() -> Option<(String, bool)> {
    let case = std::env::var(CASE_ENV).ok()?;
    let lib = std::env::var(LIB_ENV).unwrap_or_else(|_| "c".into());
    Some((case, lib == "c"))
}

/// Run `case` in two child processes (C and Rust) and assert they terminated
/// identically. Returns the shared termination so the caller can additionally
/// assert it was a signal / a clean exit.
#[track_caller]
pub fn diff_abort_case(case: &str) -> Term {
    let c = run_child(case, "c");
    let r = run_child(case, "rust");
    assert_eq!(
        c, r,
        "abort-path case {case:?}: C terminated {c:?} but Rust terminated {r:?}"
    );
    c
}

/// Assert the case aborts (SIGABRT=6 / SIGSEGV=11 / SIGKILL=9 / SIGBUS=7)
/// identically in both libraries.
#[track_caller]
pub fn assert_both_abort(case: &str) {
    let t = diff_abort_case(case);
    assert!(
        t.signal.is_some() || t.code == Some(101) || t.code.map(|c| c != 0).unwrap_or(false),
        "case {case:?}: expected abnormal termination in both, got {t:?}"
    );
}
