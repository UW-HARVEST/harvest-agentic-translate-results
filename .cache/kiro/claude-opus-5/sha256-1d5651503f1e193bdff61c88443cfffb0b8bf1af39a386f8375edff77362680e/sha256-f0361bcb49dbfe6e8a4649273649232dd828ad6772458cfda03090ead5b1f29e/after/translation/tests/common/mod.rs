//! Shared differential-test harness.
//!
//! Both the C `libdriver.so` and the Rust `libdriver.so` are loaded at runtime
//! with `libloading` and driven **only** through their exported `custom_strdup`
//! symbol. The Rust implementation is never called directly as a Rust function,
//! so every test also exercises the `#[no_mangle] extern "C"` export wrapper
//! exactly as an external C consumer would.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::c_char;
use std::path::PathBuf;
use std::sync::OnceLock;

pub type StrdupFn = unsafe extern "C" fn(*const c_char) -> *mut c_char;

/// One loaded implementation: the `dlopen`ed object plus its resolved symbol.
#[derive(Copy, Clone)]
pub struct Impl {
    pub name: &'static str,
    pub custom_strdup: StrdupFn,
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("DIFF_C_SO") {
        return PathBuf::from(p);
    }
    manifest_dir().join("../c_src/build/libdriver.so")
}

fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("DIFF_RUST_SO") {
        return PathBuf::from(p);
    }
    // Prefer whatever this `cargo test` invocation just built.
    let debug = manifest_dir().join("target/debug/libdriver.so");
    if debug.exists() {
        return debug;
    }
    manifest_dir().join("target/release/libdriver.so")
}

fn load(name: &'static str, path: PathBuf) -> Impl {
    let lib = unsafe { Library::new(&path) }
        .unwrap_or_else(|e| panic!("failed to dlopen {} at {}: {e}", name, path.display()));
    // Leak so the object (and therefore the code the fn pointer points at)
    // stays mapped for the whole process lifetime.
    let lib: &'static Library = Box::leak(Box::new(lib));
    let sym: Symbol<'static, StrdupFn> = unsafe { lib.get(b"custom_strdup\0") }
        .unwrap_or_else(|e| panic!("{name} does not export `custom_strdup`: {e}"));
    Impl { name, custom_strdup: *sym }
}

/// The C implementation, loaded from `c_src/build/libdriver.so`.
pub fn c_impl() -> Impl {
    static C: OnceLock<Impl> = OnceLock::new();
    *C.get_or_init(|| load("C", c_so_path()))
}

/// The Rust implementation, loaded from the crate's `cdylib`.
pub fn rust_impl() -> Impl {
    static R: OnceLock<Impl> = OnceLock::new();
    *R.get_or_init(|| load("Rust", rust_so_path()))
}

// ---------------------------------------------------------------------------
// Deterministic RNG (SplitMix64) — fixed seed, reproducible across runs.
// ---------------------------------------------------------------------------

pub const SEED: u64 = 0x5EED_C0FF_EE00_0001;

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
    pub fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
    }
    pub fn in_range(&mut self, lo: usize, hi_inclusive: usize) -> usize {
        lo + self.below(hi_inclusive - lo + 1)
    }
    /// A byte in `1..=255` — a `0` would terminate the string early.
    pub fn non_nul_byte(&mut self) -> u8 {
        (self.next_u64() % 255) as u8 + 1
    }
    pub fn ascii_byte(&mut self) -> u8 {
        (self.next_u64() % 95) as u8 + 32
    }
    /// NUL-terminated buffer with `len` random non-NUL content bytes.
    pub fn cstring(&mut self, len: usize) -> Vec<u8> {
        let mut v = Vec::with_capacity(len + 1);
        for _ in 0..len {
            v.push(self.non_nul_byte());
        }
        v.push(0);
        v
    }
    pub fn ascii_cstring(&mut self, len: usize) -> Vec<u8> {
        let mut v = Vec::with_capacity(len + 1);
        for _ in 0..len {
            v.push(self.ascii_byte());
        }
        v.push(0);
        v
    }
}

// ---------------------------------------------------------------------------
// Differential assertions
// ---------------------------------------------------------------------------

unsafe fn strlen(p: *const c_char) -> usize {
    let mut n = 0usize;
    while unsafe { *p.add(n) } != 0 {
        n += 1;
    }
    n
}

/// Result of one `custom_strdup` call, captured for comparison.
struct Captured {
    was_null: bool,
    bytes: Vec<u8>, // content including the trailing NUL; empty when null
    ptr: usize,
}

unsafe fn call_and_capture(imp: Impl, input: *const c_char, expect_len: Option<usize>) -> Captured {
    let out = unsafe { (imp.custom_strdup)(input) };
    if out.is_null() {
        return Captured { was_null: true, bytes: Vec::new(), ptr: 0 };
    }
    // Checked BEFORE the free: the C hands back a fresh `malloc` buffer, so a
    // returned pointer equal to the input would mean the implementation aliased
    // the caller's memory. Freeing it would corrupt the heap and abort the
    // process, hiding the real diagnosis behind SIGABRT.
    assert_ne!(
        out as usize, input as usize,
        "{} returned the input pointer instead of a fresh copy",
        imp.name
    );
    // The contract is "a NUL-terminated copy". Read exactly `strlen+1` bytes.
    let n = match expect_len {
        Some(n) => n,
        None => unsafe { strlen(out) },
    };
    let bytes = unsafe { std::slice::from_raw_parts(out as *const u8, n + 1) }.to_vec();
    let ptr = out as usize;
    unsafe { libc::free(out as *mut libc::c_void) };
    Captured { was_null: false, bytes, ptr }
}

/// Call BOTH implementations on `input` and assert byte-identical results.
///
/// `ctx` identifies the case in failure messages. Returns the copied bytes
/// (including the trailing NUL) when both returned non-NULL.
pub fn assert_same(input: *const c_char, ctx: &str) -> Option<Vec<u8>> {
    // Independently determine what the input actually says, so we can also
    // assert the copy is faithful (not merely that C and Rust agree).
    let expect: Option<Vec<u8>> = if input.is_null() {
        None
    } else {
        let n = unsafe { strlen(input) };
        Some(unsafe { std::slice::from_raw_parts(input as *const u8, n + 1) }.to_vec())
    };
    let expect_len = expect.as_ref().map(|v| v.len() - 1);

    let c = unsafe { call_and_capture(c_impl(), input, expect_len) };
    let r = unsafe { call_and_capture(rust_impl(), input, expect_len) };

    assert_eq!(
        c.was_null, r.was_null,
        "[{ctx}] NULL-ness diverged: C returned {}, Rust returned {}",
        if c.was_null { "NULL" } else { "non-NULL" },
        if r.was_null { "NULL" } else { "non-NULL" },
    );

    if c.was_null {
        assert!(input.is_null(), "[{ctx}] both returned NULL for a non-NULL input");
        return None;
    }

    assert_eq!(
        c.bytes.len(),
        r.bytes.len(),
        "[{ctx}] copied length diverged: C {} vs Rust {}",
        c.bytes.len(),
        r.bytes.len()
    );
    if c.bytes != r.bytes {
        let at = c.bytes.iter().zip(r.bytes.iter()).position(|(a, b)| a != b).unwrap();
        panic!(
            "[{ctx}] copied bytes diverged at index {at}: C {:#04x} vs Rust {:#04x} (len {})",
            c.bytes[at], r.bytes[at], c.bytes.len()
        );
    }

    let expect = expect.unwrap();
    assert_eq!(c.bytes, expect, "[{ctx}] C copy is not a faithful strlen+1 copy of the input");

    // The C hands back fresh `malloc` memory; neither side may alias the input.
    assert_ne!(c.ptr, input as usize, "[{ctx}] C returned the input pointer");
    assert_ne!(r.ptr, input as usize, "[{ctx}] Rust returned the input pointer");

    Some(c.bytes)
}

/// Convenience: `assert_same` for a byte buffer that contains a NUL terminator
/// somewhere (bytes after the first NUL are deliberately allowed to be garbage —
/// see `CONFIGS.md` row 11).
pub fn assert_same_bytes(buf: &[u8], ctx: &str) -> Option<Vec<u8>> {
    debug_assert!(buf.contains(&0), "test buffer must contain a NUL terminator");
    assert_same(buf.as_ptr() as *const c_char, ctx)
}
