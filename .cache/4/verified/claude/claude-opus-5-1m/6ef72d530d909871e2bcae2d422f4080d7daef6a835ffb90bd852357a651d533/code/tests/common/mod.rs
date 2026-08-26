//! Shared harness: loads BOTH the C `.so` and the Rust `.so` through
//! `libloading` and exposes their `custom_strdup` exports.
//!
//! The Rust implementation is NEVER called directly — every call goes through
//! the dynamically loaded shared object, exactly like an external C consumer,
//! so the `#[no_mangle] extern "C"` export wrapper is under test too.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int};
use std::path::PathBuf;
use std::sync::OnceLock;

/// ABI of the function under test: `char *custom_strdup(const char *str);`
pub type StrdupFn = unsafe extern "C" fn(*const c_char) -> *mut c_char;

/// Same symbol, viewed with two extra integer parameters. On the SysV / AAPCS
/// ABIs surplus register arguments are simply ignored by the callee; used to
/// probe "garbage / out-of-range scalar passed across the FFI boundary" for an
/// API that has no enum parameter of its own.
pub type StrdupFnExtraArgs = unsafe extern "C" fn(*const c_char, c_int, c_int) -> *mut c_char;

pub fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

pub fn c_so_path() -> PathBuf {
    manifest_dir().join("c_src/build/libdriver.so")
}

/// The Rust cdylib lives next to the test executable's `deps/` directory.
pub fn rust_so_path() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    // .../target/<profile>/deps/<test bin>
    let deps = exe.parent().expect("deps dir");
    let profile = deps.parent().expect("profile dir");
    for cand in [
        profile.join("libdriver.so"),
        deps.join("libdriver.so"),
    ] {
        if cand.exists() {
            return cand;
        }
    }
    panic!(
        "Rust cdylib libdriver.so not found next to {}. Run `cargo build` first.",
        exe.display()
    );
}

struct Loaded {
    c: StrdupFn,
    r: StrdupFn,
    c_extra: StrdupFnExtraArgs,
    r_extra: StrdupFnExtraArgs,
}

static LOADED: OnceLock<Loaded> = OnceLock::new();

fn loaded() -> &'static Loaded {
    LOADED.get_or_init(|| unsafe {
        let cp = c_so_path();
        let rp = rust_so_path();
        assert!(cp.exists(), "missing C .so at {}", cp.display());
        assert!(rp.exists(), "missing Rust .so at {}", rp.display());

        // Leaked on purpose: the libraries (and therefore the code the function
        // pointers point at) must stay mapped for the whole test binary.
        let c: &'static Library =
            Box::leak(Box::new(Library::new(&cp).expect("dlopen C .so")));
        let r: &'static Library =
            Box::leak(Box::new(Library::new(&rp).expect("dlopen Rust .so")));

        let cf: Symbol<StrdupFn> = c.get(b"custom_strdup\0").expect("C custom_strdup");
        let rf: Symbol<StrdupFn> = r.get(b"custom_strdup\0").expect("Rust custom_strdup");
        let cfe: Symbol<StrdupFnExtraArgs> =
            c.get(b"custom_strdup\0").expect("C custom_strdup");
        let rfe: Symbol<StrdupFnExtraArgs> =
            r.get(b"custom_strdup\0").expect("Rust custom_strdup");

        Loaded {
            c: *cf,
            r: *rf,
            c_extra: *cfe,
            r_extra: *rfe,
        }
    })
}

pub fn c_strdup() -> StrdupFn {
    loaded().c
}

pub fn rust_strdup() -> StrdupFn {
    loaded().r
}

pub fn c_strdup_extra() -> StrdupFnExtraArgs {
    loaded().c_extra
}

pub fn rust_strdup_extra() -> StrdupFnExtraArgs {
    loaded().r_extra
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (SplitMix64) — fixed seed, reproducible runs.
// ---------------------------------------------------------------------------

pub const SEED: u64 = 0x5EED_1234_ABCD_EF01;

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
    /// Uniform-ish in `0..n` (n > 0).
    pub fn below(&mut self, n: u64) -> u64 {
        self.next_u64() % n
    }
    /// A byte that can never terminate a C string.
    pub fn nonzero_byte(&mut self) -> u8 {
        1 + (self.below(255) as u8)
    }
    /// Printable ASCII (0x20..=0x7E).
    pub fn ascii_byte(&mut self) -> u8 {
        0x20 + (self.below(0x5F) as u8)
    }
    pub fn nonzero_bytes(&mut self, len: usize) -> Vec<u8> {
        (0..len).map(|_| self.nonzero_byte()).collect()
    }
    pub fn ascii_bytes(&mut self, len: usize) -> Vec<u8> {
        (0..len).map(|_| self.ascii_byte()).collect()
    }
}

// ---------------------------------------------------------------------------
// Differential comparison helpers
// ---------------------------------------------------------------------------

/// Duplicate `payload` (must contain no interior NUL) with both `.so`s and
/// assert byte-identical results. Frees both results with `libc::free`,
/// proving the blocks came from the C allocator.
///
/// Returns nothing; panics with a descriptive message on any divergence.
pub fn assert_same_dup(payload: &[u8], ctx: &str) {
    assert!(
        !payload.contains(&0),
        "{ctx}: test bug, payload has interior NUL"
    );
    let mut buf: Vec<u8> = payload.to_vec();
    buf.push(0);
    unsafe { assert_same_dup_raw(buf.as_ptr() as *const c_char, payload.len(), ctx) };
    // The source must not have been modified by either implementation.
    assert_eq!(&buf[..payload.len()], payload, "{ctx}: source was modified");
    assert_eq!(buf[payload.len()], 0, "{ctx}: source terminator was modified");
}

/// Lower-level variant: `src` is an arbitrary pointer to a NUL-terminated
/// string whose `strlen` is `expected_len`.
pub unsafe fn assert_same_dup_raw(src: *const c_char, expected_len: usize, ctx: &str) {
    let cf = c_strdup();
    let rf = rust_strdup();

    let cres = unsafe { cf(src) };
    let rres = unsafe { rf(src) };

    assert!(!cres.is_null(), "{ctx}: C returned NULL unexpectedly");
    assert!(
        !rres.is_null(),
        "{ctx}: Rust returned NULL while C returned {cres:p}"
    );
    assert_ne!(cres as *const c_char, src, "{ctx}: C returned the source pointer");
    assert_ne!(rres as *const c_char, src, "{ctx}: Rust returned the source pointer");
    assert_ne!(cres, rres, "{ctx}: both returned the same block");

    let want =
        unsafe { std::slice::from_raw_parts(src as *const u8, expected_len + 1) }.to_vec();
    let cgot = unsafe { std::slice::from_raw_parts(cres as *const u8, expected_len + 1) };
    let rgot = unsafe { std::slice::from_raw_parts(rres as *const u8, expected_len + 1) };

    assert_eq!(
        cgot, &want[..],
        "{ctx}: C copy differs from source (len {expected_len})"
    );
    assert_eq!(
        rgot, cgot,
        "{ctx}: Rust copy differs from C copy (len {expected_len})"
    );
    assert_eq!(
        cgot[expected_len], 0,
        "{ctx}: C copy is not NUL terminated"
    );
    assert_eq!(
        rgot[expected_len], 0,
        "{ctx}: Rust copy is not NUL terminated"
    );

    // strlen of both copies must match too (uses the platform strlen).
    let clen = unsafe { libc::strlen(cres) };
    let rlen = unsafe { libc::strlen(rres) };
    assert_eq!(clen, expected_len, "{ctx}: C strlen mismatch");
    assert_eq!(rlen, clen, "{ctx}: Rust strlen mismatch");

    unsafe {
        libc::free(cres as *mut libc::c_void);
        libc::free(rres as *mut libc::c_void);
    }
}
