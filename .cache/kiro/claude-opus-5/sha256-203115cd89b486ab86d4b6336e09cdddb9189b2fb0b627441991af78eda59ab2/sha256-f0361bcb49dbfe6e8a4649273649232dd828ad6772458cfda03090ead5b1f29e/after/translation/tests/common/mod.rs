//! Shared differential-test harness.
//!
//! BOTH implementations are loaded as shared objects through `libloading` and
//! invoked purely through their exported `UTIL_createLinePointers` symbol. The
//! Rust function is never called directly, so the `#[no_mangle] extern "C"`
//! wrapper is part of what is under test.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::os::raw::c_char;
use std::path::PathBuf;
use std::sync::OnceLock;

pub type CreateFn = unsafe extern "C" fn(*mut c_char, usize, usize) -> *const *const c_char;

extern "C" {
    fn free(ptr: *mut std::ffi::c_void);
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("C_SO_PATH") {
        return PathBuf::from(p);
    }
    manifest_dir()
        .parent()
        .expect("crate has a parent dir")
        .join("c_src/build/libdriver.so")
}

fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_SO_PATH") {
        return PathBuf::from(p);
    }
    let root = manifest_dir();
    for profile in ["release", "debug"] {
        let cand = root.join("target").join(profile).join("libdriver.so");
        if cand.exists() {
            return cand;
        }
    }
    panic!(
        "Rust cdylib not found under {}/target/{{release,debug}}. \
         Run `cargo build --release` first.",
        root.display()
    );
}

/// Guard against a VACUOUS PASS.
///
/// `crate-type = ["cdylib"]` means integration tests never link the library, so
/// `cargo test` does NOT rebuild `libdriver.so`. Running `cargo test` after
/// editing `src/lib.rs` would otherwise silently test a stale `.so` and report
/// success. Refuse to run if the source is newer than the artifact.
fn assert_so_is_fresh(so: &PathBuf) {
    let src = manifest_dir().join("src/lib.rs");
    let mtime = |p: &PathBuf| {
        std::fs::metadata(p)
            .and_then(|m| m.modified())
            .unwrap_or_else(|e| panic!("cannot stat {}: {e}", p.display()))
    };
    if mtime(&src) > mtime(so) {
        panic!(
            "STALE ARTIFACT: {} is newer than {}.\n\
             `cargo test` does not rebuild a cdylib-only crate — run \
             `cargo build --release` (or ./verify.sh) first, otherwise these \
             tests would pass vacuously against an old library.",
            src.display(),
            so.display()
        );
    }
}

struct Libs {
    c: Library,
    rust: Library,
}

// Safety: libloading::Library is Send+Sync; the loaded symbols are plain
// stateless C functions.
static LIBS: OnceLock<Libs> = OnceLock::new();

fn libs() -> &'static Libs {
    LIBS.get_or_init(|| {
        let cp = c_so_path();
        let rp = rust_so_path();
        assert_so_is_fresh(&rp);
        let c = unsafe { Library::new(&cp) }
            .unwrap_or_else(|e| panic!("failed to dlopen C .so {}: {e}", cp.display()));
        let rust = unsafe { Library::new(&rp) }
            .unwrap_or_else(|e| panic!("failed to dlopen Rust .so {}: {e}", rp.display()));
        Libs { c, rust }
    })
}

pub fn c_fn() -> CreateFn {
    let s: Symbol<CreateFn> = unsafe { libs().c.get(b"UTIL_createLinePointers\0") }
        .expect("C .so does not export UTIL_createLinePointers");
    *s
}

pub fn rust_fn() -> CreateFn {
    let s: Symbol<CreateFn> = unsafe { libs().rust.get(b"UTIL_createLinePointers\0") }
        .expect("Rust .so does not export UTIL_createLinePointers");
    *s
}

/// Outcome of one call, normalised so that the two implementations can be
/// compared even though their heap addresses necessarily differ.
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Outcome {
    /// The function returned NULL.
    Null,
    /// The function returned a non-NULL array; the payload holds the
    /// `numLines` entries expressed as byte offsets from `buffer`.
    /// `None` entries mean the raw pointer was NULL (only possible if the C
    /// stored a NULL, e.g. `buffer == NULL`).
    Ok(Vec<Option<isize>>),
    /// Non-NULL return whose contents were intentionally not inspected
    /// (`numLines` too large to read safely).
    OkOpaque,
}

/// Read-back cap: above this we do not dereference the returned array.
const READBACK_CAP: usize = 1 << 20;

/// Invoke one implementation and normalise its result. Frees the returned
/// allocation with libc `free`, exactly as the C API contract requires.
unsafe fn invoke(f: CreateFn, buffer: *mut c_char, num_lines: usize, buffer_size: usize) -> Outcome {
    let ret = f(buffer, num_lines, buffer_size);
    if ret.is_null() {
        return Outcome::Null;
    }
    let outcome = if num_lines > READBACK_CAP {
        Outcome::OkOpaque
    } else {
        let mut v = Vec::with_capacity(num_lines);
        for i in 0..num_lines {
            let p = *ret.add(i);
            v.push(if p.is_null() {
                None
            } else {
                Some(p as isize - buffer as isize)
            });
        }
        Outcome::Ok(v)
    };
    free(ret as *mut std::ffi::c_void);
    outcome
}

/// Call BOTH `.so`s with the *same* buffer address and arguments and return
/// their normalised outcomes.
pub fn call_both(buf: &mut [u8], num_lines: usize, buffer_size: usize) -> (Outcome, Outcome) {
    let ptr = buf.as_mut_ptr() as *mut c_char;
    call_both_raw(ptr, num_lines, buffer_size)
}

pub fn call_both_raw(
    ptr: *mut c_char,
    num_lines: usize,
    buffer_size: usize,
) -> (Outcome, Outcome) {
    let c = unsafe { invoke(c_fn(), ptr, num_lines, buffer_size) };
    let r = unsafe { invoke(rust_fn(), ptr, num_lines, buffer_size) };
    (c, r)
}

/// Differential assertion with a descriptive context label.
#[track_caller]
pub fn assert_same(ctx: &str, buf: &mut [u8], num_lines: usize, buffer_size: usize) -> Outcome {
    let snapshot = buf.to_vec();
    let (c, r) = call_both(buf, num_lines, buffer_size);
    assert_eq!(
        c, r,
        "DIVERGENCE [{ctx}]: numLines={num_lines} bufferSize={buffer_size} \
         buffer={:?}\n  C   = {c:?}\n  RUST= {r:?}",
        Preview(&snapshot)
    );
    // Neither implementation may mutate the input buffer.
    assert_eq!(
        &snapshot[..],
        &buf[..],
        "input buffer was mutated [{ctx}]: numLines={num_lines} bufferSize={buffer_size}"
    );
    c
}

#[track_caller]
pub fn assert_same_raw(ctx: &str, ptr: *mut c_char, num_lines: usize, buffer_size: usize) -> Outcome {
    let (c, r) = call_both_raw(ptr, num_lines, buffer_size);
    assert_eq!(
        c, r,
        "DIVERGENCE [{ctx}]: ptr={ptr:?} numLines={num_lines} bufferSize={buffer_size}\n  \
         C   = {c:?}\n  RUST= {r:?}"
    );
    c
}

/// Compact buffer preview for failure messages.
pub struct Preview<'a>(pub &'a [u8]);

impl std::fmt::Debug for Preview<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let n = self.0.len().min(64);
        write!(f, "[len={}]", self.0.len())?;
        for b in &self.0[..n] {
            match b {
                0 => write!(f, "\\0")?,
                0x20..=0x7e => write!(f, "{}", *b as char)?,
                _ => write!(f, "\\x{b:02x}")?,
            }
        }
        if n < self.0.len() {
            write!(f, "...")?;
        }
        Ok(())
    }
}

/// Deterministic xorshift64* PRNG — fixed seed per test for reproducibility.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(if seed == 0 { 0x9E3779B97F4A7C15 } else { seed })
    }
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545F4914F6CDD1D)
    }
    pub fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next_u64() % n as u64) as usize
        }
    }
    pub fn range(&mut self, lo: usize, hi_inclusive: usize) -> usize {
        lo + self.below(hi_inclusive - lo + 1)
    }
    pub fn byte(&mut self) -> u8 {
        (self.next_u64() >> 32) as u8
    }
    /// Non-zero byte.
    pub fn nz_byte(&mut self) -> u8 {
        loop {
            let b = self.byte();
            if b != 0 {
                return b;
            }
        }
    }
    pub fn bool(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }
}

/// Reference model of the C scanning loop: how many entries would be written,
/// and at which offsets, for a given `bufferSize` and content. Used to build
/// inputs whose segment count is known exactly.
pub fn scan_offsets(buf: &[u8], buffer_size: usize) -> Vec<usize> {
    let mut offsets = Vec::new();
    let mut pos = 0usize;
    while pos < buffer_size {
        offsets.push(pos);
        let mut len = 0usize;
        while pos + len < buffer_size && buf[pos + len] != 0 {
            len += 1;
        }
        pos += len;
        if pos < buffer_size {
            pos += 1;
        }
    }
    offsets
}

/// Build a buffer of `count` NUL-terminated random segments.
/// Returns (bytes, expected offsets).
pub fn build_segments(
    rng: &mut Rng,
    count: usize,
    max_seg_len: usize,
    terminate_last: bool,
) -> (Vec<u8>, Vec<usize>) {
    let mut bytes = Vec::new();
    let mut offsets = Vec::new();
    for i in 0..count {
        offsets.push(bytes.len());
        let last = i + 1 == count;
        // An unterminated FINAL segment must be non-empty, otherwise it is
        // indistinguishable from "no segment at all".
        let min_len = if last && !terminate_last { 1 } else { 0 };
        let len = rng.range(min_len, max_seg_len.max(min_len));
        for _ in 0..len {
            bytes.push(rng.nz_byte());
        }
        if !last || terminate_last {
            bytes.push(0);
        }
    }
    debug_assert_eq!(offsets, scan_offsets(&bytes, bytes.len()));
    (bytes, offsets)
}
