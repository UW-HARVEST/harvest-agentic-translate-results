//! Shared differential-test harness.
//!
//! Both the C `.so` and the Rust `.so` are loaded with `libloading` and called
//! only through their exported `encode_base64` symbol. The Rust function is
//! NEVER called directly, so the `#[no_mangle]`/`extern "C"` export wrapper is
//! part of what is under test.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, c_void};
use std::path::PathBuf;

unsafe extern "C" {
    fn free(p: *mut c_void);
}

/// `char *encode_base64(int size, const char *src)`
pub type EncodeBase64 = unsafe extern "C" fn(c_int, *const c_char) -> *mut c_char;

pub struct Libs {
    c: Library,
    rust: Library,
    pub c_path: PathBuf,
    pub rust_path: PathBuf,
}

/// Locate the Rust cdylib next to the running test executable
/// (`target/<profile>/deps/<test>` -> `target/<profile>/libdriver.so`).
fn rust_so_path() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    let deps = exe.parent().expect("deps dir");
    let profile = deps.parent().expect("profile dir");
    let candidates = [
        profile.join("libdriver.so"),
        deps.join("libdriver.so"),
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/release/libdriver.so"),
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/debug/libdriver.so"),
    ];
    for c in &candidates {
        if c.exists() {
            return c.clone();
        }
    }
    panic!(
        "Rust cdylib libdriver.so not found. Looked in: {:#?}\n\
         Build it first with `cargo build` / `cargo build --release`.",
        candidates
    );
}

fn c_so_path() -> PathBuf {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace parent")
        .to_path_buf();
    let candidates = [
        root.join("c_src/build/libdriver.so"),
        root.join("c_src/build/lib/libdriver.so"),
    ];
    for c in &candidates {
        if c.exists() {
            return c.clone();
        }
    }
    panic!(
        "C libdriver.so not found. Looked in: {:#?}\n\
         Build it with:\n  cd c_src && mkdir -p build && cd build \\\n\
         \x20   && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        candidates
    );
}

impl Libs {
    pub fn load() -> Self {
        let c_path = c_so_path();
        let rust_path = rust_so_path();
        // Safety: both objects are plain C-ABI libraries with no initializers
        // that could misbehave.
        let c = unsafe { Library::new(&c_path) }.expect("load C .so");
        let rust = unsafe { Library::new(&rust_path) }.expect("load Rust .so");
        Libs {
            c,
            rust,
            c_path,
            rust_path,
        }
    }

    pub fn c_encode(&self) -> Symbol<'_, EncodeBase64> {
        unsafe { self.c.get(b"encode_base64\0") }.expect("C encode_base64 symbol")
    }

    pub fn rust_encode(&self) -> Symbol<'_, EncodeBase64> {
        unsafe { self.rust.get(b"encode_base64\0") }
            .expect("Rust encode_base64 symbol (missing #[no_mangle] export?)")
    }
}

/* ------------------------------------------------------------------ */
/* Faithful models of the C size arithmetic (used only to decide how   */
/* many bytes are legal to compare, never to compute expected output). */
/* ------------------------------------------------------------------ */

/// `n = size * 4 / 3 + 4`, in wrapping `int` arithmetic, exactly as `lib.c:41`.
pub fn n_bytes(effective_size: c_int) -> c_int {
    effective_size
        .wrapping_mul(4)
        .wrapping_div(3)
        .wrapping_add(4)
}

/// The `size` the C code actually uses: `if (!size) size = strlen(src);`
pub fn effective_size(size: c_int, buf: &[u8]) -> c_int {
    if size == 0 {
        let len = buf.iter().position(|&b| b == 0).unwrap_or_else(|| {
            panic!("strlen-mode input buffer must be NUL-terminated: {:?}", buf)
        });
        len as c_int
    } else {
        size
    }
}

/// Number of bytes of the returned allocation that are safe to read: the whole
/// `calloc(1, n)` region, so both the emitted base64 bytes *and* the zero
/// padding are compared.
pub fn comparable_len(effective_size: c_int) -> usize {
    let n = n_bytes(effective_size);
    if n > 0 { n as usize } else { 0 }
}

/// Guard against invoking documented-UB inputs that would segfault the C `.so`
/// and take the whole test process down with it.
///
/// The read loop runs iff `effective_size > 0`. It reads `src[0..size]` and
/// writes `4*ceil(size/3)` bytes into an `n`-byte buffer. So a call is
/// well-defined iff the pointer is NULL, or `effective_size <= 0`, or `calloc`
/// fails before the loop (`n <= 0`), or the caller really did supply
/// `effective_size` readable bytes.
pub fn is_well_defined(size: c_int, buf_len: usize, effective_size: c_int) -> bool {
    if effective_size <= 0 {
        // Loop never runs. (`size == 0` additionally requires a NUL-terminated
        // buffer, which `effective_size()` already asserted.)
        let _ = size;
        return true;
    }
    if n_bytes(effective_size) <= 0 {
        return true; // calloc fails -> early NULL return, loop unreachable
    }
    (effective_size as usize) <= buf_len
}

/* ------------------------------------------------------------------ */
/* The differential comparison                                         */
/* ------------------------------------------------------------------ */

pub struct Differ<'a> {
    c: Symbol<'a, EncodeBase64>,
    rust: Symbol<'a, EncodeBase64>,
    pub calls: std::cell::Cell<u64>,
}

impl<'a> Differ<'a> {
    pub fn new(libs: &'a Libs) -> Self {
        Differ {
            c: libs.c_encode(),
            rust: libs.rust_encode(),
            calls: std::cell::Cell::new(0),
        }
    }

    /// Call both `.so`s with `(size, buf.as_ptr())` and assert the results are
    /// byte-identical over the whole allocated region. `ctx` names the
    /// CONFIGS.md / ERRORS.md row for failure messages.
    pub fn assert_same(&self, ctx: &str, size: c_int, buf: &[u8]) {
        let eff = effective_size(size, buf);
        assert!(
            is_well_defined(size, buf.len(), eff),
            "{ctx}: refusing to invoke C undefined behaviour \
             (size={size}, effective={eff}, buf_len={})",
            buf.len()
        );
        self.assert_same_raw(ctx, size, buf.as_ptr() as *const c_char, eff);
    }

    /// Null-pointer variant (there is no buffer at all).
    pub fn assert_same_null(&self, ctx: &str, size: c_int) {
        self.assert_same_raw(ctx, size, std::ptr::null(), 0);
    }

    /// Raw single call into the C `.so`. The caller owns the returned pointer.
    /// Only used where the expected answer is the `NULL` sentinel, so that the
    /// test can assert on `NULL` itself rather than merely on C/Rust equality.
    pub unsafe fn call_c(&self, size: c_int, src: *const c_char) -> *mut c_char {
        unsafe { (self.c)(size, src) }
    }

    /// Raw single call into the Rust `.so` (via its exported symbol).
    pub unsafe fn call_rust(&self, size: c_int, src: *const c_char) -> *mut c_char {
        unsafe { (self.rust)(size, src) }
    }

    fn assert_same_raw(&self, ctx: &str, size: c_int, src: *const c_char, eff: c_int) {
        self.calls.set(self.calls.get() + 1);

        let cp = unsafe { (self.c)(size, src) };
        let rp = unsafe { (self.rust)(size, src) };

        assert_eq!(
            cp.is_null(),
            rp.is_null(),
            "{ctx}: NULL-ness diverged for size={size} \
             (C {}, Rust {})",
            if cp.is_null() { "NULL" } else { "non-NULL" },
            if rp.is_null() { "NULL" } else { "non-NULL" },
        );

        if cp.is_null() {
            return; // nothing allocated on either side
        }

        let len = if src.is_null() {
            0
        } else {
            comparable_len(eff)
        };

        let cs = unsafe { std::slice::from_raw_parts(cp as *const u8, len) };
        let rs = unsafe { std::slice::from_raw_parts(rp as *const u8, len) };

        if cs != rs {
            let at = cs
                .iter()
                .zip(rs.iter())
                .position(|(a, b)| a != b)
                .unwrap_or(0);
            let lo = at.saturating_sub(8);
            let hi = (at + 8).min(len);
            let msg = format!(
                "{ctx}: output diverged for size={size} (effective={eff}, n={len})\n  \
                 first difference at byte {at}: C=0x{:02x} Rust=0x{:02x}\n  \
                 C   [{lo}..{hi}] = {:?}\n  Rust[{lo}..{hi}] = {:?}",
                cs[at],
                rs[at],
                String::from_utf8_lossy(&cs[lo..hi]),
                String::from_utf8_lossy(&rs[lo..hi]),
            );
            unsafe {
                free(cp as *mut c_void);
                free(rp as *mut c_void);
            }
            panic!("{msg}");
        }

        unsafe {
            free(cp as *mut c_void);
            free(rp as *mut c_void);
        }
    }

    /// Return the C `.so`'s output as an owned byte vector (harness liveness
    /// checks only -- the C side is ground truth).
    pub fn c_output(&self, size: c_int, buf: &[u8]) -> Option<Vec<u8>> {
        let eff = effective_size(size, buf);
        let p = unsafe { (self.c)(size, buf.as_ptr() as *const c_char) };
        if p.is_null() {
            return None;
        }
        let len = comparable_len(eff);
        let v = unsafe { std::slice::from_raw_parts(p as *const u8, len) }.to_vec();
        unsafe { free(p as *mut c_void) };
        Some(v)
    }

    /// Same, from the Rust `.so`.
    pub fn rust_output(&self, size: c_int, buf: &[u8]) -> Option<Vec<u8>> {
        let eff = effective_size(size, buf);
        let p = unsafe { (self.rust)(size, buf.as_ptr() as *const c_char) };
        if p.is_null() {
            return None;
        }
        let len = comparable_len(eff);
        let v = unsafe { std::slice::from_raw_parts(p as *const u8, len) }.to_vec();
        unsafe { free(p as *mut c_void) };
        Some(v)
    }
}

/* ------------------------------------------------------------------ */
/* Deterministic PRNG (xorshift64*), fixed seed for reproducibility.   */
/* ------------------------------------------------------------------ */

pub const SEED: u64 = 0x243F_6A88_85A3_08D3;

pub struct Rng(u64);

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
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    /// Uniform-ish value in `0..n` (n > 0).
    pub fn below(&mut self, n: u32) -> u32 {
        self.next_u32() % n
    }
    /// Inclusive range.
    pub fn range(&mut self, lo: u32, hi: u32) -> u32 {
        lo + self.below(hi - lo + 1)
    }
    pub fn byte(&mut self) -> u8 {
        (self.next_u64() >> 24) as u8
    }
    pub fn bytes(&mut self, n: usize) -> Vec<u8> {
        (0..n).map(|_| self.byte()).collect()
    }
    /// Random bytes drawn from an inclusive value range (`lo..=hi`).
    pub fn bytes_in(&mut self, n: usize, lo: u8, hi: u8) -> Vec<u8> {
        assert!(lo <= hi);
        let span = (hi as u16) - (lo as u16) + 1; // avoids u8 overflow at 0..=255
        (0..n)
            .map(|_| (lo as u16 + (self.byte() as u16) % span) as u8)
            .collect()
    }
}
