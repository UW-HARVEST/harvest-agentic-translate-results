//! Shared differential-test harness.
//!
//! Both the C `libdriver.so` and the Rust `libdriver.so` are loaded with
//! `libloading` and called *only* through their exported `custom_strdup`
//! symbol. The Rust implementation is never called directly, so the
//! `#[no_mangle] extern "C"` wrapper is part of what is under test.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::c_char;
use std::path::PathBuf;
use std::sync::OnceLock;

pub type StrdupFn = unsafe extern "C" fn(*const c_char) -> *mut c_char;

/// Path to the C shared library built by `c_src/CMakeLists.txt`.
pub fn c_so_path() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.pop();
    p.push("c_src/build/libdriver.so");
    p
}

/// Path to the Rust `cdylib`. Prefers the profile directory this test binary
/// was built into, then falls back to the other profile — cargo does not build
/// a `cdylib`-only crate as a side effect of `cargo test`, so the runner script
/// invokes `cargo build` first.
pub fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("DRIVER_RUST_SO") {
        return PathBuf::from(p);
    }
    // .../target/<profile>/deps/<test-bin>  ->  .../target/<profile>/libdriver.so
    let exe = std::env::current_exe().expect("current_exe");
    let profile_dir = exe
        .parent()
        .and_then(|deps| deps.parent())
        .expect("target/<profile> dir")
        .to_path_buf();

    let mut candidates = vec![profile_dir.join("libdriver.so")];
    if let Some(target_dir) = profile_dir.parent() {
        candidates.push(target_dir.join("release/libdriver.so"));
        candidates.push(target_dir.join("debug/libdriver.so"));
    }
    for c in &candidates {
        if c.exists() {
            return c.clone();
        }
    }
    panic!(
        "Rust cdylib not found. Run `cargo build` (and/or `cargo build --release`) first. \
         Looked in: {candidates:?}"
    );
}

fn load(path: &PathBuf) -> StrdupFn {
    assert!(
        path.exists(),
        "shared library not found: {} — build it first",
        path.display()
    );
    unsafe {
        // Leak the `Library` so the extracted symbol is valid for `'static`.
        let lib: &'static Library = Box::leak(Box::new(
            Library::new(path).unwrap_or_else(|e| panic!("dlopen {}: {e}", path.display())),
        ));
        let sym: Symbol<StrdupFn> = lib
            .get(b"custom_strdup\0")
            .unwrap_or_else(|e| panic!("dlsym custom_strdup in {}: {e}", path.display()));
        *sym
    }
}

pub struct Impls {
    pub c: StrdupFn,
    pub rust: StrdupFn,
}

pub fn impls() -> &'static Impls {
    static IMPLS: OnceLock<Impls> = OnceLock::new();
    IMPLS.get_or_init(|| Impls {
        c: load(&c_so_path()),
        rust: load(&rust_so_path()),
    })
}

/// Short, printable rendition of an input buffer for assertion messages.
pub fn describe(input: &[u8]) -> String {
    let head: Vec<String> = input.iter().take(24).map(|b| format!("{b:02x}")).collect();
    format!(
        "len_with_nul={} bytes=[{}{}]",
        input.len(),
        head.join(" "),
        if input.len() > 24 { " ..." } else { "" }
    )
}

/// Call both implementations on `input` (which MUST be NUL-terminated) and
/// assert the results are byte-identical, including the terminator.
///
/// Also checks the ownership contract the C establishes: a fresh `malloc`
/// block, distinct from the input, releasable with `free`.
pub fn assert_same(input: &[u8]) {
    assert_eq!(
        input.last().copied(),
        Some(0u8),
        "test bug: input must be NUL-terminated"
    );
    let i = impls();
    let ptr = input.as_ptr() as *const c_char;
    assert_same_ptr(i, ptr, Some(input));
}

/// Lower-level form: call both on a raw pointer. `expected` is the buffer the
/// pointer refers to when the caller can supply it (used for content checks).
pub fn assert_same_ptr(i: &Impls, ptr: *const c_char, expected: Option<&[u8]>) {
    let c_res = unsafe { (i.c)(ptr) };
    let r_res = unsafe { (i.rust)(ptr) };

    let desc = expected.map(describe).unwrap_or_else(|| format!("ptr={ptr:p}"));

    assert_eq!(
        c_res.is_null(),
        r_res.is_null(),
        "null-ness mismatch: C={:?} Rust={:?} for {desc}",
        c_res,
        r_res
    );

    if c_res.is_null() {
        return;
    }

    // Neither implementation may return the input pointer itself.
    assert_ne!(c_res as *const c_char, ptr, "C returned the input pointer");
    assert_ne!(r_res as *const c_char, ptr, "Rust returned the input pointer");
    assert_ne!(c_res, r_res, "both returned the same pointer");

    // Compare using the length each implementation actually produced, so a
    // wrong `+1` or a wrong copy length is caught rather than masked.
    let c_len = unsafe { libc::strlen(c_res) };
    let r_len = unsafe { libc::strlen(r_res) };
    assert_eq!(c_len, r_len, "strlen(result) mismatch for {desc}");

    let c_bytes = unsafe { std::slice::from_raw_parts(c_res as *const u8, c_len + 1) };
    let r_bytes = unsafe { std::slice::from_raw_parts(r_res as *const u8, r_len + 1) };
    assert_eq!(
        c_bytes, r_bytes,
        "result bytes (incl. NUL) differ for {desc}"
    );

    if let Some(exp) = expected {
        assert_eq!(c_bytes, exp, "C result does not equal the input for {desc}");
        assert_eq!(r_bytes, exp, "Rust result does not equal the input for {desc}");
    }

    // The C hands back a `malloc` block; both results must survive `free`.
    unsafe {
        libc::free(c_res as *mut libc::c_void);
        libc::free(r_res as *mut libc::c_void);
    }
}

/// Deterministic xorshift64* PRNG — no external rand dependency, fixed seed so
/// every run examines the same inputs.
pub struct Rng(u64);

pub const SEED: u64 = 0x2024_0601_C0FF_EE01;

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(if seed == 0 { 0x9E37_79B9_7F4A_7C15 } else { seed })
    }
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    /// Uniform-ish value in `0..n`.
    pub fn below(&mut self, n: u64) -> u64 {
        assert!(n > 0);
        self.next_u64() % n
    }
    /// A byte legal inside a C string: never `0`.
    pub fn nonzero_byte(&mut self) -> u8 {
        1 + (self.next_u64() % 255) as u8
    }
    /// NUL-terminated buffer whose payload is `payload_len` random non-zero bytes.
    pub fn cstring(&mut self, payload_len: usize) -> Vec<u8> {
        let mut v: Vec<u8> = (0..payload_len).map(|_| self.nonzero_byte()).collect();
        v.push(0);
        v
    }
}
