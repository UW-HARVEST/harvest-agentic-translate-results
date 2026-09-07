//! Shared harness: loads BOTH the C `.so` and the Rust `.so` through
//! `libloading` and calls `UTIL_createLinePointers` across the FFI boundary.
//!
//! Nothing in the Rust crate is ever called directly — the exported symbol of
//! the compiled `cdylib` is used, exactly as an external C consumer would.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::os::raw::c_char;
use std::path::PathBuf;
use std::sync::OnceLock;

pub type CreateLinePointersFn =
    unsafe extern "C" fn(*mut c_char, usize, usize) -> *const *const c_char;

extern "C" {
    fn free(ptr: *mut std::os::raw::c_void);
}

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn workspace_root() -> PathBuf {
    crate_root().parent().unwrap().to_path_buf()
}

fn c_lib_path() -> PathBuf {
    workspace_root().join("c_src/build/libdriver.so")
}

fn rust_lib_path() -> PathBuf {
    // The cdylib produced by this crate. `cargo test` builds it alongside the
    // test binary, so it lives in the same profile dir as the test executable
    // (`target/<profile>/deps/<test>` -> `target/<profile>/libdriver.so`).
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(exe) = std::env::current_exe() {
        if let Some(deps) = exe.parent() {
            candidates.push(deps.join("libdriver.so"));
            if let Some(profile) = deps.parent() {
                candidates.push(profile.join("libdriver.so"));
            }
        }
    }
    // NOTE: deliberately NO cross-profile fallback. `cargo test` does not build
    // the `cdylib` artifact, so falling back to another profile's `.so` would
    // silently verify the wrong binary. `run_all_combos.sh` builds the library
    // for the profile under test before invoking `cargo test`.
    for c in candidates.iter() {
        if c.exists() {
            return c.clone();
        }
    }
    let _ = crate_root();
    panic!(
        "Rust cdylib not found for this profile; build it first with \
         `cargo build [--release]` (or use ./run_all_combos.sh). Looked in: {candidates:?}"
    );
}

pub struct Libs {
    _c: Library,
    _rust: Library,
    pub c_fn: CreateLinePointersFn,
    pub rust_fn: CreateLinePointersFn,
}

static LIBS: OnceLock<Libs> = OnceLock::new();

pub fn libs() -> &'static Libs {
    LIBS.get_or_init(|| unsafe {
        let c = Library::new(c_lib_path())
            .unwrap_or_else(|e| panic!("failed to load {:?}: {e}", c_lib_path()));
        let rust = Library::new(rust_lib_path())
            .unwrap_or_else(|e| panic!("failed to load {:?}: {e}", rust_lib_path()));

        let c_sym: Symbol<CreateLinePointersFn> = c
            .get(b"UTIL_createLinePointers\0")
            .expect("C .so is missing UTIL_createLinePointers");
        let rust_sym: Symbol<CreateLinePointersFn> = rust
            .get(b"UTIL_createLinePointers\0")
            .expect("Rust .so is missing UTIL_createLinePointers");

        let c_fn = *c_sym;
        let rust_fn = *rust_sym;

        Libs {
            _c: c,
            _rust: rust,
            c_fn,
            rust_fn,
        }
    })
}

/// The observable output of one call: whether NULL was returned and, if not,
/// the offset of each returned line pointer relative to the input buffer.
///
/// Raw pointer *values* of the returned heap block are allocation-dependent, so
/// the byte-identical comparison is done on the semantic content: the
/// `numLines` pointers, each expressed as `ptr - buffer` (which is exactly what
/// the C computes: `buffer + pos`).
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Outcome {
    Null,
    Ok(Vec<isize>),
}

/// Call one implementation on `buffer` and normalize its result.
///
/// `buffer` is passed by raw pointer to BOTH implementations so the returned
/// `buffer + pos` values are directly comparable. The returned block is freed
/// with libc `free`, matching the C library's contract.
unsafe fn call_one(f: CreateLinePointersFn, buf: *mut c_char, num_lines: usize, size: usize) -> Outcome {
    let ret = f(buf, num_lines, size);
    if ret.is_null() {
        return Outcome::Null;
    }
    let mut offsets = Vec::with_capacity(num_lines);
    for i in 0..num_lines {
        let p = *ret.add(i);
        offsets.push((p as isize) - (buf as isize));
    }
    free(ret as *mut std::os::raw::c_void);
    Outcome::Ok(offsets)
}

/// Run the same call against both `.so`s and assert identical results.
pub fn assert_same(buffer: &mut [u8], num_lines: usize, buffer_size: usize, ctx: &str) {
    let l = libs();
    let ptr = buffer.as_mut_ptr() as *mut c_char;
    let (c_out, r_out) = unsafe {
        // C first, then Rust, on the very same memory. The function never
        // mutates the buffer, so ordering is irrelevant.
        let a = call_one(l.c_fn, ptr, num_lines, buffer_size);
        let b = call_one(l.rust_fn, ptr, num_lines, buffer_size);
        (a, b)
    };
    assert_eq!(
        c_out, r_out,
        "divergence [{ctx}]: numLines={num_lines} bufferSize={buffer_size} buffer={:?}",
        &buffer[..buffer.len().min(80)]
    );
}

/// Same as [`assert_same`] but for a raw (possibly NULL) buffer pointer.
pub fn assert_same_raw(ptr: *mut c_char, num_lines: usize, buffer_size: usize, ctx: &str) {
    let l = libs();
    let (c_out, r_out) = unsafe {
        let a = call_one(l.c_fn, ptr, num_lines, buffer_size);
        let b = call_one(l.rust_fn, ptr, num_lines, buffer_size);
        (a, b)
    };
    assert_eq!(
        c_out, r_out,
        "divergence [{ctx}]: ptr={ptr:?} numLines={num_lines} bufferSize={buffer_size}"
    );
}

/// Deterministic xorshift64* PRNG so every test run is reproducible.
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
    /// Uniform in `[0, n)`; returns 0 when `n == 0`.
    pub fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next_u64() % (n as u64)) as usize
        }
    }
    pub fn range(&mut self, lo: usize, hi_inclusive: usize) -> usize {
        lo + self.below(hi_inclusive - lo + 1)
    }
    /// A byte that is never `'\0'`.
    pub fn nonzero_byte(&mut self) -> u8 {
        1u8.wrapping_add((self.next_u64() % 255) as u8)
    }
    pub fn any_byte(&mut self) -> u8 {
        (self.next_u64() & 0xFF) as u8
    }
}

pub const SEED: u64 = 0x5EED_1234;
pub const ITERS: usize = 200;

/// Build a NUL-separated buffer from `lines` (each line gets a terminator).
pub fn join_terminated(lines: &[Vec<u8>]) -> Vec<u8> {
    let mut v = Vec::new();
    for l in lines {
        v.extend_from_slice(l);
        v.push(0);
    }
    v
}
