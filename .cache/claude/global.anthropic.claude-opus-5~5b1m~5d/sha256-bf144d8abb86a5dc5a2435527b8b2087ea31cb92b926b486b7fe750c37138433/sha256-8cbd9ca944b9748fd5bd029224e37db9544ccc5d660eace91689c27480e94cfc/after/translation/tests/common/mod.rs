#![allow(dead_code)]
//! Shared harness: loads BOTH the C `.so` and the Rust `.so` through
//! `libloading` and calls `tool_basename` only through its exported symbol, so
//! the `#[no_mangle] extern "C"` wrapper is under test too.

use libloading::{Library, Symbol};
use std::ffi::c_char;
use std::path::PathBuf;
use std::sync::OnceLock;

pub type ToolBasename = unsafe extern "C" fn(*mut c_char) -> *mut c_char;

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Path to the C shared object, built by
/// `cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .`
pub fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("DRIVER_C_SO") {
        return PathBuf::from(p);
    }
    let p = crate_root().join("../c_src/build/libdriver.so");
    assert!(
        p.exists(),
        "C shared library not found at {}. Build it first:\n  \
         cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        p.display()
    );
    p
}

/// Path to the Rust shared object produced by this crate's `cdylib` target.
pub fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("DRIVER_RUST_SO") {
        return PathBuf::from(p);
    }
    let root = crate_root();
    // Prefer whichever profile actually produced an artifact; when both exist
    // take the newer one so a fresh `cargo build` is picked up.
    let mut best: Option<(PathBuf, std::time::SystemTime)> = None;
    for profile in ["debug", "release"] {
        let p = root.join("target").join(profile).join("libdriver.so");
        if let Ok(md) = std::fs::metadata(&p) {
            let t = md.modified().unwrap_or(std::time::UNIX_EPOCH);
            if best.as_ref().map(|(_, bt)| t > *bt).unwrap_or(true) {
                best = Some((p, t));
            }
        }
    }
    best.map(|(p, _)| p).unwrap_or_else(|| {
        panic!(
            "Rust cdylib not found under {}/target/{{debug,release}}/libdriver.so; \
             run `cargo build` / `cargo build --release` first",
            root.display()
        )
    })
}

pub struct Libs {
    _c: Library,
    _rust: Library,
    pub c: ToolBasename,
    pub rust: ToolBasename,
}

// The two `Library` handles are kept alive for the process lifetime, and the
// function under test is pure w.r.t. shared state, so sharing across the test
// threads is sound.
unsafe impl Send for Libs {}
unsafe impl Sync for Libs {}

pub fn load_one(path: &std::path::Path) -> (Library, ToolBasename) {
    let lib = unsafe { Library::new(path) }
        .unwrap_or_else(|e| panic!("failed to dlopen {}: {e}", path.display()));
    let f = unsafe {
        let s: Symbol<ToolBasename> = lib
            .get(b"tool_basename\0")
            .unwrap_or_else(|e| panic!("`tool_basename` missing from {}: {e}", path.display()));
        *s
    };
    (lib, f)
}

pub fn libs() -> &'static Libs {
    static LIBS: OnceLock<Libs> = OnceLock::new();
    LIBS.get_or_init(|| {
        let (lc, c) = load_one(&c_so_path());
        let (lr, rust) = load_one(&rust_so_path());
        Libs {
            _c: lc,
            _rust: lr,
            c,
            rust,
        }
    })
}

/// Result of one call, expressed in a way that is comparable between the two
/// libraries: the returned pointer as a byte OFFSET into the buffer that was
/// handed to it, plus the bytes the pointer designates.
#[derive(Debug, PartialEq, Eq)]
pub struct CallResult {
    pub offset: isize,
    pub tail: Vec<u8>,
}

/// Call one implementation on `buf` (which must already contain a NUL) and
/// describe the returned pointer relative to `buf`.
fn call_one(f: ToolBasename, buf: &mut [u8]) -> CallResult {
    let base = buf.as_mut_ptr() as *mut c_char;
    let ret = unsafe { f(base) };
    assert!(!ret.is_null(), "tool_basename returned NULL");
    let offset = (ret as isize) - (base as isize);
    let tail = unsafe { std::ffi::CStr::from_ptr(ret) }.to_bytes().to_vec();
    CallResult { offset, tail }
}

/// Build a NUL-terminated buffer from `bytes` (which must not contain a NUL
/// unless the test is deliberately probing interior NULs — use
/// [`compare_raw`] for that).
pub fn nul_terminated(bytes: &[u8]) -> Vec<u8> {
    let mut v = Vec::with_capacity(bytes.len() + 1);
    v.extend_from_slice(bytes);
    v.push(0);
    v
}

/// Differential call: hand the SAME buffer to the C and to the Rust
/// implementation and assert the two returned pointers are byte-identical.
/// `raw` must be NUL-terminated already.
pub fn compare_raw(raw: &[u8], ctx: &str) -> CallResult {
    let l = libs();
    let mut buf_c = raw.to_vec();
    let mut buf_r = raw.to_vec();

    let c = call_one(l.c, &mut buf_c);
    let r = call_one(l.rust, &mut buf_r);

    assert_eq!(
        c, r,
        "DIVERGENCE [{ctx}] input={:?}\n  C   -> offset {} tail {:?}\n  Rust-> offset {} tail {:?}",
        Bytes(raw),
        c.offset,
        Bytes(&c.tail),
        r.offset,
        Bytes(&r.tail)
    );

    // The input must not have been mutated by either implementation.
    assert_eq!(buf_c, raw, "C mutated its input buffer [{ctx}]");
    assert_eq!(buf_r, raw, "Rust mutated its input buffer [{ctx}]");

    // The returned pointer must alias into the caller's buffer (no copy).
    let strlen = raw.iter().position(|&b| b == 0).unwrap_or(raw.len()) as isize;
    assert!(
        (0..=strlen).contains(&c.offset),
        "returned pointer outside the input string [{ctx}]: offset {} strlen {}",
        c.offset,
        strlen
    );

    c
}

/// Convenience wrapper: appends the NUL for you.
pub fn compare(bytes: &[u8], ctx: &str) -> CallResult {
    compare_raw(&nul_terminated(bytes), ctx)
}

/// Pretty-printer that keeps non-UTF-8 bytes readable in failure messages.
pub struct Bytes<'a>(pub &'a [u8]);
impl std::fmt::Debug for Bytes<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("b\"")?;
        for &b in self.0 {
            match b {
                b'\\' => f.write_str("\\\\")?,
                b'"' => f.write_str("\\\"")?,
                0x20..=0x7e => f.write_str(unsafe { std::str::from_utf8_unchecked(&[b]) })?,
                _ => write!(f, "\\x{b:02x}")?,
            }
        }
        f.write_str("\"")
    }
}

/// Deterministic xorshift64* PRNG so every property-style row is reproducible.
pub struct Rng(u64);

impl Rng {
    pub const SEED: u64 = 0x5EED_1234;

    pub fn new(seed: u64) -> Self {
        Rng(if seed == 0 { 1 } else { seed })
    }
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    /// Uniform-ish value in `0..n` (n > 0).
    pub fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % (n as u64)) as usize
    }
    pub fn range(&mut self, lo: usize, hi_inclusive: usize) -> usize {
        lo + self.below(hi_inclusive - lo + 1)
    }
    pub fn pick<T: Copy>(&mut self, xs: &[T]) -> T {
        xs[self.below(xs.len())]
    }
}

/// Bytes that are never a path separator: includes the ASCII near-misses of
/// `'/'` (0x2f) and `'\\'` (0x5c), and high-bit / non-UTF-8 bytes.
pub const NON_SEP: &[u8] = &[
    b'a', b'b', b'Z', b'.', b'0', b'[', b']', b':', b'-', b'_', b' ', 0x2e, 0x30, 0x5b, 0x5d, 0x80,
    0xc3, 0xff, 0xfe, 0x7f, 0x01,
];

/// Random string of `len` bytes drawn from `NON_SEP` (never a separator, never
/// a NUL).
pub fn rand_non_sep(rng: &mut Rng, len: usize) -> Vec<u8> {
    (0..len).map(|_| rng.pick(NON_SEP)).collect()
}
