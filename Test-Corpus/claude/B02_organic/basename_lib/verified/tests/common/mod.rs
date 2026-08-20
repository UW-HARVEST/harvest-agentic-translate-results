//! Shared differential-test harness.
//!
//! Both implementations are loaded as *shared libraries* with `libloading` and
//! called only through their exported C ABI symbols — the Rust functions are
//! never called directly, so the `#[no_mangle] extern "C"` wrappers are part of
//! what is under test.

#![allow(dead_code)]

use std::ffi::c_char;
use std::fmt;
use std::path::PathBuf;
use std::sync::OnceLock;

/// `char *tool_basename(char *path)`
pub type ToolBasenameFn = unsafe extern "C" fn(*mut c_char) -> *mut c_char;

pub struct Impl {
    pub name: &'static str,
    pub path: PathBuf,
    // Keep the library alive for the whole process; the raw fn pointer below
    // borrows from it.
    _lib: libloading::Library,
    pub tool_basename: ToolBasenameFn,
}

impl Impl {
    fn load(name: &'static str, path: PathBuf) -> Impl {
        unsafe {
            let lib = libloading::Library::new(&path).unwrap_or_else(|e| {
                panic!("failed to dlopen {} ({}): {e}", path.display(), name)
            });
            let sym: libloading::Symbol<ToolBasenameFn> = lib
                .get(b"tool_basename\0")
                .unwrap_or_else(|e| panic!("{} ({}) has no `tool_basename`: {e}", path.display(), name));
            let f = *sym;
            Impl { name, path, _lib: lib, tool_basename: f }
        }
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("C_DRIVER_SO") {
        return PathBuf::from(p);
    }
    let p = manifest_dir().join("c_src/build/libdriver.so");
    assert!(
        p.exists(),
        "C shared library not found at {} — build it with:\n  cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        p.display()
    );
    p
}

fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_DRIVER_SO") {
        return PathBuf::from(p);
    }
    // Default: prefer the profile the tests themselves were built in.
    let dbg = manifest_dir().join("target/debug/libdriver.so");
    let rel = manifest_dir().join("target/release/libdriver.so");
    if dbg.exists() {
        return dbg;
    }
    assert!(
        rel.exists(),
        "Rust cdylib not found at {} or {} — build it with `cargo build --offline`",
        dbg.display(),
        rel.display()
    );
    rel
}

pub fn c_impl() -> &'static Impl {
    static S: OnceLock<Impl> = OnceLock::new();
    S.get_or_init(|| Impl::load("C", c_so_path()))
}

pub fn rust_impl() -> &'static Impl {
    static S: OnceLock<Impl> = OnceLock::new();
    S.get_or_init(|| Impl::load("Rust", rust_so_path()))
}

/// Result of one call, in a form that can be compared byte-for-byte.
#[derive(Debug, PartialEq, Eq)]
pub struct Outcome {
    /// Returned pointer expressed as a byte offset from the pointer passed in.
    pub offset: isize,
    /// The bytes of the returned C string (read through the returned pointer).
    pub string: Vec<u8>,
}

/// Call one implementation. `buf` is the whole allocation; the string handed to
/// the library starts at `buf[start]` (lets us test unaligned starts and data
/// living before/after the string).
pub fn run_one(im: &Impl, buf: &mut [u8], start: usize) -> Outcome {
    assert!(start < buf.len());
    let base = unsafe { buf.as_mut_ptr().add(start) };
    let ret = unsafe { (im.tool_basename)(base as *mut c_char) } as *const u8;
    assert!(!ret.is_null(), "{}: tool_basename returned NULL", im.name);
    let offset = ret as isize - base as isize;
    // Read the returned string the way any C consumer would: through the
    // returned pointer, up to the NUL.
    let mut string = Vec::new();
    unsafe {
        let mut i = 0usize;
        loop {
            let b = *ret.add(i);
            if b == 0 {
                break;
            }
            string.push(b);
            i += 1;
            assert!(i < (1 << 26), "{}: returned string is not terminated", im.name);
        }
    }
    Outcome { offset, string }
}

/// Call the library, then feed the *returned* interior pointer straight back in
/// (a real consumer pattern, and it exercises pointers that are not the start of
/// an allocation). Returns both offsets relative to the original base.
pub fn run_twice(im: &Impl, buf: &mut [u8], start: usize) -> (isize, isize) {
    let base = unsafe { buf.as_mut_ptr().add(start) } as *mut c_char;
    unsafe {
        let r1 = (im.tool_basename)(base);
        let r2 = (im.tool_basename)(r1);
        (r1 as isize - base as isize, r2 as isize - base as isize)
    }
}

/// Differential version of [`run_twice`].
#[track_caller]
pub fn diff_twice(buf: &[u8], start: usize) {
    let mut b_c = buf.to_vec();
    let mut b_r = buf.to_vec();
    let c = run_twice(c_impl(), &mut b_c, start);
    let r = run_twice(rust_impl(), &mut b_r, start);
    assert_eq!(c, r, "chained-call offsets differ for {}", Esc(&buf[start..]));
    assert_eq!(c.0, c.1, "C is not idempotent on its own result for {}", Esc(&buf[start..]));
    assert_eq!(b_c, b_r, "buffers differ after chained calls");
}

pub struct Esc<'a>(pub &'a [u8]);

impl fmt::Display for Esc<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "\"")?;
        for &b in self.0.iter().take(160) {
            match b {
                b'\\' => write!(f, "\\\\")?,
                0x20..=0x7e => write!(f, "{}", b as char)?,
                0 => write!(f, "\\0")?,
                _ => write!(f, "\\x{b:02x}")?,
            }
        }
        if self.0.len() > 160 {
            write!(f, "...({} bytes)", self.0.len())?;
        }
        write!(f, "\"")
    }
}

/// Differential check: run C and Rust on identical private copies of `buf`
/// (string starting at `start`) and require identical results.
#[track_caller]
pub fn diff_at(buf: &[u8], start: usize) {
    assert!(buf[start..].contains(&0), "test input must be NUL-terminated");

    let mut b_c = buf.to_vec();
    let mut b_r = buf.to_vec();

    let out_c = run_one(c_impl(), &mut b_c, start);
    let out_r = run_one(rust_impl(), &mut b_r, start);

    let shown = &buf[start..];
    assert_eq!(
        out_c.offset, out_r.offset,
        "returned-pointer offset differs (C={}, Rust={}) for input {} (start={start})",
        out_c.offset, out_r.offset, Esc(shown)
    );
    assert_eq!(
        Esc(&out_c.string).to_string(),
        Esc(&out_r.string).to_string(),
        "returned string differs for input {} (start={start})",
        Esc(shown)
    );
    assert_eq!(out_c, out_r);

    // Both must leave the caller's buffer untouched (the C function is pure).
    assert_eq!(b_c, buf, "C mutated the input buffer for {}", Esc(shown));
    assert_eq!(b_r, buf, "Rust mutated the input buffer for {}", Esc(shown));

    // Invariant: the result is a pointer inside the string [base, base+strlen].
    let strlen = buf[start..].iter().position(|&b| b == 0).unwrap() as isize;
    assert!(
        out_c.offset >= 0 && out_c.offset <= strlen,
        "C returned out-of-bounds offset {} (strlen={strlen}) for {}",
        out_c.offset,
        Esc(shown)
    );

    // Cross-check against an independent model of the C source.
    let expected = model(&buf[start..start + strlen as usize]);
    assert_eq!(
        out_c.offset, expected,
        "model disagrees with the C implementation for {} — fix the model",
        Esc(shown)
    );
}

/// Convenience: `s` is the string content (no NUL); a NUL is appended.
#[track_caller]
pub fn diff(s: &[u8]) {
    let mut v = s.to_vec();
    v.push(0);
    diff_at(&v, 0);
}

/// Independent re-derivation of `c_src/src/lib.c` used as a third opinion:
/// offset of the byte after the last `/` or `\`, else 0.
pub fn model(s: &[u8]) -> isize {
    let last_slash = s.iter().rposition(|&b| b == b'/');
    let last_bslash = s.iter().rposition(|&b| b == b'\\');
    match (last_slash, last_bslash) {
        (Some(a), Some(b)) => (if a > b { a } else { b }) as isize + 1,
        (Some(a), None) => a as isize + 1,
        (None, Some(b)) => b as isize + 1,
        (None, None) => 0,
    }
}

/// Deterministic SplitMix64 PRNG (no external crate, reproducible).
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed)
    }
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    /// Uniform in `0..n`.
    pub fn below(&mut self, n: usize) -> usize {
        assert!(n > 0);
        (self.next_u64() % n as u64) as usize
    }
    /// Uniform in `1..=255` (never NUL).
    pub fn nonzero_byte(&mut self) -> u8 {
        1 + (self.next_u64() % 255) as u8
    }
    pub fn pick(&mut self, set: &[u8]) -> u8 {
        set[self.below(set.len())]
    }
}

pub const SEED: u64 = 0x5EED_1234_ABCD_0001;
